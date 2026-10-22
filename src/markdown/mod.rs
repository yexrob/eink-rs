//! Markdown rendering for text that arrives in pieces.
//!
//! [`Markdown`] renders a document with [pulldown-cmark](https://docs.rs/pulldown-cmark):
//! CommonMark plus GitHub tables, task lists, strikethrough, footnotes and
//! alerts. [`MarkdownStream`] splits a growing source into finished top-level
//! blocks and a live tail. Put the finished blocks in a
//! [`Static`](ink_rs::Static) so they enter terminal history once, and render
//! the tail with [`Markdown::streaming`] set, which hides half-typed markers
//! and closes open inline styles.
//!
//! With the `markdown-highlight` feature, fenced code is highlighted with
//! [syntect](https://docs.rs/syntect) and the syntaxes and themes of
//! [bat](https://github.com/sharkdp/bat).

#[cfg(feature = "markdown-highlight")]
mod highlight;
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

pub use stream::MarkdownStream;
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
