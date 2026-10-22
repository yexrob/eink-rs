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
#[cfg(feature = "markdown-image")]
mod images;
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
#[cfg(feature = "markdown-image")]
pub use images::MarkdownImages;
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
    #[cfg(not(feature = "markdown-image"))]
    {
        render::render(&source, &theme, &cache)
    }
    #[cfg(feature = "markdown-image")]
    {
        use crate::image::{use_image_effect, use_images};
        use ink_rs::{ColorLevel, FlexDirection, GraphicsSupport, Styled, View};
        let graphics = cx.use_graphics();
        let level = cx.use_color_level();
        let (node, columns) = images::use_columns(cx);
        let links = render::Links::new();
        let requests = if graphics.support == GraphicsSupport::Kitty
            && matches!(level, ColorLevel::TrueColor | ColorLevel::Ansi256)
        {
            images::requests(&source, &links, columns, graphics.cell_pixels, level)
        } else {
            Vec::new()
        };
        let mut images = images::Images {
            statuses: use_images(cx, &requests),
            columns,
            cell_pixels: graphics.cell_pixels,
            ..Default::default()
        };
        let body = render::render_document(&source, &theme, &cache, None, &links, &mut images);
        use_image_effect(cx, images.placements);
        View::new()
            .node_ref(node)
            .width(ink_rs::Percent(100.0))
            .flex_direction(FlexDirection::Column)
            .child(body)
            .into()
    }
}

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
}
