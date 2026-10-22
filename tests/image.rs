#![cfg(feature = "image")]

use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use eink_rs::image::{Image, ImageLoader, ImageLoaders, ImageTheme, LoadError};
use ink_rs::testing::{TestApp, TestTerminal, mount, render_with};
use ink_rs::{
    Color, ColorLevel, Element, FlexDirection, Options, RenderOptions, Scope, Styled, Text, View,
    component,
};

const KITTY: &str = "\x1b_Gi=31;OK\x1b\\\x1b[6;20;10t\x1b[?1;2c";

struct TestLoader {
    calls: AtomicUsize,
    returned: AtomicUsize,
    results: Mutex<mpsc::Receiver<Result<Vec<u8>, LoadError>>>,
}

impl ImageLoader for TestLoader {
    fn load(&self, _src: &str) -> Result<Vec<u8>, LoadError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let result = self.results.lock().unwrap().recv().unwrap();
        self.returned.fetch_add(1, Ordering::SeqCst);
        result
    }
}

type Reply = mpsc::Sender<Result<Vec<u8>, LoadError>>;

fn loader() -> (Arc<TestLoader>, Reply) {
    let (send, results) = mpsc::channel();
    (
        Arc::new(TestLoader {
            calls: AtomicUsize::new(0),
            returned: AtomicUsize::new(0),
            results: Mutex::new(results),
        }),
        send,
    )
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([40, 90, 180, 255]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[derive(Clone, PartialEq)]
struct Harness {
    image: Image,
    loaders: ImageLoaders,
}

fn harness(cx: &mut Scope, props: &Harness) -> Element {
    cx.use_input(true, |_, _| {});
    cx.provide(props.loaders.clone());
    cx.provide(ImageTheme {
        color: Some(Color::Yellow),
        dim: false,
        italic: false,
    });
    props.image.clone().into()
}

fn props(loader: Arc<TestLoader>) -> Harness {
    Harness {
        image: Image {
            src: "test-image".into(),
            alt: "description".into(),
            ..Image::default()
        },
        loaders: ImageLoaders {
            local: Some(loader),
            network: None,
        },
    }
}

fn mounted(props: Harness, level: ColorLevel) -> (TestApp, TestTerminal) {
    let terminal = TestTerminal::new(40, 60);
    let app = mount(
        component(harness, props),
        Options {
            color_level: Some(level),
            max_fps: 120,
            ..Options::default()
        },
        &terminal,
    );
    (app, terminal)
}

fn wait(app: &mut TestApp, condition: impl Fn(&TestApp) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition(app) {
        assert!(Instant::now() < deadline, "image worker did not finish");
        app.advance(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn commands(terminal: &TestTerminal, action: &str) -> Vec<String> {
    terminal
        .writes()
        .iter()
        .filter_map(|write| {
            // Each write is one command, with optional tmux passthrough.
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

fn assert_alt(app: &TestApp, level: ColorLevel) {
    let expected = render_with(
        Text::new("[description]").color(Color::Yellow),
        RenderOptions {
            columns: 40,
            color_level: level,
        },
    );
    assert_eq!(app.live_frame(), expected.last_frame());
}

#[test]
fn transmits_once_and_replaces_placement_on_resize() {
    let (loader, reply) = loader();
    reply.send(Ok(png(400, 200))).unwrap();
    let root = props(loader.clone());
    let (mut app, terminal) = mounted(root.clone(), ColorLevel::TrueColor);
    assert_alt(&app, ColorLevel::TrueColor);
    assert_eq!(loader.calls.load(Ordering::SeqCst), 0);
    app.input(KITTY);
    wait(&mut app, |_| !commands(&terminal, "p").is_empty());
    let transmissions = commands(&terminal, "t");
    assert_eq!(transmissions.len(), 1);
    let id = image_id(&transmissions[0]);
    assert!(
        transmissions[0].starts_with(&format!("\x1b_Ga=t,t=d,f=100,i={id},q=2,m=0;iVBORw0KGgo"))
    );
    assert_eq!(
        commands(&terminal, "p"),
        [format!("\x1b_Ga=p,U=1,i={id},p=1,c=40,r=10,q=2\x1b\\")],
    );
    let writes = terminal.writes();
    let placeholders = writes
        .iter()
        .position(|write| write.contains('\u{10eeee}'))
        .unwrap();
    let transmission = writes
        .iter()
        .position(|write| write.contains("a=t,t=d"))
        .unwrap();
    assert!(placeholders < transmission);

    app.rerender(component(harness, root));
    app.advance(Duration::from_millis(30));
    assert_eq!(commands(&terminal, "t"), transmissions);
    assert_eq!(commands(&terminal, "p").len(), 1);
    app.resize(20, 60);
    app.advance(Duration::from_millis(30));
    assert_eq!(commands(&terminal, "t"), transmissions);
    assert_eq!(
        commands(&terminal, "p")[1],
        format!("\x1b_Ga=p,U=1,i={id},p=1,c=20,r=5,q=2\x1b\\"),
    );
    assert_eq!(loader.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn placeholders_use_row_column_marks_and_the_color_levels_id() {
    for level in [ColorLevel::TrueColor, ColorLevel::Ansi256] {
        let (loader, reply) = loader();
        reply.send(Ok(png(20, 40))).unwrap();
        let (mut app, terminal) = mounted(props(loader), level);
        app.input(KITTY);
        wait(&mut app, |_| !commands(&terminal, "p").is_empty());
        let id = image_id(&commands(&terminal, "t")[0]);
        let color = if level == ColorLevel::TrueColor {
            assert!((256..=0xff_ffff).contains(&id));
            Color::Rgb((id >> 16) as u8, (id >> 8) as u8, id as u8)
        } else {
            assert!((1..256).contains(&id));
            Color::Ansi256(id as u8)
        };
        let expected = render_with(
            View::new().flex_direction(FlexDirection::Column).children([
                Text::new("\u{10eeee}\u{305}\u{305}\u{10eeee}\u{305}\u{30d}").color(color),
                Text::new("\u{10eeee}\u{30d}\u{305}\u{10eeee}\u{30d}\u{30d}").color(color),
            ]),
            RenderOptions {
                columns: 40,
                color_level: level,
            },
        );
        assert_eq!(app.live_frame(), expected.last_frame());
    }
}

#[test]
fn alt_covers_loading_failure_and_local_and_network_deadlines() {
    for source in ["test-image", "https://example.invalid/image"] {
        let (loader, reply) = loader();
        let mut root = props(loader.clone());
        root.image.src = source.into();
        root.loaders.network = Some(loader.clone());
        let (mut app, terminal) = mounted(root, ColorLevel::TrueColor);
        app.input(KITTY);
        wait(&mut app, |_| loader.calls.load(Ordering::SeqCst) == 1);
        assert_alt(&app, ColorLevel::TrueColor);
        app.advance(Duration::from_secs(if source.starts_with("https") {
            10
        } else {
            5
        }));
        assert_alt(&app, ColorLevel::TrueColor);
        reply.send(Ok(png(20, 40))).unwrap();
        wait(&mut app, |_| loader.returned.load(Ordering::SeqCst) == 1);
        app.advance(Duration::from_millis(100));
        assert_alt(&app, ColorLevel::TrueColor);
        assert!(commands(&terminal, "t").is_empty());
    }
    for result in [Err(LoadError::new("missing")), Ok(b"not an image".to_vec())] {
        let (loader, reply) = loader();
        reply.send(result).unwrap();
        let (mut app, terminal) = mounted(props(loader.clone()), ColorLevel::TrueColor);
        app.input(KITTY);
        wait(&mut app, |_| loader.returned.load(Ordering::SeqCst) == 1);
        app.advance(Duration::from_secs(6));
        assert_alt(&app, ColorLevel::TrueColor);
        assert!(commands(&terminal, "t").is_empty());
    }
}

#[test]
fn unsupported_terminals_and_disabled_slots_do_not_load() {
    for (level, reply) in [
        (ColorLevel::Basic, KITTY),
        (ColorLevel::TrueColor, "\x1b[?1;2c"),
    ] {
        let (loader, _send) = loader();
        let (mut app, terminal) = mounted(props(loader.clone()), level);
        app.input(reply);
        app.advance(Duration::from_secs(11));
        assert_alt(&app, level);
        assert_eq!(loader.calls.load(Ordering::SeqCst), 0);
        assert!(commands(&terminal, "t").is_empty());
    }
    for source in [
        "http://example.invalid/image",
        "https://example.invalid/image",
    ] {
        let (loader, _send) = loader();
        let mut root = props(loader.clone());
        root.image.src = source.into();
        let (mut app, terminal) = mounted(root, ColorLevel::TrueColor);
        app.input(KITTY);
        app.advance(Duration::from_secs(11));
        assert_alt(&app, ColorLevel::TrueColor);
        assert_eq!(loader.calls.load(Ordering::SeqCst), 0);
        assert!(commands(&terminal, "t").is_empty());
    }
}

#[test]
fn provided_loaders_replace_both_slots() {
    for source in ["provided-source", "http://example.invalid/image"] {
        let (first, old_reply) = loader();
        let (second, reply) = loader();
        let mut root = props(first.clone());
        root.image.src = source.into();
        root.loaders.network = Some(first.clone());
        let (mut app, terminal) = mounted(root.clone(), ColorLevel::TrueColor);
        app.input(KITTY);
        wait(&mut app, |_| first.calls.load(Ordering::SeqCst) == 1);
        root.loaders = ImageLoaders {
            local: Some(second.clone()),
            network: Some(second.clone()),
        };
        reply.send(Ok(png(20, 40))).unwrap();
        app.rerender(component(harness, root));
        wait(&mut app, |_| !commands(&terminal, "p").is_empty());
        let transmissions = commands(&terminal, "t");
        old_reply.send(Ok(png(400, 200))).unwrap();
        wait(&mut app, |_| first.returned.load(Ordering::SeqCst) == 1);
        app.advance(Duration::from_millis(100));
        assert_eq!(commands(&terminal, "t"), transmissions);
        assert_eq!(second.calls.load(Ordering::SeqCst), 1);
        assert_eq!(app.live_frame().lines().count(), 2);
    }
}
