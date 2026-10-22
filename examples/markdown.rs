//! Streams a canned answer the way a chat client shows a model's reply:
//! chunks of uneven size arrive at uneven times, finished blocks go to the
//! scrollback through `Static`, and only the tail repaints.
//!
//! Keys: `s` settles the tail, `p` paces the reveal, `r` replays, `q` quits.
//! `--fps N` sets the paint rate, 120 by default.

use std::rc::Rc;
use std::time::Duration;

use eink_rs::markdown::{Markdown, MarkdownStream};
use eink_rs::ui::Spinner;
use ink_rs::{
    AnimationOptions, Color, Element, FlexDirection, Options, Scope, Static, Styled, Text, View,
    component,
};

const PROMPT: &str = "用 Rust 写一个 LRU 缓存，并说明设计取舍";
const ANSWER: &str = include_str!("markdown-answer.md");
const CATCH_UP: Duration = Duration::from_millis(300);

#[derive(Clone)]
enum Item {
    Prompt,
    Block { source: Rc<str>, first: bool },
    Summary(String),
}

#[derive(Clone)]
struct Demo {
    history: Vec<Item>,
    stream: MarkdownStream,
    /// Chunks of the schedule pushed so far.
    received: usize,
    started: Duration,
    last: Duration,
    restart: bool,
    running: bool,
    settle: bool,
    paced: bool,
    /// Finished blocks moved into the history.
    committed: usize,
    fps: f64,
}

impl Demo {
    fn new() -> Self {
        Self {
            history: vec![Item::Prompt],
            stream: MarkdownStream::new(),
            received: 0,
            started: Duration::ZERO,
            last: Duration::ZERO,
            restart: true,
            running: true,
            settle: true,
            paced: false,
            committed: 0,
            fps: 0.0,
        }
    }

    fn replay(&mut self) {
        self.history.push(Item::Prompt);
        self.stream = MarkdownStream::new();
        self.stream.set_pacing(self.pacing());
        self.received = 0;
        self.committed = 0;
        self.restart = true;
        self.running = true;
    }

    fn pacing(&self) -> Option<Duration> {
        self.paced.then_some(CATCH_UP)
    }

    fn poll(&mut self, schedule: &[(Duration, usize)], now: Duration) {
        if self.restart {
            self.restart = false;
            self.started = now;
            self.last = now;
        }
        let elapsed = now.saturating_sub(self.started);
        let from = self.stream.source().len();
        let mut end = from;
        while let Some(&(at, stop)) = schedule.get(self.received)
            && at <= elapsed
        {
            end = stop;
            self.received += 1;
        }
        if end > from {
            self.stream.push(&ANSWER[from..end]);
            if self.received == schedule.len() {
                self.stream.finish();
            }
        }
        let delta = now.saturating_sub(self.last);
        self.stream.tick(delta);
        if !delta.is_zero() {
            self.fps = self.fps * 0.9 + 0.1 / delta.as_secs_f64();
        }
        self.last = now;
        for block in &self.stream.blocks()[self.committed..] {
            self.history.push(Item::Block {
                source: block.clone(),
                first: self.committed == 0,
            });
            self.committed += 1;
        }
        if self.running && self.stream.is_done() {
            self.running = false;
            self.history.push(Item::Summary(format!(
                "✓ {} 字节 · {} 块 · {:.1}s",
                ANSWER.len(),
                self.committed,
                elapsed.as_secs_f64()
            )));
        }
    }
}

/// When each chunk of the answer arrives, and the byte it ends at:
/// 2–41 bytes every 12–71 ms, with an occasional pause of 250–600 ms.
fn schedule() -> Vec<(Duration, usize)> {
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut random = move |range: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % range
    };
    let mut at = Duration::ZERO;
    let mut end = 0;
    let mut chunks = Vec::new();
    while end < ANSWER.len() {
        at += Duration::from_millis(12 + random(60));
        if random(25) == 0 {
            at += Duration::from_millis(250 + random(350));
        }
        end = (end + 2 + random(40) as usize).min(ANSWER.len());
        while !ANSWER.is_char_boundary(end) {
            end += 1;
        }
        chunks.push((at, end));
    }
    chunks
}

fn item(item: &Item, _: usize) -> Element {
    match item {
        Item::Prompt => View::new()
            .margin_top(1)
            .child(
                Text::new(format!("› {PROMPT}"))
                    .bold(true)
                    .color(Color::Magenta),
            )
            .into(),
        Item::Block { source, first } => View::new()
            .flex_direction(FlexDirection::Column)
            .margin_top(i16::from(!first))
            .child(Markdown::new(source.clone()))
            .into(),
        Item::Summary(summary) => View::new()
            .margin_top(1)
            .child(Text::new(summary).dim_color(true))
            .into(),
    }
}

fn demo(cx: &mut Scope, _: &()) -> Element {
    let schedule = cx.use_memo((), || Rc::new(schedule()));
    let state = cx.use_state(Demo::new);
    let app = cx.use_app();
    let window = cx.use_window_size();
    let current = state.get();
    // Ticks land on the interval grid and those inside the paint window are
    // skipped, so an interval equal to the window delivers about half its
    // ticks. A 1 ms interval leaves the rate to `max_fps`.
    let clock = cx.use_animation(AnimationOptions {
        interval: Duration::from_millis(1),
        active: current.running,
    });
    cx.use_layout_effect(clock.time, move || {
        state.update(|demo| demo.poll(&schedule, clock.time));
    });
    cx.use_input(true, move |input, key| match input {
        "s" => state.update(|demo| demo.settle = !demo.settle),
        "p" => state.update(|demo| {
            demo.paced = !demo.paced;
            let pacing = demo.pacing();
            demo.stream.set_pacing(pacing);
        }),
        "r" => state.update(Demo::replay),
        "q" => app.exit(),
        _ if key.escape => app.exit(),
        _ => {}
    });

    let tail = current.stream.tail();
    let mut live = View::new().flex_direction(FlexDirection::Column);
    if !tail.is_empty() {
        live = live.child(
            View::new()
                .flex_direction(FlexDirection::Column)
                .margin_top(i16::from(current.committed > 0))
                .child(Markdown::new(tail).streaming(current.settle)),
        );
    }
    let switch = |on: bool| if on { "开" } else { "关" };
    let status = format!(
        "收到 {:.1}/{:.1} KB · 显示 {:.1} KB · 定稿 {} 块 · {:.0} fps · s 补齐:{} · p 匀速:{} · r 重放 · q 退出",
        current.stream.source().len() as f64 / 1024.0,
        ANSWER.len() as f64 / 1024.0,
        current.stream.shown().len() as f64 / 1024.0,
        current.committed,
        current.fps,
        switch(current.settle),
        switch(current.paced),
    );
    let mut bar = View::new().margin_top(1).gap(1);
    if current.running {
        bar = bar.child(View::new().flex_shrink(0.0).child(Spinner::default()));
    }
    View::new()
        .flex_direction(FlexDirection::Column)
        // Static sizes to its content; rules and tables need the full width.
        .child(Static::new(current.history, item).width(window.columns))
        .child(live)
        .child(bar.child(Text::new(status).dim_color(true)))
        .into()
}

fn main() -> Result<(), ink_rs::Error> {
    let mut args = std::env::args().skip_while(|arg| arg != "--fps").skip(1);
    let fps = args.next().and_then(|fps| fps.parse().ok()).unwrap_or(120);
    let options = Options {
        max_fps: fps,
        ..Options::default()
    };
    ink_rs::render(component(demo, ()), options)
        .wait_until_exit()
        .map(|_| ())
}
