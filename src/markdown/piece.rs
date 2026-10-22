use std::cell::RefCell;
use std::rc::Rc;

use ink_rs::{Element, FlexDirection, Props, Scope, Styled, Text, View, text_width};

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
    pub hang: Rc<[(Marker, bool)]>,
    #[cfg(feature = "markdown-image")]
    pub images: super::images::Images,
}

#[derive(Clone, Copy)]
pub(crate) enum Marker {
    Number(u64),
    Bullet(usize),
    Task(bool),
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
        spans: Option<crate::highlight::Spans>,
    },
}

impl PartialEq for MarkdownPiece {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl MarkdownPiece {
    pub(crate) fn new(gap: u16, kind: Kind, hang: Rc<[(Marker, bool)]>) -> Self {
        Self {
            inner: Rc::new(Content {
                gap,
                kind,
                hang,
                #[cfg(feature = "markdown-image")]
                images: Default::default(),
            }),
        }
    }

    /// Blank rows preceding this piece.
    pub fn spacing(&self) -> u16 {
        self.inner.gap
    }

    pub(crate) fn render(&self, theme: &MarkdownTheme, cache: &RefCell<render::Cache>) -> Element {
        self.render_with(
            theme,
            cache,
            #[cfg(feature = "markdown-image")]
            &mut super::images::Images::default(),
        )
    }

    pub(crate) fn render_with(
        &self,
        theme: &MarkdownTheme,
        cache: &RefCell<render::Cache>,
        #[cfg(feature = "markdown-image")] images: &mut super::images::Images,
    ) -> Element {
        let body: Element = match &self.inner.kind {
            Kind::Document {
                source,
                number,
                links,
            } => render::render_document(
                source,
                &nested_theme(theme, self.inner.hang.len()),
                cache,
                *number,
                links,
                #[cfg(feature = "markdown-image")]
                images,
            ),
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
                    } => crate::highlight::text(spans),
                    _ => plain_line(source, theme),
                };
                #[cfg(not(feature = "markdown-highlight"))]
                let text = plain_line(source, theme);
                View::new().padding_left(2).child(text).into()
            }
        };
        hanging(
            View::new()
                .flex_direction(FlexDirection::Column)
                .padding_top(self.inner.gap)
                .child(body)
                .into(),
            &self.inner.hang,
            theme,
        )
    }
}

pub(crate) fn nested_theme(theme: &MarkdownTheme, depth: usize) -> MarkdownTheme {
    let mut theme = theme.clone();
    let count = theme.bullets.len();
    if count > 0 {
        theme.bullets.rotate_left(depth % count);
    }
    theme
}

pub(crate) fn hanging(
    mut body: Element,
    hang: &[(Marker, bool)],
    theme: &MarkdownTheme,
) -> Element {
    for (marker, shown) in hang.iter().rev() {
        let marker = marker_text(marker, theme);
        let mut column = View::new()
            .width(u16::try_from(text_width(&marker) + 1).unwrap_or(u16::MAX))
            .flex_shrink(0.0);
        if *shown {
            column = column.child(Text::new(marker).color(theme.bullet_color));
        }
        body = View::new()
            .child(column)
            .child(
                View::new()
                    .flex_direction(FlexDirection::Column)
                    .flex_grow(1.0)
                    .min_width(0)
                    .child(body),
            )
            .into();
    }
    body
}

fn marker_text(marker: &Marker, theme: &MarkdownTheme) -> String {
    match marker {
        Marker::Number(number) => format!("{number}."),
        Marker::Bullet(depth) => theme
            .bullets
            .get(depth % theme.bullets.len().max(1))
            .cloned()
            .unwrap_or_else(|| "-".into()),
        Marker::Task(true) => theme.task_done.clone(),
        Marker::Task(false) => theme.task_open.clone(),
    }
}

#[cfg(feature = "markdown-image")]
pub(crate) fn hanging_width(hang: &[(Marker, bool)], theme: &MarkdownTheme) -> usize {
    hang.iter()
        .map(|(marker, _)| text_width(&marker_text(marker, theme)) + 1)
        .sum()
}

pub(crate) fn plain_line(source: &str, theme: &MarkdownTheme) -> Text {
    let source = source.trim_end_matches(['\r', '\n']);
    Text::new(if source.is_empty() { " " } else { source }).color(theme.code_block_color)
}

fn piece(cx: &mut Scope, props: &MarkdownPiece) -> Element {
    let theme = use_theme::<MarkdownTheme>(cx);
    let cache = cx.use_memo((), Rc::<RefCell<render::Cache>>::default);
    #[cfg(not(feature = "markdown-image"))]
    {
        props.render(&theme, &cache)
    }
    #[cfg(feature = "markdown-image")]
    {
        let mut images = props.inner.images.clone();
        let body = props.render_with(&theme, &cache, &mut images);
        crate::image::use_image_effect(cx, images.placements);
        body
    }
}
