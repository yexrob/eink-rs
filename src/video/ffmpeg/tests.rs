use super::*;
use crate::video::VideoDecoders;
use std::sync::Arc;

fn request() -> FrameRequest {
    FrameRequest {
        start: Duration::from_millis(500),
        width: 32,
        height: 18,
        fps: 10,
    }
}

/// The input of a local source under the decoder's base, with the platform's separator.
fn local_input(source: &str) -> String {
    format!("file:{}", PathBuf::from("videos").join(source).display())
}

fn args(command: Command) -> Vec<String> {
    command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn commands_constrain_protocols_and_keep_sources_as_inputs() {
    let decoder = FfmpegDecoder::new("videos").with_programs("my-ffmpeg", "my-ffprobe");
    assert_eq!(
        decoder.frames_command("-x", &request()).get_program(),
        "my-ffmpeg"
    );
    let local = args(decoder.frames_command("-x", &request()));
    assert_eq!(
        local,
        [
            "-nostdin",
            "-v",
            "error",
            "-protocol_whitelist",
            "file",
            "-ss",
            "0.500000000",
            "-i",
            local_input("-x").as_str(),
            "-map",
            "0:v:0",
            "-an",
            "-sn",
            "-dn",
            "-vf",
            "fps=10,scale=32:18",
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "pipe:1",
        ]
    );
    for source in ["concat:one|two", "subfile:video", "list.m3u8"] {
        let command = args(decoder.frames_command(source, &request()));
        assert_eq!(command[8], local_input(source));
    }
    for source in ["http://example.org/v.mp4", "https://example.org/v.mp4"] {
        let command = args(decoder.frames_command(source, &request()));
        assert_eq!(
            &command[3..7],
            [
                "-protocol_whitelist",
                "http,https,tcp,tls,crypto,httpproxy",
                "-rw_timeout",
                "10000000"
            ]
        );
        assert_eq!(&command[7..11], ["-ss", "0.500000000", "-i", source]);
    }
    let local_probe = decoder.probe_command("-x");
    assert_eq!(local_probe.get_program(), "my-ffprobe");
    assert_eq!(
        args(local_probe),
        [
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,avg_frame_rate:stream_tags=rotate:stream_side_data=rotation:format=duration",
            "-of",
            "default=noprint_wrappers=1",
            "-protocol_whitelist",
            "file",
            "-i",
            local_input("-x").as_str(),
        ]
    );
    let probe = args(decoder.probe_command("HTTPS://example.org/v.mp4"));
    assert_eq!(
        &probe[8..],
        [
            "-protocol_whitelist",
            "http,https,tcp,tls,crypto,httpproxy",
            "-rw_timeout",
            "10000000",
            "-i",
            "https://example.org/v.mp4"
        ]
    );
}

#[test]
fn probe_parses_dimensions_rotation_and_unknown_duration() {
    assert_eq!(
        parse_info("width=64\nheight=36\navg_frame_rate=30000/1001\nduration=1.500000\n").unwrap(),
        VideoInfo {
            width: 64,
            height: 36,
            duration: Some(Duration::from_millis(1500)),
            fps: Some(30),
        }
    );
    assert_eq!(
        parse_info("width=64\nheight=36\navg_frame_rate=0/0")
            .unwrap()
            .fps,
        None
    );
    for rotation in [
        "rotation=90",
        "rotation=-90",
        "rotation=270",
        "TAG:rotate=90",
    ] {
        let info = parse_info(&format!("width=64\nheight=36\n{rotation}\nduration=N/A\n")).unwrap();
        assert_eq!((info.width, info.height), (36, 64));
        assert_eq!(info.duration, None);
    }
    assert_eq!(parse_info("width=64\nheight=36\n").unwrap().duration, None);
    for output in ["", "duration=1.0", "width=0\nheight=0", "width=64"] {
        assert!(parse_info(output).is_err());
    }
    assert!(parse_info("width=64\nheight=36\nduration=NaN").is_err());
}

#[test]
fn slots_compare_identity_and_reject_disabled_sources() {
    let defaults = VideoDecoders::default();
    assert!(defaults == VideoDecoders::default());
    assert!(defaults.decoder("video.mp4").is_ok());
    assert!(defaults.decoder("https://example.org/video").is_ok());
    let mut local = defaults.clone();
    local.network = None;
    assert!(local != defaults);
    assert!(local.decoder("HTTP://example.org/video").is_err());
    assert!(local.decoder("video.mp4").is_ok());
    local.local = Some(Arc::new(FfmpegDecoder::default()));
    assert!(!Arc::ptr_eq(
        &local.decoder("x").unwrap(),
        &defaults.decoder("x").unwrap()
    ));
}

#[test]
fn missing_programs_name_the_program_and_stderr_keeps_its_tail() {
    let decoder = FfmpegDecoder::default().with_programs(
        "eink-missing-ffmpeg-program",
        "eink-missing-ffprobe-program",
    );
    let error = decoder.info("x").unwrap_err();
    assert!(error.to_string().contains("eink-missing-ffprobe-program"));
    let error = decoder.frames("x", &request()).err().unwrap();
    assert!(error.to_string().contains("eink-missing-ffmpeg-program"));
    let bytes: Vec<_> = (0..40_000).map(|index| (index % 251) as u8).collect();
    assert_eq!(
        read_tail(bytes.as_slice()).unwrap(),
        bytes[bytes.len() - 16 * 1024..]
    );
}

#[test]
fn real_ffmpeg_probes_and_seeks_a_clip() {
    for program in ["ffmpeg", "ffprobe"] {
        match Command::new(program)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return,
            result => assert!(result.unwrap().success()),
        }
    }
    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let directory = Directory(std::env::temp_dir().join(format!(
        "eink-video-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    std::fs::create_dir(&directory.0).unwrap();
    let output = Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x36:rate=10:duration=1",
            "-c:v",
            "ffv1",
        ])
        .arg(directory.0.join("clip.mkv"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decoder = FfmpegDecoder::new(&directory.0);
    let info = decoder.info("clip.mkv").unwrap();
    assert_eq!((info.width, info.height), (64, 36));
    assert_eq!(info.duration, Some(Duration::from_secs(1)));
    let frames = decoder
        .frames("clip.mkv", &request())
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(frames.len(), 5);
    assert!(frames.iter().all(|frame| frame.len() == 32 * 18 * 3));
    let error = decoder
        .frames("missing.mkv", &request())
        .unwrap()
        .next()
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("ffmpeg"));
    assert!(error.to_string().contains("missing.mkv"));
}
