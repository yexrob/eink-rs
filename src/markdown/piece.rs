use std::cell::RefCell;
use std::rc::Rc;

use ink_rs::{Element, FlexDirection, Props, Scope, Styled, Text, View};

use super::{MarkdownTheme, render};
use crate::ui::use_theme;

/// A final part of a Markdown stream, including its leading blank rows.
/// Cloning and equality compare shared storage, not the document text.
#[derive(Clone, Props)]
#[props(component = piece)]
pub struct MarkdownPiece {
    pub(crate) inner: Rc<Content>,
}

pub(crate) struct Content {
    pub gap: u16,
    pub kind: Kind,
}

pub(crate) enum Kind {
    Document {
        source: Rc<str>,
        number: Option<u64>,
        links: render::Links,
    },
    Opening(String),
    Line {
        source: String,
        #[cfg(feature = "markdown-highlight")]
        spans: Option<super::highlight::Spans>,
    },
}

impl PartialEq for MarkdownPiece {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl MarkdownPiece {
    pub(crate) fn new(gap: u16, kind: Kind) -> Self {
        Self {
            inner: Rc::new(Content { gap, kind }),
        }
    }

    /// Blank rows preceding this piece.
    pub fn spacing(&self) -> u16 {
        self.inner.gap
    }

    pub(crate) fn render(&self, theme: &MarkdownTheme, cache: &RefCell<render::Cache>) -> Element {
        let body: Element = match &self.inner.kind {
            Kind::Document {
                source,
                number,
                links,
            } => render::render_links(source, theme, cache, *number, links),
            Kind::Opening(language) => {
                let mut view = View::new();
                if !language.is_empty() {
                    view = view.child(Text::new(language).dim_color(true));
                }
                view.into()
            }
            Kind::Line { source, .. } => {
                #[cfg(feature = "markdown-highlight")]
                let text = match &self.inner.kind {
                    Kind::Line {
                        spans: Some(spans), ..
                    } => super::highlight::text(spans),
                    _ => plain_line(source, theme),
                };
                #[cfg(not(feature = "markdown-highlight"))]
                let text = plain_line(source, theme);
                View::new().padding_left(2).child(text).into()
            }
        };
        View::new()
            .flex_direction(FlexDirection::Column)
            .padding_top(self.inner.gap)
            .child(body)
            .into()
    }
}

pub(crate) fn plain_line(source: &str, theme: &MarkdownTheme) -> Text {
    let source = source.trim_end_matches(['\r', '\n']);
    Text::new(if source.is_empty() { " " } else { source }).color(theme.code_block_color)
}

fn piece(cx: &mut Scope, props: &MarkdownPiece) -> Element {
    let theme = use_theme::<MarkdownTheme>(cx);
    let cache = cx.use_memo((), Rc::<RefCell<render::Cache>>::default);
    props.render(&theme, &cache)
}
