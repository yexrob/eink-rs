//! Markdown documents and streamed model replies.
//!
//! [`use_markdown_stream`] owns one model. Pass an app handle to
//! [`MarkdownStreamHandle::sender`], send deltas from a producer thread, and
//! put each [`MarkdownPiece`] in the application's `Static` history.
//! [`MarkdownTail`] renders the remaining live region. Pieces carry spacing;
//! stack them without an extra gap. [`MarkdownStream`] is the headless layer.
//!
//! The `markdown-highlight` feature highlights streamed top-level fences on
//! one background thread. [`Markdown`] highlights finished documents with a
//! synchronous line cache. Later link definitions cannot change emitted pieces.
//! Footnote references keep the surrounding text color so a later definition
//! does not recolor an emitted reference. Unknown code languages use the plain
//! code color; their first lines wait for the worker to identify the language.

mod hook;
mod piece;
mod render;
mod settle;
mod stream;
mod theme;

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use ink_rs::{Element, Props, Scope};
use pulldown_cmark::Options;

use crate::ui::use_theme;

pub use hook::{
    MarkdownProgress, MarkdownSender, MarkdownStreamHandle, MarkdownTail, use_markdown_stream,
};
pub use piece::MarkdownPiece;
pub use stream::{MarkdownStream, MarkdownStreamOptions};
pub use theme::MarkdownTheme;

/// A rendered Markdown document.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = markdown)]
pub struct Markdown {
    /// The document source.
    #[props(into)]
    pub source: Rc<str>,
    /// Treats the source as the live tail of a stream: a half-typed block
    /// marker stays hidden until its line goes on, and open inline styles
    /// render as if their closing markers had arrived.
    pub streaming: bool,
}

impl Markdown {
    /// Creates a document from its source.
    pub fn new(source: impl Into<Rc<str>>) -> Self {
        Self {
            source: source.into(),
            streaming: false,
        }
    }
}

fn markdown(cx: &mut Scope, props: &Markdown) -> Element {
    let theme = use_theme::<MarkdownTheme>(cx);
    let cache = cx.use_memo((), Rc::<RefCell<render::Cache>>::default);
    let source = if props.streaming {
        settle::settle(&props.source)
    } else {
        Cow::Borrowed(&*props.source)
    };
    render::render(&source, &theme, &cache)
}

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
}
