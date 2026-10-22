#![cfg(feature = "markdown-image")]

use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use eink_rs::image::{ImageLoader, ImageLoaders, LoadError};
use eink_rs::markdown::{
    Markdown, MarkdownImages, MarkdownPiece, MarkdownStream, MarkdownStreamOptions, MarkdownTail,
    use_markdown_stream,
};
use ink_rs::testing::{TestApp, TestTerminal, mount, render};
use ink_rs::{ColorLevel, Element, FlexDirection, Options, Scope, Static, Styled, View, component};

const KITTY: &str = "\x1b_Gi=31;OK\x1b\\\x1b[6;20;10t\x1b[?1;2c";
const PIXEL: char = '\u{10eeee}';

struct Loader {
    calls: AtomicUsize,
    results: Mutex<mpsc::Receiver<Vec<u8>>>,
}

impl ImageLoader for Loader {
    fn load(&self, _: &str) -> Result<Vec<u8>, LoadError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.results
            .lock()
            .unwrap()
            .recv()
            .map_err(|error| LoadError::new(error.to_string()))
    }
}

fn loader() -> (Arc<Loader>, mpsc::Sender<Vec<u8>>, ImageLoaders) {
    let (send, results) = mpsc::channel();
    let loader = Arc::new(Loader {
        calls: AtomicUsize::new(0),
        results: Mutex::new(results),
    });
    let loaders = ImageLoaders {
        local: Some(loader.clone()),
        network: None,
    };
    (loader, send, loaders)
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(width, height, image::Rgba([40, 90, 180, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

fn kitty(loaders: ImageLoaders) -> MarkdownImages {
    MarkdownImages::Kitty {
        loaders,
        cell_pixels: Some((10, 20)),
        color_level: ColorLevel::TrueColor,
    }
}

#[derive(Clone, PartialEq)]
struct Document {
    source: String,
    loaders: ImageLoaders,
    columns: u16,
}

fn provider(cx: &mut Scope, props: &Document) -> Element {
    cx.use_input(true, |_, _| {});
    cx.provide(props.loaders.clone());
    View::new()
        .width(props.columns)
        .child(Markdown::new(props.source.clone()))
        .into()
}

fn mounted(element: impl Into<Element>, columns: u16) -> (TestApp, TestTerminal) {
    let terminal = TestTerminal::new(columns, 60);
    let app = mount(
        element,
        Options {
            color_level: Some(ColorLevel::TrueColor),
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
        assert!(Instant::now() < deadline, "image did not settle");
        app.advance(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn plain(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.next() {
                Some('[') => {
                    for ch in chars.by_ref() {
                        if ('@'..='~').contains(&ch) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(ch) = chars.next() {
                        if ch == '\x07' || (ch == '\x1b' && chars.next() == Some('\\')) {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn document(source: &str, size: (u32, u32), columns: u16) -> (TestApp, TestTerminal) {
    let (_, reply, loaders) = loader();
    reply.send(png(size.0, size.1)).unwrap();
    let (mut app, terminal) = mounted(
        component(
            provider,
            Document {
                source: source.into(),
                loaders,
                columns,
            },
        ),
        40,
    );
    app.input(KITTY);
    wait(&mut app, |app| app.live_frame().contains(PIXEL));
    (app, terminal)
}

#[test]
fn block_image_fits_cells_without_enlarging() {
    let (app, _) = document("![alt](image)", (20, 40), 40);
    let frame = plain(&app.live_frame());
    assert_eq!(frame.lines().count(), 2);
    assert!(frame.lines().all(|line| line.matches(PIXEL).count() == 2));
}

#[test]
fn small_inline_image_keeps_text_and_link_on_one_row() {
    let (app, _) = document(
        "before [![alt](image)](https://example.invalid) after",
        (30, 30),
        40,
    );
    let frame = app.live_frame();
    assert!(frame.contains("\x1b]8;;https://example.invalid\x1b\\"));
    let frame = plain(&frame);
    assert_eq!(frame.lines().count(), 1);
    assert!(frame.starts_with("before "));
    assert!(frame.ends_with(" after"));
    assert_eq!(frame.matches(PIXEL).count(), 2);
}

#[test]
fn tall_inline_image_splits_the_paragraph() {
    let (app, _) = document("before ![alt](image) after", (20, 60), 40);
    let frame = plain(&app.live_frame());
    let lines: Vec<_> = frame.lines().collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0].trim(), "before");
    assert_eq!(lines[4].trim(), "after");
    assert!(
        lines[1..4]
            .iter()
            .all(|line| line.matches(PIXEL).count() == 2)
    );
}

#[test]
fn list_image_subtracts_indentation_from_measured_columns() {
    let (app, _) = document("- ![alt](image)", (400, 100), 12);
    let frame = plain(&app.live_frame());
    assert_eq!(frame.lines().count(), 2, "{frame:?}");
    assert!(frame.lines().all(|line| line.matches(PIXEL).count() == 10));
    assert!(frame.starts_with("• "));
}

#[test]
fn table_image_remains_alt() {
    let (_, reply, loaders) = loader();
    reply.send(png(20, 40)).unwrap();
    let (mut app, terminal) = mounted(
        component(
            provider,
            Document {
                source: "| image |\n| --- |\n| ![alt](image) |".into(),
                loaders,
                columns: 40,
            },
        ),
        40,
    );
    app.input(KITTY);
    app.advance(Duration::from_secs(6));
    assert!(plain(&app.live_frame()).contains("[alt]"));
    assert!(!terminal.writes().concat().contains("a=t,t=d"));
}

#[test]
fn stream_holds_following_text_and_matches_document() {
    for (source, size) in [
        ("![alt](image)\n\nfollowing text\n", (20, 40)),
        ("before ![alt](image) after\n\nfollowing text\n", (30, 30)),
        ("before ![alt](image) after\n\nfollowing text\n", (20, 60)),
        ("- ![alt](image)\n\nfollowing text\n", (400, 100)),
        ("> ![alt](image)\n\nfollowing text\n", (400, 100)),
    ] {
        let (_, reply, loaders) = loader();
        let mut stream = MarkdownStream::new();
        stream.set_images(kitty(loaders));
        stream.set_columns(40);
        stream.push(source);
        stream.finish();
        assert!(stream.take_pieces().is_empty());
        reply.send(png(size.0, size.1)).unwrap();
        while stream.wait_for_images() {}
        let pieces = stream.take_pieces();
        assert_eq!(pieces.len(), 2);
        assert!(stream.is_done());
        let stacked = render(
            View::new()
                .flex_direction(FlexDirection::Column)
                .children(pieces),
        );
        let (whole, _) = document(source, size, 40);
        assert_eq!(plain(stacked.last_frame()), plain(&whole.live_frame()));
    }
}

#[test]
fn stream_deadline_releases_a_blocked_loader() {
    let (_, _reply, loaders) = loader();
    let mut stream = MarkdownStream::new();
    stream.set_images(kitty(loaders));
    stream.push("![alt](image)\n\nfollowing\n");
    stream.finish();
    stream.tick(Duration::from_millis(4999));
    assert!(stream.take_pieces().is_empty());
    stream.tick(Duration::from_millis(1));
    let pieces = stream.take_pieces();
    assert_eq!(pieces.len(), 2);
    let frame = render(
        View::new()
            .flex_direction(FlexDirection::Column)
            .children(pieces),
    );
    assert_eq!(plain(frame.last_frame()), "[alt]\n\nfollowing");
    assert!(!stream.wait_for_images());
}

#[test]
fn probing_then_alt_releases_without_loading() {
    let (loader, _reply, loaders) = loader();
    let mut stream = MarkdownStream::new();
    stream.set_images(kitty(loaders));
    stream.set_images(MarkdownImages::Probing);
    stream.push("![alt](image)");
    stream.finish();
    assert!(stream.take_pieces().is_empty());
    stream.set_images(MarkdownImages::Alt);
    let pieces = stream.take_pieces();
    assert_eq!(pieces.len(), 1);
    assert_eq!(plain(render(pieces[0].clone()).last_frame()), "[alt]");
    assert_eq!(loader.calls.load(Ordering::SeqCst), 0);
}

fn stream_provider(cx: &mut Scope, loaders: &ImageLoaders) -> Element {
    cx.provide(loaders.clone());
    component(stream_view, ())
}

fn stream_view(cx: &mut Scope, _: &()) -> Element {
    cx.use_input(true, |_, _| {});
    let history = cx.use_memo((), || Rc::new(RefCell::new(Vec::<MarkdownPiece>::new())));
    let output = history.clone();
    let stream = use_markdown_stream(cx, MarkdownStreamOptions::default(), move |piece| {
        output.borrow_mut().push(piece)
    });
    let sender = stream.sender(cx.use_app());
    cx.use_effect((), move || {
        sender.push("![alt](image)\n\nfollowing\n");
        sender.finish();
    });
    let count = history.borrow().len();
    View::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Static::from_fn(count, move |index| history.borrow()[index].clone().into()).width(40),
        )
        .child(MarkdownTail::new(&stream))
        .into()
}

#[test]
fn static_piece_sends_image_data_once_after_placeholders() {
    let (loader, reply, loaders) = loader();
    reply.send(png(20, 40)).unwrap();
    let (mut app, terminal) = mounted(component(stream_provider, loaders.clone()), 40);
    app.advance(Duration::from_millis(30));
    assert_eq!(loader.calls.load(Ordering::SeqCst), 0);
    app.input(KITTY);
    wait(&mut app, |_| terminal.writes().concat().contains("a=t,t=d"));
    app.rerender(component(stream_provider, loaders));
    app.advance(Duration::from_millis(30));
    let writes = terminal.writes();
    assert_eq!(writes.concat().matches("a=t,t=d").count(), 1);
    assert_eq!(loader.calls.load(Ordering::SeqCst), 1);
    let placeholders = writes
        .iter()
        .position(|write| write.contains(PIXEL))
        .unwrap();
    let data = writes
        .iter()
        .position(|write| write.contains("a=t,t=d"))
        .unwrap();
    assert!(placeholders < data);
}
