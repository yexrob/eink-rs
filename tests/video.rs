#![cfg(feature = "video")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eink_rs::image::{ImageTheme, LoadError};
use eink_rs::video::{
    FrameRequest, Video, VideoDecoder, VideoDecoders, VideoFrames, VideoInfo, VideoOptions,
    VideoPlayer, VideoStatus, use_video_player,
};
use ink_rs::testing::{TestApp, TestTerminal, mount, render_with};
use ink_rs::{Color, ColorLevel, Element, Options, RenderOptions, Scope, Text, component};

const KITTY: &str = "\x1b_Gi=31;OK\x1b\\\x1b[6;20;10t\x1b[?1;2c";
const INTERVAL: Duration = Duration::from_millis(100);

struct Decoder {
    fail: bool,
    requests: Mutex<Vec<FrameRequest>>,
    duration: Duration,
}

impl VideoDecoder for Decoder {
    fn info(&self, _: &str) -> Result<VideoInfo, LoadError> {
        if self.fail {
            return Err(LoadError::new("missing video"));
        }
        Ok(VideoInfo {
            width: 10,
            height: 20,
            duration: Some(self.duration),
            fps: Some(10),
        })
    }

    fn frames(&self, _: &str, request: &FrameRequest) -> Result<VideoFrames, LoadError> {
        self.requests.lock().unwrap().push(request.clone());
        let count = (self.duration.saturating_sub(request.start).as_secs_f64()
            * f64::from(request.fps))
        .ceil() as u32;
        let bytes = (request.width * request.height * 3) as usize;
        let start = (request.start.as_secs_f64() * 10.0) as u32;
        Ok(Box::new(
            (0..count).map(move |index| Ok(vec![(start + index) as u8; bytes])),
        ))
    }
}

#[derive(Clone)]
struct Harness {
    options: VideoOptions,
    decoders: VideoDecoders,
    handle: Rc<RefCell<Option<VideoPlayer>>>,
}

impl PartialEq for Harness {
    fn eq(&self, other: &Self) -> bool {
        self.options == other.options
            && self.decoders == other.decoders
            && Rc::ptr_eq(&self.handle, &other.handle)
    }
}

fn root(cx: &mut Scope, props: &Harness) -> Element {
    cx.use_input(true, |_, _| {});
    cx.provide(props.decoders.clone());
    cx.provide(ImageTheme {
        color: Some(Color::Yellow),
        dim: false,
        italic: false,
    });
    component(view, props.clone())
}

fn view(cx: &mut Scope, props: &Harness) -> Element {
    let player = use_video_player(cx, props.options.clone());
    *props.handle.borrow_mut() = Some(player.clone());
    Video {
        player,
        alt: "description".into(),
    }
    .into()
}

fn mounted(looping: bool, fail: bool) -> (TestApp, TestTerminal, Harness, Arc<Decoder>) {
    let decoder = Arc::new(Decoder {
        fail,
        requests: Mutex::new(Vec::new()),
        duration: Duration::from_millis(950),
    });
    let props = Harness {
        options: VideoOptions {
            src: "test".into(),
            looping,
            ..VideoOptions::default()
        },
        decoders: VideoDecoders {
            local: Some(decoder.clone()),
            network: None,
        },
        handle: Rc::new(RefCell::new(None)),
    };
    let terminal = TestTerminal::new(40, 60);
    let app = mount(
        component(root, props.clone()),
        Options {
            color_level: Some(ColorLevel::TrueColor),
            max_fps: 120,
            ..Options::default()
        },
        &terminal,
    );
    (app, terminal, props, decoder)
}

fn handle(props: &Harness) -> VideoPlayer {
    props.handle.borrow().as_ref().unwrap().clone()
}

fn wait(app: &mut TestApp, condition: impl Fn(&TestApp) -> bool) {
    for _ in 0..3000 {
        if condition(app) {
            return;
        }
        app.advance(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!(
        "video worker did not reach expected state: {}",
        app.live_frame()
    );
}

fn commands(terminal: &TestTerminal, action: &str) -> Vec<String> {
    terminal
        .writes()
        .iter()
        .filter_map(|write| {
            let normalized = write.replace("\x1b\x1b", "\x1b");
            let start = normalized.find(&format!("\x1b_Ga={action},"))?;
            let end = normalized[start..].find("\x1b\\")? + start + 2;
            Some(normalized[start..end].to_owned())
        })
        .collect()
}

fn image_id(command: &str) -> u32 {
    command
        .split_once(';')
        .unwrap()
        .0
        .split(',')
        .find_map(|key| key.strip_prefix("i="))
        .unwrap()
        .parse()
        .unwrap()
}

fn frame(id: u32) -> String {
    render_with(
        Text::new("\u{10eeee}\u{305}\u{305}").color(Color::Rgb(
            (id >> 16) as u8,
            (id >> 8) as u8,
            id as u8,
        )),
        RenderOptions {
            columns: 40,
            color_level: ColorLevel::TrueColor,
        },
    )
    .last_frame()
    .to_owned()
}

fn red(command: &str) -> u8 {
    let payload = command
        .split_once(';')
        .unwrap()
        .1
        .strip_suffix("\x1b\\")
        .unwrap();
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut png = Vec::new();
    for chunk in payload.as_bytes().chunks(4) {
        let mut value = 0_u32;
        for &byte in chunk {
            value = (value << 6) | alphabet.iter().position(|&c| c == byte).unwrap_or(0) as u32;
        }
        png.push((value >> 16) as u8);
        if chunk[2] != b'=' {
            png.push((value >> 8) as u8);
        }
        if chunk[3] != b'=' {
            png.push(value as u8);
        }
    }
    image::load_from_memory(&png)
        .unwrap()
        .to_rgb8()
        .get_pixel(0, 0)
        .0[0]
}

#[test]
fn playback_pause_seek_and_unmount_keep_uploads_off_the_drawn_id() {
    let (mut app, terminal, props, decoder) = mounted(false, false);
    assert_eq!(handle(&props).status(), VideoStatus::Loading);
    app.input(KITTY);
    wait(&mut app, |_| !commands(&terminal, "t").is_empty());
    let first = image_id(&commands(&terminal, "t")[0]);
    assert_eq!(app.live_frame(), frame(first));
    assert_eq!(handle(&props).position(), Duration::ZERO);
    assert_eq!(red(&commands(&terminal, "t")[0]), 0);
    wait(&mut app, |_| commands(&terminal, "t").len() >= 2);
    let second = image_id(&commands(&terminal, "t")[1]);
    assert_ne!(first, second);
    assert_eq!(app.live_frame(), frame(first));
    let before = terminal.writes().len();
    wait(&mut app, |_| commands(&terminal, "t").len() >= 3);
    assert_eq!(app.live_frame(), frame(second));
    assert_eq!(image_id(&commands(&terminal, "t")[2]), first);
    let writes = terminal.writes();
    let writes = &writes[before..];
    assert!(
        writes
            .iter()
            .position(|w| w.contains('\u{10eeee}'))
            .unwrap()
            < writes.iter().position(|w| w.contains("a=t,t=d")).unwrap()
    );
    assert_eq!(
        commands(&terminal, "t").len(),
        commands(&terminal, "p").len()
    );

    let player = handle(&props);
    player.pause();
    app.advance(INTERVAL);
    let position = player.position();
    let uploads = commands(&terminal, "t");
    app.advance(Duration::from_millis(500));
    assert_eq!(player.status(), VideoStatus::Paused);
    assert_eq!(player.position(), position);
    assert_eq!(commands(&terminal, "t"), uploads);
    let drawn = if app.live_frame() == frame(first) {
        first
    } else {
        second
    };
    player.seek(Duration::from_millis(700));
    wait(&mut app, |_| commands(&terminal, "t").len() > uploads.len());
    let sought = commands(&terminal, "t").last().unwrap().clone();
    assert_eq!(red(&sought), 7);
    assert_ne!(image_id(&sought), drawn);
    assert_eq!(app.live_frame(), frame(drawn));
    wait(&mut app, |app| app.live_frame() == frame(image_id(&sought)));
    assert_eq!(player.position(), Duration::from_millis(700));
    assert_eq!(player.status(), VideoStatus::Paused);
    assert_eq!(
        decoder.requests.lock().unwrap().last().unwrap().start,
        Duration::from_millis(700)
    );
    app.advance(Duration::from_millis(500));
    assert_eq!(commands(&terminal, "t").len(), uploads.len() + 1);
    app.rerender(Text::new(""));
    assert_eq!(
        commands(&terminal, "d"),
        [format!("\x1b_Ga=d,d=I,i={drawn},q=2\x1b\\")]
    );
}

#[test]
fn end_loop_restart_and_failed_info() {
    let (mut app, terminal, mut props, decoder) = mounted(false, false);
    app.input(KITTY);
    wait(&mut app, |_| handle(&props).status() == VideoStatus::Ended);
    let player = handle(&props);
    assert_eq!(player.position(), Duration::from_millis(950));
    assert_eq!(player.duration(), Some(Duration::from_millis(950)));
    app.advance(INTERVAL);
    let end_frame = app.live_frame().to_owned();
    let uploads = commands(&terminal, "t");
    app.advance(Duration::from_secs(1));
    assert_eq!(commands(&terminal, "t"), uploads);
    assert_eq!(app.live_frame(), end_frame);
    player.seek(Duration::from_secs(5));
    assert_eq!(player.status(), VideoStatus::Paused);
    assert_eq!(player.position(), Duration::from_millis(950));
    app.advance(INTERVAL);
    assert_eq!(player.status(), VideoStatus::Paused);
    player.play();
    wait(&mut app, |_| player.status() == VideoStatus::Ended);
    player.play();
    assert_eq!(player.position(), Duration::ZERO);
    assert_eq!(player.status(), VideoStatus::Playing);
    props.options.looping = true;
    app.rerender(component(root, props.clone()));
    wait(&mut app, |_| {
        player.position() >= Duration::from_millis(500)
    });
    let starts = decoder.requests.lock().unwrap().len();
    wait(&mut app, |_| {
        decoder.requests.lock().unwrap().len() > starts
    });
    assert!(player.position() < Duration::from_millis(300));
    assert_eq!(player.status(), VideoStatus::Playing);
    assert_eq!(
        decoder.requests.lock().unwrap().last().unwrap().start,
        Duration::ZERO
    );

    let (mut app, terminal, props, _) = mounted(false, true);
    app.input(KITTY);
    wait(&mut app, |_| {
        matches!(handle(&props).status(), VideoStatus::Failed(_))
    });
    assert_eq!(
        handle(&props).status(),
        VideoStatus::Failed(LoadError::new("missing video"))
    );
    assert_eq!(
        app.live_frame(),
        render_with(
            Text::new("[description]").color(Color::Yellow),
            RenderOptions {
                columns: 40,
                color_level: ColorLevel::TrueColor
            }
        )
        .last_frame()
    );
    assert!(commands(&terminal, "t").is_empty());
}
