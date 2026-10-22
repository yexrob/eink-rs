use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use unicode_segmentation::UnicodeSegmentation;

use super::piece::{Kind, MarkdownPiece, Marker};
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
pub struct MarkdownStream {
    source: String,
    shown: usize,
    scanned: usize,
    stable: usize,
    backlog: usize,
    finished: bool,
    pacing: Option<Duration>,
    frames: Vec<Frame>,
    count: usize,
    fence: Option<Fence>,
    pending: VecDeque<Pending>,
    links: render::Links,
    #[cfg(feature = "markdown-image")]
    pub(crate) images: super::images::StreamImages,
    #[cfg(feature = "markdown-highlight")]
    results: (
        std::sync::mpsc::Sender<crate::highlight::ResultLine>,
        std::sync::mpsc::Receiver<crate::highlight::ResultLine>,
    ),
    #[cfg(feature = "markdown-highlight")]
    code_theme: String,
    /// Lets the highlight thread skip the queued lines of a dropped stream.
    #[cfg(feature = "markdown-highlight")]
    alive: std::sync::Arc<()>,
}

pub(super) struct Pending {
    start: usize,
    piece: MarkdownPiece,
    #[cfg(feature = "markdown-highlight")]
    waiting: Option<u64>,
    #[cfg(feature = "markdown-image")]
    pub(super) images: Option<super::images::Waiting>,
}

#[derive(Clone, Copy)]
struct List {
    marker: u8,
    next: Option<u64>,
}

#[derive(Default)]
struct Frame {
    // Where the item's line starts, past the parent items' indentation.
    start: usize,
    cols: usize,
    marker: Option<Marker>,
    shown: bool,
    // An empty unlabeled fence is a child without a visible marker.
    started: bool,
    list: Option<List>,
    gap: u16,
    // The tail's first block was given `gap` and the number in `list` when
    // it was held; read again, it would match its own list.
    held: bool,
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
    item: Option<Item>,
}

struct Item {
    content: usize,
    cols: usize,
    task: Option<bool>,
}

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
            frames: vec![Frame::default()],
            count: 0,
            fence: None,
            pending: VecDeque::new(),
            links: render::Links::new(),
            #[cfg(feature = "markdown-image")]
            images: Default::default(),
            #[cfg(feature = "markdown-highlight")]
            results: std::sync::mpsc::channel(),
            #[cfg(feature = "markdown-highlight")]
            code_theme: "ansi".into(),
            #[cfg(feature = "markdown-highlight")]
            alive: std::sync::Arc::new(()),
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
        #[cfg(feature = "markdown-image")]
        {
            self.images.time += elapsed;
            self.poll_images();
        }
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
        #[cfg(feature = "markdown-image")]
        self.poll_images();
        let mut pieces = Vec::new();
        while self.pending.front().is_some_and(|pending| {
            #[cfg(feature = "markdown-image")]
            if pending.images.is_some() {
                return false;
            }
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

    /// Sets image handling for future pieces and resolves pieces waiting for the probe.
    #[cfg(feature = "markdown-image")]
    pub fn set_images(&mut self, images: super::MarkdownImages) {
        self.set_images_with(images, None);
    }

    #[cfg(feature = "markdown-image")]
    pub(crate) fn set_images_with(
        &mut self,
        images: super::MarkdownImages,
        wake: Option<super::hook::Wake>,
    ) {
        self.images.setting = images;
        for (index, pending) in self.pending.iter_mut().enumerate() {
            if let Some(waiting) = &mut pending.images {
                if matches!(self.images.setting, super::MarkdownImages::Alt) && waiting.probing() {
                    pending.images = None;
                } else {
                    self.images.start(waiting, self.count + index, wake.clone());
                }
            }
        }
        self.poll_images();
    }

    /// Sets the columns available to images in later pieces; the default is 80.
    #[cfg(feature = "markdown-image")]
    pub fn set_columns(&mut self, columns: usize) {
        self.images.columns = columns;
    }

    /// Waits for one image result or its deadline. Returns false when no load is pending.
    #[cfg(feature = "markdown-image")]
    pub fn wait_for_images(&mut self) -> bool {
        let waited = self.images.wait(&mut self.pending, self.count);
        self.poll_images();
        waited
    }

    #[cfg(feature = "markdown-image")]
    pub(crate) fn waiting_for_images(&self) -> bool {
        self.pending.iter().any(|pending| pending.images.is_some())
    }

    #[cfg(feature = "markdown-image")]
    fn poll_images(&mut self) {
        self.images.poll(&mut self.pending, self.count);
        for pending in &mut self.pending {
            if pending
                .images
                .as_ref()
                .is_some_and(super::images::Waiting::ready)
            {
                let images = pending.images.take().expect("ready images");
                pending.piece = images.finish(&pending.piece);
            }
        }
    }

    #[cfg(feature = "markdown-highlight")]
    fn accept(&mut self, result: crate::highlight::ResultLine) {
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
                pending.piece.inner.hang.clone(),
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

    fn frame(&self) -> &Frame {
        self.frames.last().expect("root frame")
    }

    fn frame_mut(&mut self) -> &mut Frame {
        self.frames.last_mut().expect("root frame")
    }

    fn cols(&self) -> usize {
        self.frames.iter().map(|frame| frame.cols).sum()
    }

    fn hang(&self) -> Rc<[(Marker, bool)]> {
        self.frames
            .iter()
            .filter_map(|frame| frame.marker.map(|marker| (marker, !frame.shown)))
            .collect()
    }

    fn enqueue(&mut self, gap: u16, kind: Kind) {
        let visible = gap > 0 || !matches!(&kind, Kind::Opening(language) if language.is_empty());
        let hang = self
            .frames
            .iter_mut()
            .filter_map(|frame| {
                frame.started = true;
                frame.marker.map(|marker| {
                    let show = visible && !frame.shown;
                    frame.shown |= visible;
                    (marker, show)
                })
            })
            .collect();
        self.pending.push_back(Pending {
            start: self.stable,
            piece: MarkdownPiece::new(gap, kind, hang),
            #[cfg(feature = "markdown-highlight")]
            waiting: None,
            #[cfg(feature = "markdown-image")]
            images: None,
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
        // In a fence, or with no paragraph open above it, an unfinished line
        // that dedents leaves its items before its newline arrives, so the
        // tail draws it outside them.
        let start = self.scanned;
        if self.frames.len() > 1
            && start < shown
            && (self.fence.is_some()
                || self.source[self.stable..start].trim().is_empty()
                || self.source[line_start(&self.source, start - 1)..start]
                    .trim()
                    .is_empty())
        {
            self.close_items(start, shown, wake.clone());
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
            self.show_marker();
            self.stable = shown;
        }
    }

    fn complete_line(&mut self, end: usize, wake: Option<super::hook::Wake>) {
        let last = end - usize::from(self.source[..end].ends_with('\n'));
        let start = line_start(&self.source, last);
        self.close_items(start, end, wake.clone());
        if let Some(fence) = &self.fence {
            let stripped = Stripped::new(&self.source, self.stable, end, self.cols());
            let line = stripped.text.as_str();
            let rest = line.trim_start_matches(' ');
            let indent = line.len() - rest.len();
            let run = rest
                .bytes()
                .take_while(|byte| *byte == fence.marker)
                .count();
            if indent <= 3 && run >= fence.length && rest[run..].trim().is_empty() {
                self.close_fence();
                self.stable = end;
                self.frame_mut().gap = 1;
                return;
            }
            let source = code_line(&self.source[self.stable..end], self.cols() + fence.indent);
            #[cfg(feature = "markdown-highlight")]
            let waiting = if !fence.language.is_empty()
                && !fence.unknown.load(std::sync::atomic::Ordering::Acquire)
            {
                let id = (self.count + self.pending.len()) as u64;
                crate::highlight::submit(crate::highlight::Work::Line {
                    block: fence.id,
                    id,
                    language: crate::highlight::Lang::Token(fence.language.clone()),
                    theme: self.code_theme.clone(),
                    source: source.clone(),
                    reply: self.results.0.clone(),
                    wake: wake.map(super::hook::Wake::callback),
                    unknown: fence.unknown.clone(),
                    stream: std::sync::Arc::downgrade(&self.alive),
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

    // Leaves the items the line at `start` is indented less than, unless the
    // line is a lazy continuation of a paragraph.
    fn close_items(&mut self, start: usize, end: usize, wake: Option<super::hook::Wake>) {
        let line = &self.source[start..end];
        if line.trim().is_empty() {
            return;
        }
        let indent = indentation(line);
        while self.frames.len() > 1 && indent < self.cols() {
            if self.fence.is_none() {
                // The line is lazy when the item holds it. Parsed from the
                // marker: the tail alone reads a sibling item as text that
                // cannot interrupt a paragraph.
                let frame = self.frame();
                let item = Stripped::new(&self.source, frame.start, end, self.cols() - frame.cols);
                let lazy = Parser::new_ext(&item.text, options())
                    .into_offset_iter()
                    .find(|(event, _)| matches!(event, Event::Start(Tag::Item)))
                    .is_some_and(|(_, range)| item.offset(range.end) > start);
                if lazy {
                    break;
                }
                self.parse(start, true, wake.clone());
            }
            self.close_fence();
            self.show_marker();
            self.frames.pop();
            self.stable = start;
        }
    }

    fn close_fence(&mut self) {
        if let Some(_fence) = self.fence.take() {
            #[cfg(feature = "markdown-highlight")]
            if !_fence.language.is_empty() {
                crate::highlight::submit(crate::highlight::Work::Close(_fence.id));
            }
        }
    }

    fn show_marker(&mut self) {
        // Later children can share the marker's row until the item ends.
        if self.frame().marker.is_some() && !self.frame().shown {
            self.enqueue(
                0,
                Kind::Document {
                    source: Rc::from(""),
                    number: None,
                    links: render::Links::new(),
                },
            );
        }
    }

    fn parse(&mut self, end: usize, finish: bool, wake: Option<super::hook::Wake>) {
        let stripped = Stripped::document(&self.source, self.stable, end, self.cols());
        let text = stripped.text.as_str();
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
        let held = std::mem::take(&mut self.frame_mut().held);
        for (index, block) in blocks.iter().enumerate() {
            let start = stripped.offset(block.start);
            let stop_at = blocks.get(index + 1).map_or(text.len(), |next| next.start);
            let stop = stripped.offset(stop_at);
            let held = held && index == 0;
            let continuation = !held
                && match (self.frame().list, block.list) {
                    (Some(old), Some(new)) => old.marker == new.marker,
                    _ => false,
                };
            let number = if continuation || held {
                self.frame().list.and_then(|list| list.next)
            } else {
                block.list.and_then(|list| list.next)
            };
            let gap = if !held && (continuation || (self.frames.len() > 1 && self.frame().started))
            {
                u16::from(render::blank_before(&self.source, start))
            } else {
                self.frame().gap
            };
            if let Some(item) = &block.item
                && !self.source[start..stop].lines().any(|line| {
                    let rest = line.trim_start_matches([' ', '\t']);
                    line.bytes()
                        .take_while(|byte| byte.is_ascii_whitespace())
                        .any(|byte| byte == b'\t')
                        || marker_len(rest).is_some_and(|length| {
                            rest[length..]
                                .bytes()
                                .take_while(|byte| byte.is_ascii_whitespace())
                                .any(|byte| byte == b'\t')
                        })
                })
            {
                self.frame_mut().list = block.list.map(|list| List {
                    next: number.map(|number| number + 1),
                    ..list
                });
                self.frame_mut().gap = 1;
                let marker = item.task.map(Marker::Task).unwrap_or_else(|| {
                    number.map_or(Marker::Bullet(self.frames.len() - 1), Marker::Number)
                });
                if gap > 0 {
                    self.enqueue(gap, Kind::Opening(String::new()));
                }
                self.frames.push(Frame {
                    start,
                    cols: item.cols,
                    marker: Some(marker),
                    ..Frame::default()
                });
                self.stable = stripped.offset(item.content);
                self.parse(end, finish, wake);
                return;
            }
            let prefix = &self.source[start
                ..self.source[start..end]
                    .find('\n')
                    .map_or(end, |at| start + at)];
            let tabbed = self.frames.len() > 1
                && prefix
                    .bytes()
                    .take_while(|byte| byte.is_ascii_whitespace())
                    .any(|byte| byte == b'\t');
            if let Some(language) = &block.fence
                && !tabbed
            {
                let line_end = self.source[start..end]
                    .find('\n')
                    .map_or(end, |at| start + at + 1);
                let line = &text[block.start
                    ..text[block.start..]
                        .find('\n')
                        .map_or(text.len(), |at| block.start + at + 1)];
                let rest = line.trim_start_matches(' ');
                let indent = line.len() - rest.len();
                let marker = rest.as_bytes()[0];
                let length = rest.bytes().take_while(|byte| *byte == marker).count();
                self.enqueue(gap, Kind::Opening(language.clone()));
                self.frame_mut().list = None;
                self.frame_mut().gap = 0;
                self.fence = Some(Fence {
                    marker,
                    length,
                    indent,
                    #[cfg(feature = "markdown-highlight")]
                    language: language.clone(),
                    #[cfg(feature = "markdown-highlight")]
                    unknown: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    #[cfg(feature = "markdown-highlight")]
                    id: crate::highlight::block_id(),
                });
                self.stable = line_end;
                if line_end < end {
                    self.complete_line(end, wake);
                }
                return;
            }
            if index + 1 == blocks.len() && !finish {
                self.stable = start;
                self.frame_mut().held = true;
                self.frame_mut().gap = gap;
                self.frame_mut().list = block.list.map(|list| List {
                    next: number,
                    ..list
                });
                break;
            }
            let source: Rc<str> = Rc::from(&text[block.start..stop_at]);
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
            #[cfg(feature = "markdown-image")]
            {
                let id = self.count + self.pending.len() - 1;
                let pending = self.pending.back_mut().expect("document piece");
                pending.images = self.images.enqueue(&pending.piece, id, wake.clone());
            }
            self.stable = stop;
            self.frame_mut().gap = 1;
            self.frame_mut().list = block.list.map(|list| List {
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
        let stripped = if self.fence.is_some() {
            Stripped::new(&self.source, self.stable, self.shown, self.cols())
        } else {
            Stripped::document(&self.source, self.stable, self.shown, self.cols())
        };
        let tail = stripped.text.as_str();
        // Pending code is a viewport, not a growing tree of line components.
        let mut lines = Vec::new();
        for pending in self
            .pending
            .iter()
            .rev()
            .take(rows.saturating_sub(usize::from(self.fence.is_some() && !tail.is_empty())))
        {
            let element = match &pending.piece.inner.kind {
                Kind::Line { source, .. } => super::piece::hanging(
                    View::new()
                        .padding_left(2)
                        .child(super::piece::plain_line(source, theme))
                        .into(),
                    &pending.piece.inner.hang,
                    theme,
                ),
                _ => pending.piece.render(theme, cache),
            };
            lines.push(element);
        }
        view = view.children(lines.into_iter().rev());
        if let Some(fence) = &self.fence {
            if !tail.is_empty() {
                let tail = code_line(
                    &self.source[self.stable..self.shown],
                    self.cols() + fence.indent,
                );
                view = view.child(super::piece::hanging(
                    View::new()
                        .padding_left(2)
                        .child(super::piece::plain_line(&tail, theme))
                        .into(),
                    &self.hang(),
                    theme,
                ));
            }
        } else if !tail.trim().is_empty() {
            let gap = if self.frames.len() > 1 && self.frame().started {
                let at = tail.find(|c: char| !c.is_whitespace()).unwrap_or(0);
                u16::from(render::blank_before(&self.source, stripped.offset(at)))
            } else {
                self.frame().gap
            };
            let source = if settle {
                super::settle::settle(tail)
            } else {
                std::borrow::Cow::Borrowed(tail)
            };
            view = view.child(super::piece::hanging(
                View::new()
                    .flex_direction(FlexDirection::Column)
                    .padding_top(gap)
                    .child(render::render_links(
                        &source,
                        &super::piece::nested_theme(theme, self.frames.len() - 1),
                        cache,
                        self.frame().list.and_then(|list| list.next),
                        &self.links,
                    ))
                    .into(),
                &self.hang(),
                theme,
            ));
        }
        if tail.is_empty() && self.frame().marker.is_some() && !self.frame().shown {
            view = view.child(super::piece::hanging(
                View::new().into(),
                &self.hang(),
                theme,
            ));
        }
        view.into()
    }
}

impl Drop for MarkdownStream {
    fn drop(&mut self) {
        self.close_fence();
    }
}

// Parser offsets map back to the source, including expanded indentation tabs.
struct Stripped {
    text: String,
    lines: Vec<(std::ops::Range<usize>, std::ops::Range<usize>)>,
    end: usize,
}

impl Stripped {
    fn new(source: &str, start: usize, end: usize, cols: usize) -> Self {
        Self::with_code(source, start, end, cols, None)
    }

    fn document(source: &str, start: usize, end: usize, cols: usize) -> Self {
        let stripped = Self::new(source, start, end, cols);
        if cols == 0 || !source[start..end].contains('\t') {
            return stripped;
        }
        let mut path = Vec::new();
        let mut code = None;
        let mut lines = std::collections::HashMap::new();
        for (event, range) in Parser::new_ext(&stripped.text, options()).into_offset_iter() {
            match event {
                Event::Start(tag) => {
                    if matches!(tag, Tag::CodeBlock(CodeBlockKind::Indented))
                        && path.iter().all(|valid| *valid)
                    {
                        let indent = range.start - line_start(&stripped.text, range.start);
                        code = Some((cols + indent, indent));
                    }
                    if matches!(tag, Tag::CodeBlock(CodeBlockKind::Fenced(_)))
                        && path.iter().all(|valid| *valid)
                    {
                        let at = range.start
                            + stripped.text[range.start..]
                                .bytes()
                                .take_while(|byte| *byte == b' ')
                                .count();
                        let raw = stripped.offset(at);
                        let column = source[line_start(source, raw)..raw]
                            .bytes()
                            .fold(0, |column, byte| {
                                column + if byte == b'\t' { 4 - column % 4 } else { 1 }
                            });
                        code = Some((column, at - line_start(&stripped.text, at)));
                    }
                    path.push(matches!(tag, Tag::List(_) | Tag::Item));
                }
                Event::End(tag) => {
                    if tag == TagEnd::CodeBlock {
                        code = None;
                    }
                    path.pop();
                }
                Event::Text(_) => {
                    if let Some(code) = code {
                        let mut at = line_start(source, stripped.offset(range.start));
                        let end = stripped.offset(range.end);
                        for line in source[at..end].split_inclusive('\n') {
                            lines.insert(at, code);
                            at += line.len();
                        }
                    }
                }
                _ => {}
            }
        }
        if lines.is_empty() {
            stripped
        } else {
            Self::with_code(source, start, end, cols, Some(&lines))
        }
    }

    fn with_code(
        source: &str,
        start: usize,
        end: usize,
        cols: usize,
        code: Option<&std::collections::HashMap<usize, (usize, usize)>>,
    ) -> Self {
        let mut stripped = Self {
            text: String::new(),
            lines: Vec::new(),
            end,
        };
        let mut offset = start;
        for line in source[start..end].split_inclusive('\n') {
            let begin = stripped.text.len();
            // Fallback code keeps literal tabs beyond its content column.
            if let Some(&(cols, indent)) = code.and_then(|code| code.get(&offset)) {
                let mut column = 0;
                let mut bytes = 0;
                for byte in line.bytes() {
                    if column >= cols || !matches!(byte, b' ' | b'\t') {
                        break;
                    }
                    column += if byte == b'\t' { 4 - column % 4 } else { 1 };
                    bytes += 1;
                }
                stripped.text.extend(std::iter::repeat_n(
                    ' ',
                    indent + column.saturating_sub(cols),
                ));
                stripped
                    .lines
                    .push((begin..stripped.text.len(), offset..offset + bytes));
                stripped.text.push_str(&line[bytes..]);
                offset += line.len();
                continue;
            }
            let bytes = line
                .bytes()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count();
            if cols == 0 {
                stripped.lines.push((begin..begin, offset..offset));
                stripped.text.push_str(line);
            } else {
                let column = source[line_start(source, offset)..offset]
                    .bytes()
                    .fold(0usize, |column, byte| {
                        column + if byte == b'\t' { 4 - column % 4 } else { 1 }
                    });
                let width = line[..bytes].bytes().fold(column, |column, byte| {
                    column + if byte == b'\t' { 4 - column % 4 } else { 1 }
                });
                stripped.text.extend(std::iter::repeat_n(
                    ' ',
                    width.saturating_sub(cols.max(column)),
                ));
                stripped
                    .lines
                    .push((begin..stripped.text.len(), offset..offset + bytes));
                let mut at = bytes;
                if let Some(length) = marker_len(&line[at..]) {
                    stripped.text.push_str(&line[at..at + length]);
                    at += length;
                    let mut column = width + length;
                    while let Some(byte @ (b' ' | b'\t')) = line.as_bytes().get(at) {
                        let begin = stripped.text.len();
                        let cells = if *byte == b'\t' { 4 - column % 4 } else { 1 };
                        stripped.text.extend(std::iter::repeat_n(' ', cells));
                        stripped
                            .lines
                            .push((begin..stripped.text.len(), offset + at..offset + at + 1));
                        column += cells;
                        at += 1;
                    }
                }
                stripped.text.push_str(&line[at..]);
            }
            offset += line.len();
        }
        stripped
    }

    fn offset(&self, at: usize) -> usize {
        if at == self.text.len() {
            return self.end;
        }
        let index = self.lines.partition_point(|(text, _)| text.start <= at) - 1;
        let (text, source) = &self.lines[index];
        if at == text.start || at < text.end {
            source.start
        } else {
            source.end + at - text.end
        }
    }
}

fn indentation(line: &str) -> usize {
    line.bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .fold(0, |column, byte| {
            column + if byte == b'\t' { 4 - column % 4 } else { 1 }
        })
}

fn item_content(text: &str, start: usize) -> Option<(usize, usize)> {
    let line = line_start(text, start);
    if !text[line..start].bytes().all(|byte| byte == b' ') {
        return None;
    }
    let indent = text[start..]
        .bytes()
        .take_while(|byte| *byte == b' ')
        .count();
    let start = start + indent;
    let rest = &text[start..];
    let marker = marker_len(rest)?;
    let after = rest.get(marker..)?;
    let spaces = after.bytes().take_while(|byte| *byte == b' ').count();
    if spaces == 0 || after[spaces..].starts_with('\t') {
        return None;
    }
    let padding = if spaces > 4 { 1 } else { spaces };
    let content = start + marker + padding;
    Some((content, content - line))
}

fn marker_len(rest: &str) -> Option<usize> {
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    match rest.as_bytes().get(digits) {
        Some(b'-' | b'*' | b'+') if digits == 0 => Some(1),
        Some(b'.' | b')') if (1..=9).contains(&digits) => Some(digits + 1),
        _ => None,
    }
}

fn stream_item(text: &str, start: usize, end: usize) -> Option<Item> {
    let (mut content, cols) = item_content(text, start)?;
    let mut path = Vec::new();
    let mut task = None;
    let mut found = false;
    for (event, range) in Parser::new_ext(&text[start..end], options()).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if let Tag::CodeBlock(CodeBlockKind::Fenced(_)) = &tag
                    && path.iter().all(|valid| *valid)
                {
                    let at = start + range.start;
                    let prefix = &text[line_start(text, at)..at];
                    if !prefix.contains('\t') {
                        found = true;
                        break;
                    }
                }
                path.push(match tag {
                    Tag::List(_) => true,
                    Tag::Item => item_content(&text[start + range.start..], 0).is_some(),
                    _ => false,
                });
            }
            Event::End(_) => {
                path.pop();
            }
            Event::TaskListMarker(done) if task.is_none() && path.len() <= 3 => {
                task = Some(done);
            }
            _ => {}
        }
    }
    if !found {
        return None;
    }
    if task.is_some() {
        let rest = &text[content..];
        if rest.len() < 4 || rest[3..].trim_start().starts_with(['`', '~']) {
            return None;
        }
        content += 4;
    }
    Some(Item {
        content,
        cols,
        task,
    })
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
                            item: None,
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
                        item: stream_item(text, range.start, range.end),
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
                item: None,
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
