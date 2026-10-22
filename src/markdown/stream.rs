use std::rc::Rc;
use std::time::Duration;

use pulldown_cmark::{Event, Parser};
use unicode_segmentation::UnicodeSegmentation;

use super::options;

/// A growing Markdown source split into finished blocks and a live tail.
///
/// A top-level block is finished once a later complete line starts another
/// one. CommonMark parses line by line and never reopens a closed block, so a
/// finished block renders the same whatever arrives after it. The one
/// exception: a link reference definition arriving later does not reach
/// blocks finished before it.
///
/// With pacing, [`push`](Self::push) only buffers text and
/// [`tick`](Self::tick) reveals it at a steady rate, so text that arrives in
/// bursts appears a few characters per frame.
#[derive(Clone, Debug, Default)]
pub struct MarkdownStream {
    source: String,
    shown: usize,
    stable: usize,
    blocks: Vec<Rc<str>>,
    done: bool,
    pacing: Option<Duration>,
}

impl MarkdownStream {
    /// Creates a stream that shows text as soon as it is pushed.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a stream that reveals text on [`tick`](Self::tick), showing
    /// on each tick the share of the backlog that would clear it within
    /// `catch_up`, and at least one grapheme.
    pub fn paced(catch_up: Duration) -> Self {
        Self {
            pacing: Some(catch_up),
            ..Self::default()
        }
    }

    /// Turns pacing on or off. Off shows the backlog at once.
    pub fn set_pacing(&mut self, catch_up: Option<Duration>) {
        self.pacing = catch_up;
        if catch_up.is_none() {
            self.reveal(self.source.len());
        }
    }

    /// Appends text.
    pub fn push(&mut self, delta: &str) {
        self.source.push_str(delta);
        if self.pacing.is_none() {
            self.reveal(self.source.len());
        }
    }

    /// Marks the end of the source. Once all of it is shown, the tail
    /// becomes the last finished blocks.
    pub fn finish(&mut self) {
        self.done = true;
        self.reveal(if self.pacing.is_none() {
            self.source.len()
        } else {
            self.shown
        });
    }

    /// Reveals paced text for `elapsed` time since the previous tick.
    pub fn tick(&mut self, elapsed: Duration) {
        let Some(catch_up) = self.pacing else {
            return;
        };
        let backlog = &self.source[self.shown..];
        if backlog.is_empty() {
            return;
        }
        let total = backlog.graphemes(true).count();
        let share = elapsed.as_secs_f64() / catch_up.as_secs_f64().max(f64::EPSILON);
        let count = ((total as f64 * share).ceil() as usize).max(1);
        let bytes = backlog
            .grapheme_indices(true)
            .nth(count)
            .map_or(backlog.len(), |(at, _)| at);
        self.reveal(self.shown + bytes);
    }

    /// Finished blocks, in order. Each one is a self-contained document.
    pub fn blocks(&self) -> &[Rc<str>] {
        &self.blocks
    }

    /// The shown text after the finished blocks.
    pub fn tail(&self) -> &str {
        &self.source[self.stable..self.shown]
    }

    /// Everything pushed so far.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Everything shown so far.
    pub fn shown(&self) -> &str {
        &self.source[..self.shown]
    }

    /// Whether paced text is waiting for [`tick`](Self::tick).
    pub fn is_revealing(&self) -> bool {
        self.shown < self.source.len()
    }

    /// Whether the source is finished and every block is out of the tail.
    pub fn is_done(&self) -> bool {
        self.done && self.stable == self.source.len()
    }

    fn reveal(&mut self, shown: usize) {
        self.shown = shown;
        let complete = self.done && shown == self.source.len();
        let end = if complete {
            shown
        } else {
            self.source[..shown].rfind('\n').map_or(0, |at| at + 1)
        };
        if end <= self.stable {
            return;
        }
        let text = &self.source[self.stable..end];
        let mut starts = block_starts(text);
        if starts.is_empty() {
            if complete {
                self.stable = end;
            }
            return;
        }
        // The first block keeps any link reference definitions before it.
        starts[0] = 0;
        let finished = if complete {
            starts.len()
        } else {
            starts.len() - 1
        };
        for (index, &start) in starts[..finished].iter().enumerate() {
            let stop = starts.get(index + 1).copied().unwrap_or(text.len());
            let block = text[start..stop].trim_end();
            if !block.is_empty() {
                self.blocks.push(Rc::from(block));
            }
        }
        self.stable += starts.get(finished).copied().unwrap_or(text.len());
    }
}

/// Byte offsets of the lines where top-level blocks start.
fn block_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut depth = 0_usize;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        let opens = match event {
            Event::Start(_) => {
                depth += 1;
                depth == 1
            }
            Event::End(_) => {
                depth -= 1;
                false
            }
            _ => depth == 0,
        };
        if opens {
            let line = text[..range.start].rfind('\n').map_or(0, |at| at + 1);
            if starts.last() != Some(&line) {
                starts.push(line);
            }
        }
    }
    starts
}
