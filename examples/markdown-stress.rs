//! Times each frame of a Markdown stream without a terminal. Frames keep the
//! real cadence, so the highlight thread runs at the speed it would in an app;
//! each frame receives a chunk of the file, and the report gives percentiles
//! over the frames that rendered.
//!
//! `markdown-stress FILE [--chunk BYTES] [--bytes LIMIT] [--paced MS] [--fps N]`
//! Build with `--release`; add the `markdown-highlight` feature for code colors.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use eink_rs::markdown::{
    MarkdownPiece, MarkdownSender, MarkdownStreamHandle, MarkdownStreamOptions, MarkdownTail,
    use_markdown_stream,
};
use ink_rs::testing::{TestTerminal, mount};
use ink_rs::{Element, FlexDirection, Options, Scope, Static, Styled, View, component};

/// Gives up when nothing renders for this long while the stream is open.
const STALL: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct Probe {
    pacing: Option<Duration>,
    renders: Rc<Cell<usize>>,
    link: Rc<RefCell<Option<(MarkdownSender, MarkdownStreamHandle)>>>,
}

impl PartialEq for Probe {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.link, &other.link)
    }
}

fn reply(cx: &mut Scope, probe: &Probe) -> Element {
    probe.renders.set(probe.renders.get() + 1);
    let history = cx.use_memo((), || Rc::new(RefCell::new(Vec::<MarkdownPiece>::new())));
    let output = history.clone();
    let options = MarkdownStreamOptions {
        pacing: probe.pacing,
    };
    let stream = use_markdown_stream(cx, options, move |piece| output.borrow_mut().push(piece));
    let app = cx.use_app();
    let window = cx.use_window_size();
    let (link, handle) = (probe.link.clone(), stream.clone());
    cx.use_layout_effect((), move || {
        *link.borrow_mut() = Some((handle.sender(app), handle));
    });
    let count = history.borrow().len();
    View::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Static::from_fn(count, move |index| history.borrow()[index].clone().into())
                .width(window.columns),
        )
        .child(MarkdownTail::new(&stream))
        .into()
}

fn percentile(sorted: &[Duration], share: f64) -> f64 {
    let index = ((sorted.len() as f64 * share).ceil() as usize).clamp(1, sorted.len()) - 1;
    sorted[index].as_secs_f64() * 1000.0
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("expected a Markdown file")?;
    let (mut chunk, mut limit, mut pacing, mut fps) = (64, None, None, 120);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or("expected a value after an option")?;
        match arg.as_str() {
            "--chunk" => chunk = value.parse::<usize>()?.max(1),
            "--bytes" => limit = Some(value.parse::<usize>()?),
            "--paced" => pacing = Some(Duration::from_millis(value.parse()?)),
            "--fps" => fps = value.parse::<u32>()?.max(1),
            _ => return Err(format!("unknown option: {arg}").into()),
        }
    }
    let mut source = std::fs::read_to_string(&path)?;
    if let Some(limit) = limit
        && limit < source.len()
    {
        let mut end = limit;
        while !source.is_char_boundary(end) {
            end += 1;
        }
        source.truncate(end);
    }

    let probe = Probe {
        pacing,
        renders: Rc::default(),
        link: Rc::default(),
    };
    let terminal = TestTerminal::new(100, 40);
    let options = Options {
        max_fps: fps,
        ..Options::default()
    };
    let mut app = mount(component(reply, probe.clone()), options, &terminal);
    let frame = Duration::from_secs(1) / fps;
    app.advance(Duration::ZERO);
    let (sender, handle) = probe.link.borrow_mut().take().ok_or("stream not mounted")?;

    let mut times = Vec::new();
    let mut slowest = (Duration::ZERO, 0, 0);
    let mut from = 0;
    let mut last_render = Instant::now();
    while !handle.progress().done {
        let pieces = handle.progress().pieces;
        if from < source.len() {
            let mut end = (from + chunk).min(source.len());
            while !source.is_char_boundary(end) {
                end += 1;
            }
            sender.push(&source[from..end]);
            from = end;
            if from == source.len() {
                sender.clone().finish();
            }
        }
        let renders = probe.renders.get();
        let started = Instant::now();
        app.advance(frame);
        let spent = started.elapsed();
        if probe.renders.get() > renders {
            if spent > slowest.0 {
                slowest = (spent, times.len(), handle.progress().pieces - pieces);
            }
            times.push(spent);
            last_render = Instant::now();
        } else if last_render.elapsed() > STALL {
            return Err(format!("no render for {STALL:?} at byte {from}").into());
        }
        thread::sleep(frame.saturating_sub(spent));
    }

    let total: Duration = times.iter().sum();
    times.sort();
    let progress = handle.progress();
    println!(
        "{path} · {:.0} KB · chunk {chunk} B · {fps} fps · paced {}",
        source.len() as f64 / 1024.0,
        pacing.map_or("off".into(), |pacing| format!("{pacing:?}")),
    );
    println!(
        "{} frames · {} pieces · total {:.1} ms · {:.3} ms/KB",
        times.len(),
        progress.pieces,
        total.as_secs_f64() * 1000.0,
        total.as_secs_f64() * 1000.0 / (source.len() as f64 / 1024.0),
    );
    println!(
        "p50 {:.2} · p90 {:.2} · p99 {:.2} · max {:.2} ms (frame {}, {} pieces) · {} over {:.2} ms",
        percentile(&times, 0.5),
        percentile(&times, 0.9),
        percentile(&times, 0.99),
        slowest.0.as_secs_f64() * 1000.0,
        slowest.1,
        slowest.2,
        times.iter().filter(|time| **time > frame).count(),
        frame.as_secs_f64() * 1000.0,
    );
    Ok(())
}
