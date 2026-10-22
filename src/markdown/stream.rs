use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
use unicode_segmentation::UnicodeSegmentation;

use super::piece::{Kind, MarkdownPiece};
use super::{options, render};

/// Configuration read when a stream is created or reset.
#[derive(Clone, Debug, Default)]
pub struct MarkdownStreamOptions {
    /// Time in which paced ticks aim to clear the current backlog. `None` disables pacing.
    pub pacing: Option<Duration>,
}

/// A growing Markdown document with final pieces and a live tail.
/// Link definitions after a piece do not affect that piece.
/// Fenced code is processed by line; lists retain only the current item in the parser.
/// Tables and other open blocks are parsed again when a line completes.
#[cfg_attr(not(feature = "markdown-highlight"), derive(Default))]
pub struct MarkdownStream {
    source: String,
    shown: usize,
    scanned: usize,
    stable: usize,
    backlog: usize,
    finished: bool,
    pacing: Option<Duration>,
    gap: u16,
    count: usize,
    list: Option<List>,
    fence: Option<Fence>,
    pending: VecDeque<Pending>,
    links: render::Links,
    #[cfg(feature = "markdown-highlight")]
    results: (
        std::sync::mpsc::Sender<super::highlight::ResultLine>,
        std::sync::mpsc::Receiver<super::highlight::ResultLine>,
    ),
    #[cfg(feature = "markdown-highlight")]
    code_theme: String,
}

struct Pending {
    start: usize,
    piece: MarkdownPiece,
    #[cfg(feature = "markdown-highlight")]
    waiting: Option<u64>,
}

#[derive(Clone, Copy)]
struct List {
    marker: u8,
    next: Option<u64>,
}

struct Fence {
    marker: u8,
    length: usize,
    indent: usize,
    #[cfg(feature = "markdown-highlight")]
    language: String,
    #[cfg(feature = "markdown-highlight")]
    id: u64,
    #[cfg(feature = "markdown-highlight")]
    unknown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

struct Block {
    start: usize,
    list: Option<List>,
    fence: Option<String>,
}

#[cfg(feature = "markdown-highlight")]
impl Default for MarkdownStream {
    fn default() -> Self {
        Self {
            source: String::new(),
            shown: 0,
            scanned: 0,
            stable: 0,
            backlog: 0,
            finished: false,
            pacing: None,
            gap: 0,
            count: 0,
            list: None,
            fence: None,
            pending: VecDeque::new(),
            links: render::Links::new(),
            #[cfg(feature = "markdown-highlight")]
            results: std::sync::mpsc::channel(),
            #[cfg(feature = "markdown-highlight")]
            code_theme: "ansi".into(),
        }
    }
}

impl MarkdownStream {
    /// Creates an unpaced stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a stream with paced reveal.
    pub fn paced(catch_up: Duration) -> Self {
        let mut stream = Self::default();
        stream.pacing = Some(catch_up);
        stream
    }

    pub(crate) fn pacing(&self) -> Option<Duration> {
        self.pacing
    }

    /// Changes pacing. Disabling it reveals the backlog.
    pub fn set_pacing(&mut self, catch_up: Option<Duration>) {
        self.set_pacing_with(catch_up, None);
    }

    pub(crate) fn set_pacing_with(
        &mut self,
        catch_up: Option<Duration>,
        wake: Option<super::hook::Wake>,
    ) {
        self.pacing = catch_up;
        if catch_up.is_none() {
            self.reveal(self.source.len(), wake);
        }
    }

    /// Appends text. Pushes after finish are ignored.
    pub fn push(&mut self, delta: &str) {
        self.push_with(delta, None);
    }

    pub(crate) fn push_with(&mut self, delta: &str, wake: Option<super::hook::Wake>) {
        if self.finished {
            return;
        }
        self.backlog += delta.graphemes(true).count();
        self.source.push_str(delta);
        if self.pacing.is_none() {
            self.reveal(self.source.len(), wake);
        }
    }

    /// Marks the producer finished. Paced text and highlighting may remain pending.
    pub fn finish(&mut self) {
        self.finish_with(None);
    }

    pub(crate) fn finish_with(&mut self, wake: Option<super::hook::Wake>) {
        self.finished = true;
        self.reveal(self.shown, wake);
    }

    /// Reveals a share of the backlog, with a minimum of one grapheme.
    pub fn tick(&mut self, elapsed: Duration) {
        self.tick_with(elapsed, None);
    }

    pub(crate) fn tick_with(&mut self, elapsed: Duration, wake: Option<super::hook::Wake>) {
        let Some(catch_up) = self.pacing else { return };
        if !self.is_revealing() {
            return;
        }
        let share = elapsed.as_secs_f64() / catch_up.as_secs_f64().max(f64::EPSILON);
        let count = ((self.backlog as f64 * share).ceil() as usize).max(1);
        let mut bytes = 0;
        let mut revealed = 0;
        for grapheme in self.source[self.shown..].graphemes(true).take(count) {
            bytes += grapheme.len();
            revealed += 1;
        }
        self.backlog = self.backlog.saturating_sub(revealed);
        self.reveal(self.shown + bytes, wake);
    }

    /// Takes final pieces in document order. Each piece is returned once.
    pub fn take_pieces(&mut self) -> Vec<MarkdownPiece> {
        self.poll_highlights();
        let mut pieces = Vec::new();
        while self.pending.front().is_some_and(|pending| {
            #[cfg(feature = "markdown-highlight")]
            {
                pending.waiting.is_none()
            }
            #[cfg(not(feature = "markdown-highlight"))]
            {
                let _ = pending;
                true
            }
        }) {
            pieces.push(self.pending.pop_front().expect("front piece").piece);
            self.count += 1;
        }
        pieces
    }

    /// Takes available background results without blocking.
    pub fn poll_highlights(&mut self) {
        #[cfg(feature = "markdown-highlight")]
        while let Ok(result) = self.results.1.try_recv() {
            self.accept(result);
        }
    }

    /// Waits for one pending highlight result, without a polling or sleep loop.
    /// Returns false if no highlighting is pending.
    pub fn wait_for_highlights(&mut self) -> bool {
        #[cfg(feature = "markdown-highlight")]
        if self.pending.iter().any(|line| line.waiting.is_some()) {
            if let Ok(result) = self.results.1.recv() {
                self.accept(result);
            }
            self.poll_highlights();
            return true;
        }
        false
    }

    #[cfg(feature = "markdown-highlight")]
    fn accept(&mut self, result: super::highlight::ResultLine) {
        if let Some(pending) = self
            .pending
            .get_mut((result.id as usize).saturating_sub(self.count))
        {
            let Kind::Line { source, .. } = &pending.piece.inner.kind else {
                return;
            };
            pending.piece = MarkdownPiece::new(
                0,
                Kind::Line {
                    source: source.clone(),
                    spans: result.spans,
                },
            );
            pending.waiting = None;
        }
    }

    /// Sets the theme used for future background highlight requests.
    #[cfg(feature = "markdown-highlight")]
    pub fn set_code_theme(&mut self, theme: impl Into<String>) {
        self.code_theme = theme.into();
    }

    /// Uncommitted source, including lines waiting for highlighting.
    pub fn tail(&self) -> &str {
        let start = self
            .pending
            .front()
            .map_or(self.stable, |pending| pending.start);
        &self.source[start..self.shown]
    }
    /// All received source.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// The revealed source prefix.
    pub fn shown(&self) -> &str {
        &self.source[..self.shown]
    }
    /// Number of pieces taken by the caller.
    pub fn pieces_emitted(&self) -> usize {
        self.count
    }
    /// Whether paced text is waiting.
    pub fn is_revealing(&self) -> bool {
        self.shown < self.source.len()
    }
    /// Whether finish was received and all output has been taken.
    pub fn is_done(&self) -> bool {
        self.finished
            && self.shown == self.source.len()
            && self.stable == self.shown
            && self.pending.is_empty()
    }

    fn enqueue(&mut self, gap: u16, kind: Kind) {
        self.pending.push_back(Pending {
            start: self.stable,
            piece: MarkdownPiece::new(gap, kind),
            #[cfg(feature = "markdown-highlight")]
            waiting: None,
        });
    }

    fn reveal(&mut self, shown: usize, wake: Option<super::hook::Wake>) {
        let mut scan = self.shown;
        self.shown = shown;
        if shown == self.source.len() {
            self.backlog = 0;
        }
        while let Some(at) = self.source[scan..shown].find('\n') {
            let end = scan + at + 1;
            scan = end;
            self.scanned = end;
            self.complete_line(end, wake.clone());
        }
        if self.finished && shown == self.source.len() {
            if self.scanned < shown {
                self.scanned = shown;
                self.complete_line(shown, wake.clone());
            }
            if self.fence.is_some() {
                self.close_fence();
            }
            if self.stable < shown {
                self.parse(shown, true, wake);
            }
            self.stable = shown;
        }
    }

    fn complete_line(&mut self, end: usize, wake: Option<super::hook::Wake>) {
        if let Some(fence) = &self.fence {
            let line = &self.source[self.stable..end];
            let rest = line.trim_start_matches(' ');
            let indent = line.len() - rest.len();
            let run = rest
                .bytes()
                .take_while(|byte| *byte == fence.marker)
                .count();
            if indent <= 3 && run >= fence.length && rest[run..].trim().is_empty() {
                self.close_fence();
                self.stable = end;
                self.gap = 1;
                return;
            }
            let source = code_line(line, fence.indent);
            #[cfg(feature = "markdown-highlight")]
            let waiting = if !fence.language.is_empty()
                && !fence.unknown.load(std::sync::atomic::Ordering::Acquire)
            {
                let id = (self.count + self.pending.len()) as u64;
                super::highlight::submit(super::highlight::Work::Line {
                    block: fence.id,
                    id,
                    language: fence.language.clone(),
                    theme: self.code_theme.clone(),
                    source: source.clone(),
                    reply: self.results.0.clone(),
                    wake,
                    unknown: fence.unknown.clone(),
                });
                Some(id)
            } else {
                None
            };
            #[cfg(not(feature = "markdown-highlight"))]
            let _ = wake;
            self.enqueue(
                0,
                Kind::Line {
                    source,
                    #[cfg(feature = "markdown-highlight")]
                    spans: None,
                },
            );
            #[cfg(feature = "markdown-highlight")]
            {
                self.pending.back_mut().expect("line").waiting = waiting;
            }
            self.stable = end;
        } else {
            self.parse(end, false, wake);
        }
    }

    fn close_fence(&mut self) {
        if let Some(_fence) = self.fence.take() {
            #[cfg(feature = "markdown-highlight")]
            if !_fence.language.is_empty() {
                super::highlight::submit(super::highlight::Work::Close(_fence.id));
            }
        }
    }

    fn parse(&mut self, end: usize, finish: bool, wake: Option<super::hook::Wake>) {
        let text = &self.source[self.stable..end];
        let parser = Parser::new_ext(text, options());
        for (name, definition) in parser.reference_definitions().iter() {
            self.links.entry(render::label(name)).or_insert_with(|| {
                (
                    definition.dest.to_string(),
                    definition.title.as_deref().unwrap_or("").to_owned(),
                )
            });
        }
        let blocks = block_starts(text, parser);
        if blocks.is_empty() {
            return;
        }
        let base = self.stable;
        for (index, block) in blocks.iter().enumerate() {
            let start = base + block.start;
            let stop = blocks.get(index + 1).map_or(end, |next| base + next.start);
            if let Some(language) = &block.fence {
                let line_end = self.source[start..end]
                    .find('\n')
                    .map_or(end, |at| start + at + 1);
                let line = &self.source[start..line_end];
                let rest = line.trim_start_matches(' ');
                let indent = line.len() - rest.len();
                let marker = rest.as_bytes()[0];
                let length = rest.bytes().take_while(|byte| *byte == marker).count();
                self.enqueue(self.gap, Kind::Opening(language.clone()));
                self.list = None;
                self.gap = 0;
                self.fence = Some(Fence {
                    marker,
                    length,
                    indent,
                    #[cfg(feature = "markdown-highlight")]
                    language: language.clone(),
                    #[cfg(feature = "markdown-highlight")]
                    unknown: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    #[cfg(feature = "markdown-highlight")]
                    id: {
                        use std::sync::atomic::{AtomicU64, Ordering};
                        static NEXT: AtomicU64 = AtomicU64::new(0);
                        NEXT.fetch_add(1, Ordering::Relaxed)
                    },
                });
                self.stable = line_end;
                if line_end < end {
                    self.complete_line(end, wake);
                }
                return;
            }
            let continuation = match (self.list, block.list) {
                (Some(old), Some(new)) => old.marker == new.marker,
                _ => false,
            };
            let number = if continuation {
                self.list.and_then(|list| list.next)
            } else {
                block.list.and_then(|list| list.next)
            };
            let gap = if continuation {
                u16::from(render::blank_before(&self.source, start))
            } else {
                self.gap
            };
            if index + 1 == blocks.len() && !finish {
                self.stable = start;
                self.gap = gap;
                self.list = block.list.map(|list| List {
                    next: number,
                    ..list
                });
                break;
            }
            let source: Rc<str> = Rc::from(&self.source[start..stop]);
            let mut links = render::Links::new();
            let callback = |link: pulldown_cmark::BrokenLink<'_>| {
                let label = render::label(&link.reference);
                self.links.get(&label).map(|(url, title)| {
                    links.insert(label, (url.clone(), title.clone()));
                    (url.clone().into(), title.clone().into())
                })
            };
            Parser::new_with_broken_link_callback(&source, options(), Some(callback))
                .for_each(drop);
            self.enqueue(
                gap,
                Kind::Document {
                    source,
                    number,
                    links,
                },
            );
            self.stable = stop;
            self.gap = 1;
            self.list = block.list.map(|list| List {
                next: number.map(|number| number + 1),
                ..list
            });
        }
    }

    pub(crate) fn view(
        &self,
        theme: &super::MarkdownTheme,
        cache: &std::cell::RefCell<render::Cache>,
        rows: usize,
        settle: bool,
    ) -> ink_rs::Element {
        use ink_rs::{FlexDirection, Styled, View};
        let mut view = View::new().flex_direction(FlexDirection::Column);
        let tail = &self.source[self.stable..self.shown];
        // Pending code is a viewport, not a growing tree of line components.
        let mut lines = Vec::new();
        for pending in self
            .pending
            .iter()
            .rev()
            .take(rows.saturating_sub(usize::from(self.fence.is_some() && !tail.is_empty())))
        {
            let element = match &pending.piece.inner.kind {
                Kind::Line { source, .. } => View::new()
                    .padding_left(2)
                    .child(super::piece::plain_line(source, theme))
                    .into(),
                _ => pending.piece.render(theme, cache),
            };
            lines.push(element);
        }
        view = view.children(lines.into_iter().rev());
        if let Some(fence) = &self.fence {
            if !tail.is_empty() {
                let tail = code_line(tail, fence.indent);
                view = view.child(
                    View::new()
                        .padding_left(2)
                        .child(super::piece::plain_line(&tail, theme)),
                );
            }
        } else if !tail.trim().is_empty() {
            let source = if settle {
                super::settle::settle(tail)
            } else {
                std::borrow::Cow::Borrowed(tail)
            };
            view = view.child(
                View::new()
                    .flex_direction(FlexDirection::Column)
                    .padding_top(self.gap)
                    .child(render::render_links(
                        &source,
                        theme,
                        cache,
                        self.list.and_then(|list| list.next),
                        &self.links,
                    )),
            );
        }
        view.into()
    }
}

impl Drop for MarkdownStream {
    fn drop(&mut self) {
        self.close_fence();
    }
}

fn block_starts<'a>(text: &'a str, parser: Parser<'a>) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut depth = 0usize;
    let mut list = None;
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    if let Tag::List(number) = tag {
                        let rest = text[range.start..].trim_start();
                        let marker = rest
                            .bytes()
                            .find(|byte| !byte.is_ascii_digit())
                            .unwrap_or(b'-');
                        list = Some(List {
                            marker,
                            next: number,
                        });
                    } else {
                        list = None;
                        blocks.push(Block {
                            start: line_start(text, range.start),
                            list: None,
                            fence: match &tag {
                                Tag::CodeBlock(CodeBlockKind::Fenced(info)) => {
                                    Some(info.split_whitespace().next().unwrap_or("").to_owned())
                                }
                                _ => None,
                            },
                        });
                    }
                } else if depth == 1 && matches!(tag, Tag::Item) {
                    blocks.push(Block {
                        start: line_start(text, range.start),
                        list,
                        fence: None,
                    });
                    if let Some(list) = &mut list {
                        list.next = list.next.map(|number| number + 1);
                    }
                }
                depth += 1;
            }
            Event::End(_) => {
                depth -= 1;
            }
            _ if depth == 0 => blocks.push(Block {
                start: line_start(text, range.start),
                list: None,
                fence: None,
            }),
            _ => {}
        }
    }
    blocks
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |at| at + 1)
}

fn code_line(line: &str, indent: usize) -> String {
    let mut columns = 0;
    let mut bytes = 0;
    let mut remainder = 0;
    for byte in line.bytes() {
        if columns >= indent {
            break;
        }
        let width = match byte {
            b' ' => 1,
            b'\t' => 4 - columns % 4,
            _ => break,
        };
        columns += width;
        bytes += 1;
        remainder = columns.saturating_sub(indent);
    }
    format!("{}{}", " ".repeat(remainder), &line[bytes..]).replace('\0', "\u{fffd}")
}
