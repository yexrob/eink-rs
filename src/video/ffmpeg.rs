use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::image::{LoadError, is_network};

use super::{FrameRequest, VideoDecoder, VideoFrames, VideoInfo};

/// Decodes videos with the ffmpeg and ffprobe programs.
#[derive(Clone, Debug)]
pub struct FfmpegDecoder {
    base: PathBuf,
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

impl FfmpegDecoder {
    /// Creates a decoder with a base directory and programs found on `PATH`.
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self {
            base: base.into(),
            ffmpeg: "ffmpeg".into(),
            ffprobe: "ffprobe".into(),
        }
    }

    /// Sets the paths of the ffmpeg and ffprobe programs.
    pub fn with_programs(
        mut self,
        ffmpeg: impl Into<PathBuf>,
        ffprobe: impl Into<PathBuf>,
    ) -> Self {
        self.ffmpeg = ffmpeg.into();
        self.ffprobe = ffprobe.into();
        self
    }

    fn input(&self, command: &mut Command, src: &str) -> OsString {
        if is_network(src) {
            command.args([
                "-protocol_whitelist",
                "http,https,tcp,tls,crypto",
                "-rw_timeout",
                "10000000",
            ]);
            // The slot selector accepts mixed case, but ffmpeg's protocols use lowercase.
            let (scheme, rest) = src.split_once("://").expect("HTTP source");
            format!("{}://{rest}", scheme.to_ascii_lowercase()).into()
        } else {
            command.args(["-protocol_whitelist", "file"]);
            let mut input = OsString::from("file:");
            input.push(self.base.join(src));
            input
        }
    }

    fn probe_command(&self, src: &str) -> Command {
        let mut command = Command::new(&self.ffprobe);
        command.args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height:stream_tags=rotate:stream_side_data=rotation:format=duration",
            "-of",
            "default=noprint_wrappers=1",
        ]);
        let input = self.input(&mut command, src);
        command.arg("-i").arg(input);
        command
    }

    fn frames_command(&self, src: &str, request: &FrameRequest) -> Command {
        let mut command = Command::new(&self.ffmpeg);
        command.args(["-nostdin", "-v", "error"]);
        let input = self.input(&mut command, src);
        command
            .arg("-ss")
            .arg(format!(
                "{}.{:09}",
                request.start.as_secs(),
                request.start.subsec_nanos()
            ))
            .arg("-i")
            .arg(input)
            .args(["-map", "0:v:0", "-an", "-sn", "-dn", "-vf"])
            .arg(format!(
                "fps={},scale={}:{}",
                request.fps, request.width, request.height
            ))
            .args(["-pix_fmt", "rgb24", "-f", "rawvideo", "pipe:1"]);
        command
    }
}

impl Default for FfmpegDecoder {
    fn default() -> Self {
        Self::new(".")
    }
}

impl VideoDecoder for FfmpegDecoder {
    fn info(&self, src: &str) -> Result<VideoInfo, LoadError> {
        let mut process = Process::spawn(self.probe_command(src))?;
        let mut output = String::new();
        process.stdout.read_to_string(&mut output)?;
        process.finish()?;
        parse_info(&output)
    }

    fn frames(&self, src: &str, request: &FrameRequest) -> Result<VideoFrames, LoadError> {
        let bytes = request.frame_bytes()?;
        Ok(Box::new(Frames {
            process: Process::spawn(self.frames_command(src, request))?,
            bytes,
            ended: false,
        }))
    }
}

fn parse_info(output: &str) -> Result<VideoInfo, LoadError> {
    let mut width = None;
    let mut height = None;
    let mut rotation = 0_i32;
    let mut duration = None;
    for (key, value) in output.lines().filter_map(|line| line.split_once('=')) {
        match key {
            "width" => width = value.parse::<u32>().ok().filter(|value| *value > 0),
            "height" => height = value.parse::<u32>().ok().filter(|value| *value > 0),
            "rotation" | "TAG:rotate" => {
                rotation = value
                    .parse()
                    .map_err(|_| LoadError::new("invalid video rotation"))?;
            }
            "duration" if value != "N/A" => {
                duration = Some(
                    value
                        .parse::<f64>()
                        .ok()
                        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
                        .ok_or_else(|| LoadError::new("invalid video duration"))?,
                );
            }
            _ => {}
        }
    }
    let (Some(mut width), Some(mut height)) = (width, height) else {
        return Err(LoadError::new("no video stream with display dimensions"));
    };
    if matches!(rotation.rem_euclid(360), 90 | 270) {
        std::mem::swap(&mut width, &mut height);
    }
    Ok(VideoInfo {
        width,
        height,
        duration,
    })
}

struct Process {
    child: Child,
    stdout: ChildStdout,
    stderr: Option<JoinHandle<io::Result<Vec<u8>>>>,
    program: OsString,
    reaped: bool,
}

impl Process {
    fn spawn(mut command: Command) -> Result<Self, LoadError> {
        let program = command.get_program().to_owned();
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                LoadError::new(format!(
                    "cannot start {}: {error}",
                    program.to_string_lossy()
                ))
            })?;
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let mut process = Self {
            child,
            stdout,
            stderr: None,
            program,
            reaped: false,
        };
        process.stderr = Some(
            thread::Builder::new()
                .name("eink-video-stderr".into())
                .spawn(move || read_tail(stderr))?,
        );
        Ok(process)
    }

    fn finish(&mut self) -> Result<(), LoadError> {
        let status = self.child.wait()?;
        self.reaped = true;
        let stderr = self
            .stderr
            .take()
            .expect("stderr reader")
            .join()
            .map_err(|_| LoadError::new("video stderr reader panicked"))??;
        check_status(&self.program, status, &stderr)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(stderr) = self.stderr.take() {
            let _ = stderr.join();
        }
    }
}

fn read_tail(mut reader: impl Read) -> io::Result<Vec<u8>> {
    const LIMIT: usize = 16 * 1024;
    let mut tail = VecDeque::with_capacity(LIMIT);
    let mut buffer = [0; 4096];
    loop {
        let length = match reader.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if length == 0 {
            return Ok(tail.into());
        }
        let excess = (tail.len() + length).saturating_sub(LIMIT);
        tail.drain(..excess);
        tail.extend(&buffer[..length]);
    }
}

fn check_status(program: &OsString, status: ExitStatus, stderr: &[u8]) -> Result<(), LoadError> {
    if status.success() {
        Ok(())
    } else {
        Err(LoadError::new(format!(
            "{} exited with {status}: {}",
            program.to_string_lossy(),
            String::from_utf8_lossy(stderr).trim()
        )))
    }
}

struct Frames {
    process: Process,
    bytes: usize,
    ended: bool,
}

impl Iterator for Frames {
    type Item = Result<Vec<u8>, LoadError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let mut bytes = vec![0; self.bytes];
        match self.process.stdout.read_exact(&mut bytes) {
            Ok(()) => Some(Ok(bytes)),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                self.ended = true;
                self.process.finish().err().map(Err)
            }
            Err(error) => {
                self.ended = true;
                Some(Err(error.into()))
            }
        }
    }
}

#[cfg(test)]
mod tests;
