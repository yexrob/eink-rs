//! Streams a reply the way a chat client shows a model's answer: a producer
//! thread sends chunks of uneven size at uneven times, finished pieces go to
//! the scrollback through `Static`, and only the tail repaints.
//!
//! Keys: `s` settles the tail, `p` paces the reveal, `r` replays, `q` quits.
//! `--fps N` sets the paint rate, 120 by default. `--file PATH` streams a file
//! in place of the canned answer; `--rate BYTES` sends that many bytes a second
//! in place of the uneven schedule.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use eink_rs::markdown::{
    MarkdownPiece, MarkdownSender, MarkdownStreamOptions, MarkdownTail, use_markdown_stream,
};
use eink_rs::ui::Spinner;
use ink_rs::{
    AnimationOptions, Color, Element, FlexDirection, Options, Scope, Static, Styled, Text, View,
    component,
};

const PROMPT: &str = "用 Rust 写一个 LRU 缓存，并说明设计取舍";
const ANSWER: &str = include_str!("markdown-answer.md");
const CATCH_UP: Duration = Duration::from_millis(300);

#[derive(Clone, PartialEq)]
struct Config {
    fps: u32,
    prompt: String,
    source: Arc<str>,
    rate: Option<usize>,
}

enum Item {
    Prompt,
    Piece(MarkdownPiece),
    Summary(String),
}

/// Sends the source from a thread: `rate` bytes a second, or 2–41 bytes
/// every 12–71 ms with an occasional pause of 250–600 ms.
fn produce(sender: MarkdownSender, config: Config) {
    thread::spawn(move || {
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut random = |range| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % range
        };
        let started = Instant::now();
        let mut from = 0;
        while from < config.source.len() {
            let mut end = if let Some(rate) = config.rate {
                thread::sleep(Duration::from_millis(8));
                (started.elapsed().as_secs_f64() * rate as f64) as usize
            } else {
                let pause = if random(25) == 0 {
                    250 + random(350)
                } else {
                    0
                };
                thread::sleep(Duration::from_millis(12 + random(60) + pause));
                from + 2 + random(40) as usize
            }
            .min(config.source.len());
            while !config.source.is_char_boundary(end) {
                end += 1;
            }
            if end > from {
                sender.push(&config.source[from..end]);
                from = end;
            }
        }
        sender.finish();
    });
}

fn item(item: &Item, prompt: &str) -> Element {
    match item {
        Item::Prompt => View::new()
            .margin_top(1)
            .child(
                Text::new(format!("› {prompt}"))
                    .bold(true)
                    .color(Color::Magenta),
            )
            .into(),
        Item::Piece(piece) => piece.clone().into(),
        Item::Summary(summary) => View::new()
            .margin_top(1)
            .child(Text::new(summary).dim_color(true))
            .into(),
    }
}

fn demo(cx: &mut Scope, config: &Config) -> Element {
    let history = cx.use_memo((), || Rc::new(RefCell::new(vec![Item::Prompt])));
    let output = history.clone();
    let stream = use_markdown_stream(cx, MarkdownStreamOptions::default(), move |piece| {
        output.borrow_mut().push(Item::Piece(piece))
    });
    let settled = cx.use_state(|| true);
    let paced = cx.use_state(|| false);
    let running = cx.use_state(|| true);
    let started = cx.use_state(Instant::now);
    let replay = cx.use_state(|| 0_u64);
    let app = cx.use_app();
    let window = cx.use_window_size();
    let progress = stream.progress();

    let (handle, wake, producer) = (stream.clone(), app.clone(), config.clone());
    cx.use_effect(replay.get(), move || produce(handle.sender(wake), producer));
    let log = history.clone();
    cx.use_layout_effect(progress.done, move || {
        if progress.done {
            log.borrow_mut().push(Item::Summary(format!(
                "✓ {} 字节 · {} 段 · {:.1}s",
                progress.received,
                progress.pieces,
                started.get().elapsed().as_secs_f64()
            )));
            running.set(false);
        }
    });
    let (controls, log) = (stream.clone(), history.clone());
    cx.use_input(true, move |input, key| match input {
        "s" => settled.update(|value| *value = !*value),
        "p" => {
            let enabled = !paced.get();
            paced.set(enabled);
            controls.set_pacing(enabled.then_some(CATCH_UP));
        }
        "r" => {
            controls.reset();
            log.borrow_mut().push(Item::Prompt);
            started.set(Instant::now());
            running.set(true);
            replay.update(|value| *value += 1);
        }
        "q" => app.exit(),
        _ if key.escape => app.exit(),
        _ => {}
    });

    // Ticks in the last second. The clock ticks at most once a paint window,
    // and a slow frame drops ticks; commits can outnumber paints.
    let clock = cx.use_animation(AnimationOptions {
        interval: Duration::from_secs(1) / config.fps,
        active: running.get(),
    });
    let ticks = cx.use_memo((), || Rc::new(RefCell::new(VecDeque::<Instant>::new())));
    let record = ticks.clone();
    cx.use_layout_effect(clock.time, move || {
        record.borrow_mut().push_back(Instant::now())
    });
    let fps = {
        let mut ticks = ticks.borrow_mut();
        while ticks
            .front()
            .is_some_and(|at| at.elapsed() > Duration::from_secs(1))
        {
            ticks.pop_front();
        }
        ticks.len()
    };
    let switch = |on: bool| if on { "开" } else { "关" };
    let status = format!(
        "收到 {:.1}/{:.1} KB · 显示 {:.1} KB · 定稿 {} 段 · {} fps · s 补齐:{} · p 匀速:{} · r 重放 · q 退出",
        progress.received as f64 / 1024.0,
        config.source.len() as f64 / 1024.0,
        progress.shown as f64 / 1024.0,
        progress.pieces,
        fps,
        switch(settled.get()),
        switch(paced.get()),
    );
    let mut bar = View::new().margin_top(1).gap(1);
    if running.get() {
        bar = bar.child(View::new().flex_shrink(0.0).child(Spinner::default()));
    }
    let count = history.borrow().len();
    let prompt = config.prompt.clone();
    View::new()
        .flex_direction(FlexDirection::Column)
        // Static sizes to its content; rules and tables need the full width.
        .child(
            Static::from_fn(count, move |index| item(&history.borrow()[index], &prompt))
                .width(window.columns),
        )
        .child(MarkdownTail::new(&stream).settle(settled.get()))
        .child(bar.child(Text::new(status).dim_color(true)))
        .into()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config {
        fps: 120,
        prompt: PROMPT.into(),
        source: Arc::from(ANSWER),
        rate: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or("expected a value after an option")?;
        match arg.as_str() {
            "--fps" => config.fps = value.parse::<u32>()?.max(1),
            "--file" => {
                config.source = Arc::from(std::fs::read_to_string(&value)?);
                config.prompt = value;
            }
            "--rate" => {
                let bytes = value.parse::<usize>()?;
                if bytes == 0 {
                    return Err("--rate must be positive".into());
                }
                config.rate = Some(bytes);
            }
            _ => return Err(format!("unknown option: {arg}").into()),
        }
    }
    let fps = config.fps;
    ink_rs::render(
        component(demo, config),
        Options {
            max_fps: fps,
            ..Options::default()
        },
    )
    .wait_until_exit()?;
    Ok(())
}
