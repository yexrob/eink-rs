use std::cell::RefCell;

use ink_rs::{
    BorderStyle, Color, Element, FlexDirection, JustifyContent, Styled, Text, TextWrap, View,
    text_width,
};
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd,
};

use super::{MarkdownTheme, options};

#[cfg(feature = "markdown-highlight")]
pub(crate) use crate::highlight::Cache;

#[cfg(not(feature = "markdown-highlight"))]
#[derive(Default)]
pub(crate) struct Cache;

/// Renders a document: top-level blocks one blank line apart.
#[cfg(not(feature = "markdown-image"))]
pub(crate) fn render(source: &str, theme: &MarkdownTheme, cache: &RefCell<Cache>) -> Element {
    render_numbered(source, theme, cache, None)
}

#[cfg(not(feature = "markdown-image"))]
pub(crate) fn render_numbered(
    source: &str,
    theme: &MarkdownTheme,
    cache: &RefCell<Cache>,
    number: Option<u64>,
) -> Element {
    render_links(source, theme, cache, number, &Links::new())
}

pub(crate) type Links = std::collections::HashMap<String, (String, String)>;

pub(crate) fn label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(crate) fn render_links(
    source: &str,
    theme: &MarkdownTheme,
    cache: &RefCell<Cache>,
    number: Option<u64>,
    links: &Links,
) -> Element {
    render_document(
        source,
        theme,
        cache,
        number,
        links,
        #[cfg(feature = "markdown-image")]
        &mut super::images::Images::default(),
    )
}

pub(crate) fn render_document(
    source: &str,
    theme: &MarkdownTheme,
    cache: &RefCell<Cache>,
    number: Option<u64>,
    links: &Links,
    #[cfg(feature = "markdown-image")] images: &mut super::images::Images,
) -> Element {
    let mut builder = Builder {
        source,
        offset: 0,
        number,
        theme,
        cache,
        stack: vec![Frame::new(Kind::Root)],
        inline: None,
        spans: Vec::new(),
        links: Vec::new(),
        code: None,
        html: None,
        table: None,
        #[cfg(feature = "markdown-image")]
        images,
        #[cfg(feature = "markdown-image")]
        after_image: false,
    };
    let callback = |link: pulldown_cmark::BrokenLink<'_>| {
        links
            .get(&label(&link.reference))
            .map(|(url, title)| (url.clone().into(), title.clone().into()))
    };
    let events =
        Parser::new_with_broken_link_callback(source, options(), Some(callback)).into_offset_iter();
    #[cfg(not(feature = "markdown-image"))]
    for (event, range) in events {
        builder.offset = range.start;
        builder.event(event);
    }
    #[cfg(feature = "markdown-image")]
    {
        // Images are numbered in source order, the order `images::requests` loads them.
        let events = events.collect::<Vec<_>>();
        let is_image = |event: &Event<'_>| matches!(event, Event::Start(Tag::Image { .. }));
        let (mut index, mut image) = (0, 0);
        while let Some((event, range)) = events.get(index) {
            builder.offset = range.start;
            if is_image(event) {
                let end = super::images::image_end(&events, index);
                if builder.image(image, super::images::alone(&events, index, end)) {
                    // A placed image replaces its alt text, including images nested in it.
                    image += events[index..=end]
                        .iter()
                        .filter(|(event, _)| is_image(event))
                        .count();
                    index = end + 1;
                    continue;
                }
                image += 1;
            }
            builder.event(event.clone());
            index += 1;
        }
    }
    builder.flush();
    let root = builder.stack.swap_remove(0);
    column(root.children, 1).into()
}

struct Builder<'a> {
    source: &'a str,
    offset: usize,
    number: Option<u64>,
    theme: &'a MarkdownTheme,
    #[cfg_attr(not(feature = "markdown-highlight"), allow(dead_code))]
    cache: &'a RefCell<Cache>,
    /// Open containers; the root is first.
    stack: Vec<Frame>,
    /// The paragraph, heading or table cell being filled.
    inline: Option<Inline>,
    /// Inline styles, innermost last.
    spans: Vec<Span>,
    /// Open links: destination and where their text starts in the inline.
    links: Vec<(String, usize)>,
    code: Option<Code>,
    html: Option<String>,
    table: Option<Table>,
    #[cfg(feature = "markdown-image")]
    images: &'a mut super::images::Images,
    /// Whether the paragraph's text resumes below a block image, which drops its leading spaces.
    #[cfg(feature = "markdown-image")]
    after_image: bool,
}

struct Frame {
    kind: Kind,
    children: Vec<Element>,
    gap: bool,
}

impl Frame {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            children: Vec::new(),
            gap: false,
        }
    }
}

enum Kind {
    Root,
    #[cfg(feature = "markdown-image")]
    Paragraph,
    Quote(Option<BlockQuoteKind>),
    List {
        next: Option<u64>,
        items: Vec<(String, Vec<Element>, bool)>,
    },
    Item {
        marker: String,
        task: Option<bool>,
        gap: bool,
    },
    Footnote(String),
}

#[derive(Default)]
struct Inline {
    text: Text,
    plain: String,
}

#[derive(Clone, Default)]
struct Span {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    dim: bool,
    color: Option<Color>,
    link: Option<String>,
}

struct Code {
    language: String,
    text: String,
}

struct Table {
    alignments: Vec<Alignment>,
    rows: Vec<Row>,
    cells: Vec<(Element, usize)>,
    head: bool,
}

struct Row {
    cells: Vec<(Element, usize)>,
    head: bool,
}

impl Builder<'_> {
    fn event(&mut self, event: Event<'_>) {
        if let Some(code) = &mut self.code {
            match event {
                Event::Text(text) => code.text.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let code = self.code.take().expect("open code block");
                    let block = self.code_block(code);
                    self.push(block);
                }
                _ => {}
            }
            return;
        }
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) | Event::InlineMath(text) | Event::DisplayMath(text) => {
                self.run(&text, Span::default());
            }
            Event::Code(text) => self.run(
                &text,
                Span {
                    color: Some(self.theme.code_color),
                    ..Span::default()
                },
            ),
            Event::Html(html) => match &mut self.html {
                Some(block) => block.push_str(&html),
                None => self.run(&html, dim()),
            },
            Event::InlineHtml(html) => self.run(&html, dim()),
            Event::FootnoteReference(label) => self.run(&format!("[^{label}]"), Span::default()),
            Event::SoftBreak => self.run(" ", Span::default()),
            Event::HardBreak => self.run("\n", Span::default()),
            Event::Rule => {
                self.flush();
                if let Some(frame) = self.stack.last_mut() {
                    frame.gap = blank_before(self.source, self.offset);
                }
                let rule = edge(View::new(), self.theme.rule_color).border_left(false);
                self.push(rule.border_bottom(false).border_right(false).into());
            }
            Event::TaskListMarker(done) => {
                if let Some(Kind::Item { task, .. }) = self
                    .stack
                    .iter_mut()
                    .rev()
                    .map(|frame| &mut frame.kind)
                    .find(|kind| matches!(kind, Kind::Item { .. }))
                {
                    *task = Some(done);
                }
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        if matches!(
            tag,
            Tag::Paragraph
                | Tag::Heading { .. }
                | Tag::List(_)
                | Tag::CodeBlock(_)
                | Tag::BlockQuote(_)
                | Tag::HtmlBlock
                | Tag::Table(_)
        ) {
            self.flush();
            if let Some(frame) = self.stack.last_mut() {
                frame.gap = blank_before(self.source, self.offset);
            }
        }
        match tag {
            Tag::Paragraph => {
                self.flush();
                #[cfg(feature = "markdown-image")]
                self.open(Kind::Paragraph);
            }
            Tag::Heading { level, .. } => {
                self.flush();
                self.spans.push(Span {
                    bold: true,
                    underline: level == HeadingLevel::H1,
                    color: (level <= HeadingLevel::H2).then_some(self.theme.heading_color),
                    ..Span::default()
                });
            }
            Tag::BlockQuote(kind) => self.open(Kind::Quote(kind)),
            Tag::CodeBlock(kind) => {
                self.flush();
                let language = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_owned()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some(Code {
                    language,
                    text: String::new(),
                });
            }
            Tag::HtmlBlock => {
                self.flush();
                self.html = Some(String::new());
            }
            Tag::List(start) => {
                let next = if self.stack.len() == 1 {
                    self.number.take().or(start)
                } else {
                    start
                };
                self.open(Kind::List {
                    next,
                    items: Vec::new(),
                });
            }
            Tag::Item => {
                let marker = self.next_marker();
                self.open(Kind::Item {
                    marker,
                    task: None,
                    gap: blank_before(self.source, self.offset),
                });
            }
            Tag::FootnoteDefinition(label) => self.open(Kind::Footnote(label.to_string())),
            Tag::Table(alignments) => {
                self.flush();
                self.table = Some(Table {
                    alignments,
                    rows: Vec::new(),
                    cells: Vec::new(),
                    head: false,
                });
            }
            Tag::TableHead => {
                if let Some(table) = &mut self.table {
                    table.head = true;
                }
                self.style(|span| span.bold = true);
            }
            Tag::Emphasis => self.style(|span| span.italic = true),
            Tag::Strong => self.style(|span| span.bold = true),
            Tag::Strikethrough => self.style(|span| span.strikethrough = true),
            Tag::Link { dest_url, .. } => {
                let start = self.inline.get_or_insert_default().plain.len();
                self.links.push((dest_url.to_string(), start));
                let color = self.theme.link_color;
                self.style(|span| {
                    span.underline = true;
                    span.color = Some(color);
                    span.link = Some(dest_url.to_string());
                });
            }
            Tag::Image { .. } => {
                self.style(|span| {
                    span.dim = true;
                    span.italic = true;
                });
                self.run("[", Span::default());
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush();
                // A tall inline image splits the paragraph into several blocks.
                #[cfg(feature = "markdown-image")]
                {
                    let mut children = self.close().children;
                    match children.len() {
                        0 => {}
                        1 => self.push(children.pop().expect("paragraph block")),
                        _ => self.push(column(children, 0).into()),
                    }
                }
            }
            TagEnd::Heading(_) => {
                self.flush();
                self.spans.pop();
            }
            TagEnd::BlockQuote(_) => {
                let Frame { kind, children, .. } = self.close();
                if let Kind::Quote(kind) = kind {
                    let quote = self.quote(kind, children);
                    self.push(quote);
                }
            }
            TagEnd::HtmlBlock => {
                if let Some(html) = self.html.take() {
                    self.push(Text::new(html.trim_end()).dim_color(true).into());
                }
            }
            TagEnd::List(_) => {
                let Frame { kind, .. } = self.close();
                if let Kind::List { items, .. } = kind {
                    let list = self.list(items);
                    self.push(list);
                }
            }
            TagEnd::Item => {
                let Frame { kind, children, .. } = self.close();
                let Kind::Item { marker, task, gap } = kind else {
                    return;
                };
                let marker = match task {
                    Some(true) => self.theme.task_done.clone(),
                    Some(false) => self.theme.task_open.clone(),
                    None => marker,
                };
                if let Some(Frame {
                    kind: Kind::List { items, .. },
                    ..
                }) = self.stack.last_mut()
                {
                    items.push((marker, children, gap));
                }
            }
            TagEnd::FootnoteDefinition => {
                let Frame { kind, children, .. } = self.close();
                if let Kind::Footnote(label) = kind {
                    let note =
                        View::new()
                            .child(View::new().flex_shrink(0.0).padding_right(1).child(
                                Text::new(format!("[^{label}]:")).color(self.theme.link_color),
                            ))
                            .child(column(children, 1).flex_grow(1.0).min_width(0));
                    self.push(note.into());
                }
            }
            TagEnd::TableCell => {
                let inline = self.inline.take().unwrap_or_default();
                if let Some(table) = &mut self.table {
                    table
                        .cells
                        .push((inline.text.into(), text_width(&inline.plain)));
                }
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(table) = &mut self.table {
                    let cells = std::mem::take(&mut table.cells);
                    table.rows.push(Row {
                        cells,
                        head: table.head,
                    });
                    table.head = false;
                }
                if tag == TagEnd::TableHead {
                    self.spans.pop();
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    let table = self.table_view(table);
                    self.push(table);
                }
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.spans.pop();
            }
            TagEnd::Link => {
                self.spans.pop();
                if let Some((url, start)) = self.links.pop() {
                    let text = self
                        .inline
                        .as_ref()
                        .and_then(|inline| inline.plain.get(start..))
                        .unwrap_or("");
                    if !self.theme.hyperlinks && !url.is_empty() && text != url {
                        self.run(&format!(" ({url})"), dim());
                    }
                }
            }
            TagEnd::Image => {
                self.run("]", Span::default());
                self.spans.pop();
            }
            _ => {}
        }
    }

    fn open(&mut self, kind: Kind) {
        self.flush();
        self.stack.push(Frame::new(kind));
    }

    fn close(&mut self) -> Frame {
        self.flush();
        self.stack.pop().expect("open container")
    }

    fn push(&mut self, element: Element) {
        if let Some(frame) = self.stack.last_mut() {
            let element = if matches!(frame.kind, Kind::Item { .. })
                && !frame.children.is_empty()
                && frame.gap
            {
                View::new()
                    .flex_direction(FlexDirection::Column)
                    .padding_top(1)
                    .child(element)
                    .into()
            } else {
                element
            };
            frame.gap = false;
            frame.children.push(element);
        }
    }

    /// Moves the open inline text into its container.
    fn flush(&mut self) {
        if let Some(inline) = self.inline.take() {
            self.push(inline.text.into());
        }
        #[cfg(feature = "markdown-image")]
        {
            for (_, start) in &mut self.links {
                *start = 0;
            }
            self.after_image = false;
        }
    }

    #[cfg(feature = "markdown-image")]
    fn image(&mut self, index: usize, alone: bool) -> bool {
        if self.table.is_some() {
            return false;
        }
        let indent: usize = self
            .stack
            .iter()
            .map(|frame| match &frame.kind {
                Kind::Item { marker, task, .. } => {
                    let marker = match task {
                        Some(true) => &self.theme.task_done,
                        Some(false) => &self.theme.task_open,
                        None => marker,
                    };
                    text_width(marker) + 1
                }
                Kind::Quote(_) => 2,
                Kind::Footnote(label) => text_width(label) + 5,
                _ => 0,
            })
            .sum();
        let Some((placement, inline)) = self.images.place(index, indent, alone) else {
            return false;
        };
        if inline {
            let url = self
                .spans
                .last()
                .and_then(|span| span.link.as_deref())
                .filter(|url| self.theme.hyperlinks && !url.is_empty());
            let text =
                Text::default().children(crate::image::linked_placeholder_rows(&placement, url));
            let inline = self.inline.get_or_insert_default();
            inline.plain.push_str(&" ".repeat(placement.size.columns));
            inline.text = std::mem::take(&mut inline.text).child(text);
            self.after_image = false;
        } else {
            if self
                .inline
                .as_ref()
                .is_some_and(|inline| inline.plain.is_empty())
            {
                self.inline = None;
            }
            self.flush();
            self.push(
                column(crate::image::placeholder_rows(&placement), 0)
                    .flex_shrink(0.0)
                    .into(),
            );
            self.after_image = true;
        }
        true
    }

    fn style(&mut self, change: impl FnOnce(&mut Span)) {
        let mut span = self.spans.last().cloned().unwrap_or_default();
        change(&mut span);
        self.spans.push(span);
    }

    /// Appends text in the current style, with `extra` applied on top.
    fn run(&mut self, content: &str, extra: Span) {
        #[cfg(feature = "markdown-image")]
        let content = if self.after_image {
            let content = content.trim_start();
            if content.is_empty() {
                return;
            }
            self.after_image = false;
            content
        } else {
            content
        };
        let span = self.spans.last().cloned().unwrap_or_default();
        let inline = self.inline.get_or_insert_default();
        inline.plain.push_str(content);
        let piece = match span.link.as_deref() {
            Some(url) if self.theme.hyperlinks && !url.is_empty() => {
                format!("\x1b]8;;{url}\x1b\\{content}\x1b]8;;\x1b\\")
            }
            _ => content.to_owned(),
        };
        let piece = Text::new(piece)
            .bold(span.bold || extra.bold)
            .italic(span.italic || extra.italic)
            .underline(span.underline || extra.underline)
            .strikethrough(span.strikethrough || extra.strikethrough)
            .dim_color(span.dim || extra.dim)
            .color(extra.color.or(span.color));
        inline.text = std::mem::take(&mut inline.text).child(piece);
    }

    fn next_marker(&mut self) -> String {
        let depth = self
            .stack
            .iter()
            .filter(|frame| matches!(frame.kind, Kind::List { .. }))
            .count()
            .saturating_sub(1);
        if let Some(Frame {
            kind: Kind::List {
                next: Some(number), ..
            },
            ..
        }) = self.stack.last_mut()
        {
            let marker = format!("{number}.");
            *number += 1;
            return marker;
        }
        let bullets = &self.theme.bullets;
        bullets
            .get(depth % bullets.len().max(1))
            .cloned()
            .unwrap_or_else(|| "-".into())
    }

    fn list(&self, items: Vec<(String, Vec<Element>, bool)>) -> Element {
        column(
            items
                .into_iter()
                .enumerate()
                .map(|(index, (marker, children, gap))| {
                    let width = text_width(&marker) + 1;
                    View::new()
                        .padding_top(u16::from(index > 0 && gap))
                        .child(
                            View::new()
                                .width(cells(width))
                                .flex_shrink(0.0)
                                .child(Text::new(marker).color(self.theme.bullet_color)),
                        )
                        .child(column(children, 0).flex_grow(1.0).min_width(0))
                }),
            0,
        )
        .into()
    }

    fn quote(&self, kind: Option<BlockQuoteKind>, children: Vec<Element>) -> Element {
        let theme = self.theme;
        let (title, color) = match kind {
            None => ("", theme.quote_color),
            Some(BlockQuoteKind::Note) => ("ℹ Note", theme.note_color),
            Some(BlockQuoteKind::Tip) => ("✔ Tip", theme.tip_color),
            Some(BlockQuoteKind::Important) => ("! Important", theme.important_color),
            Some(BlockQuoteKind::Warning) => ("⚠ Warning", theme.warning_color),
            Some(BlockQuoteKind::Caution) => ("✘ Caution", theme.caution_color),
        };
        let body = column(children, 1);
        let body = if title.is_empty() {
            body
        } else {
            View::new()
                .flex_direction(FlexDirection::Column)
                .child(Text::new(title).bold(true).color(color))
                .child(body)
        };
        edge(body, color)
            .border_top(false)
            .border_right(false)
            .border_bottom(false)
            .padding_left(1)
            .into()
    }

    #[cfg(feature = "markdown-highlight")]
    fn highlight(&self, code: &Code) -> Option<Vec<Text>> {
        crate::highlight::highlight(
            &mut self.cache.borrow_mut(),
            &crate::highlight::Lang::Token(code.language.clone()),
            &code.text,
            &self.theme.code_theme,
        )
    }

    #[cfg(not(feature = "markdown-highlight"))]
    fn highlight(&self, _: &Code) -> Option<Vec<Text>> {
        None
    }

    fn code_block(&self, code: Code) -> Element {
        let lines = self.highlight(&code).unwrap_or_else(|| {
            code.text
                .lines()
                .map(|line| {
                    Text::new(if line.is_empty() { " " } else { line })
                        .color(self.theme.code_block_color)
                })
                .collect()
        });
        let mut block = View::new().flex_direction(FlexDirection::Column);
        if !code.language.is_empty() {
            block = block.child(Text::new(code.language).dim_color(true));
        }
        block.child(column(lines, 0).padding_left(2)).into()
    }

    /// Columns take their widest cell and shrink in proportion, so the
    /// layout keeps them aligned from row to row.
    fn table_view(&self, table: Table) -> Element {
        let count = table
            .rows
            .iter()
            .map(|row| row.cells.len())
            .max()
            .unwrap_or(0);
        let mut widths = vec![1; count];
        for row in &table.rows {
            for (index, (_, width)) in row.cells.iter().enumerate() {
                widths[index] = widths[index].max(*width);
            }
        }
        let color = self.theme.rule_color;
        let cell = |index: usize| {
            let view = View::new()
                .flex_basis(cells(widths[index] + 2 + usize::from(index > 0)))
                .flex_shrink(1.0)
                .min_width(0);
            if index == 0 {
                view
            } else {
                edge(view, color)
                    .border_top(false)
                    .border_right(false)
                    .border_bottom(false)
            }
        };
        let mut rows: Vec<Element> = Vec::new();
        for row in table.rows {
            let head = row.head;
            let mut contents = row.cells.into_iter();
            rows.push(
                View::new()
                    .children((0..count).map(|index| {
                        let content = contents
                            .next()
                            .map_or_else(|| Text::new(" ").into(), |(element, _)| element);
                        let justify = match table.alignments.get(index) {
                            Some(Alignment::Right) => JustifyContent::FlexEnd,
                            Some(Alignment::Center) => JustifyContent::Center,
                            _ => JustifyContent::FlexStart,
                        };
                        cell(index)
                            .padding_x(1)
                            .justify_content(justify)
                            .child(content)
                    }))
                    .into(),
            );
            if head {
                rows.push(
                    View::new()
                        .children((0..count).map(|index| {
                            let joint = if index == 0 { "" } else { "┼" };
                            View::new()
                                .flex_basis(cells(widths[index] + 2 + usize::from(index > 0)))
                                .flex_shrink(1.0)
                                .min_width(0)
                                .child(
                                    Text::new(format!("{joint}{}", "─".repeat(widths[index] + 2)))
                                        .wrap(TextWrap::TruncateEnd)
                                        .color(color),
                                )
                        }))
                        .into(),
                );
            }
        }
        column(rows, 0).into()
    }
}

fn column(children: impl IntoIterator<Item = impl Into<Element>>, gap: u16) -> View {
    View::new()
        .flex_direction(FlexDirection::Column)
        .gap(gap)
        .children(children)
}

fn edge(view: View, color: Color) -> View {
    view.border_style(BorderStyle::Single).border_color(color)
}

fn dim() -> Span {
    Span {
        dim: true,
        ..Span::default()
    }
}

fn cells(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

/// A source blank row before the line containing `offset`.
pub(crate) fn blank_before(source: &str, offset: usize) -> bool {
    let prefix = &source[..offset];
    let Some(line) = prefix.rfind('\n') else {
        return false;
    };
    let previous = &prefix[..line];
    let previous = previous.rsplit('\n').next().unwrap_or("");
    previous.trim().trim_matches('>').trim().is_empty()
}
