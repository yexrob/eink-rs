use std::rc::Rc;

use ink_rs::{
    Callback, Color, ColorLevel, Element, FlexDirection, Props, Scope, Styled, Text, TextWrap, View,
};

use super::diff::{self, DiffError, Kind, Row};
use super::{Palette, use_theme};

/// The representation of a code source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CodeFormat {
    /// Source text with an optional line-number gutter.
    #[default]
    Source,
    /// Unified diff hunks with old and new line numbers.
    Diff,
}

/// How code fits the available width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CodeWrap {
    /// Breaks lines at the available column width, preserving indentation.
    #[default]
    Wrap,
    /// Truncates each line with an ellipsis at the end.
    TruncateEnd,
}

/// Code with an optional line-number gutter or unified diff formatting.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = code)]
pub struct Code {
    /// The source text or unified diff.
    #[props(into)]
    pub source: Rc<str>,
    /// The language token used for syntax highlighting.
    #[props(into)]
    pub language: Option<String>,
    /// Gives the language from the file name when `language` is `None`.
    #[props(into)]
    pub path: Option<String>,
    /// Draws a line-number gutter starting at this number. Diffs number from their hunk headers.
    pub start_line: Option<usize>,
    /// Selects source text or unified diff formatting.
    pub format: CodeFormat,
    /// Selects line wrapping or end truncation.
    pub wrap: CodeWrap,
    /// Called once per source when a diff does not parse and is drawn as plain code.
    pub on_fallback: Option<Callback<DiffError>>,
}

/// Colors and highlighting theme for [`Code`].
#[derive(Clone, Debug, PartialEq)]
pub struct CodeTheme {
    /// Line numbers, gray by default.
    pub gutter_color: Color,
    /// Added lines and markers, from [`Palette::success`].
    pub added: Color,
    /// Removed lines and markers, from [`Palette::error`].
    pub removed: Color,
    /// Added row tint, dark green by default.
    pub added_background: Option<Color>,
    /// Removed row tint, dark red by default.
    pub removed_background: Option<Color>,
    /// Unhighlighted source text, cyan by default.
    pub code_color: Color,
    /// A bat theme name; `ansi` by default.
    pub theme: String,
}

impl From<&Palette> for CodeTheme {
    fn from(palette: &Palette) -> Self {
        Self {
            gutter_color: Color::Gray,
            added: palette.success,
            removed: palette.error,
            added_background: Some(Color::Rgb(24, 48, 30)),
            removed_background: Some(Color::Rgb(56, 24, 24)),
            code_color: Color::Cyan,
            theme: "ansi".into(),
        }
    }
}

struct Document {
    rows: Vec<Row>,
    error: Option<DiffError>,
    diff: bool,
}

fn document(props: &Code) -> Document {
    let error = if props.format == CodeFormat::Diff {
        match diff::parse(&props.source) {
            Ok(rows) => {
                return Document {
                    rows,
                    error: None,
                    diff: true,
                };
            }
            Err(error) => Some(error),
        }
    } else {
        None
    };
    Document {
        rows: props
            .source
            .lines()
            .map(|text| Row {
                text: text.into(),
                old: None,
                new: None,
                kind: Kind::Source,
            })
            .collect(),
        error,
        diff: false,
    }
}

fn code(cx: &mut Scope, props: &Code) -> Element {
    let theme = use_theme::<CodeTheme>(cx);
    let level = cx.use_color_level();
    let doc = cx.use_memo((props.source.clone(), props.format), || {
        Rc::new(document(props))
    });
    let callback = props.on_fallback.clone();
    let error = doc.error.clone();
    cx.use_effect(props.source.clone(), move || {
        if let (Some(callback), Some(error)) = (callback, error) {
            callback.call(error);
        }
    });
    #[cfg(feature = "highlight")]
    let texts = highlighted(cx, props, &theme, &doc);
    #[cfg(not(feature = "highlight"))]
    let texts: Vec<Option<Text>> = Vec::new();
    let digits = |number: usize| number.to_string().len();
    let old_width = doc
        .rows
        .iter()
        .filter_map(|row| row.old)
        .max()
        .map_or(1, digits);
    let new_width = if doc.diff {
        doc.rows
            .iter()
            .filter_map(|row| row.new)
            .max()
            .map_or(1, digits)
    } else {
        props.start_line.map_or(0, |start| {
            digits(start.saturating_add(doc.rows.len().saturating_sub(1)))
        })
    };
    let wrap = match props.wrap {
        CodeWrap::Wrap => TextWrap::Hard,
        CodeWrap::TruncateEnd => TextWrap::TruncateEnd,
    };
    let mut view = View::new()
        .flex_direction(FlexDirection::Column)
        .min_width(0);
    for (index, row) in doc.rows.iter().enumerate() {
        let (marker, color, background) = match row.kind {
            Kind::Added => ("+", theme.added, theme.added_background),
            Kind::Removed => ("-", theme.removed, theme.removed_background),
            _ => (" ", theme.code_color, None),
        };
        let dim = matches!(
            row.kind,
            Kind::Context | Kind::Header | Kind::Hunk | Kind::Notice
        );
        let number = |value: Option<usize>, width| {
            format!(
                "{:>width$}",
                value.map(|n| n.to_string()).unwrap_or_default()
            )
        };
        let gutter = if doc.diff {
            format!(
                "{} {} ",
                number(row.old, old_width),
                number(row.new, new_width)
            )
        } else if let Some(start) = props.start_line {
            format!("{} ", number(Some(start.saturating_add(index)), new_width))
        } else {
            String::new()
        };
        let mut line = View::new().min_width(0);
        if level >= ColorLevel::Ansi256 {
            line = line.background_color(background);
        }
        if !gutter.is_empty() {
            line = line.child(
                View::new()
                    .width(u16::try_from(gutter.len()).unwrap_or(u16::MAX))
                    .flex_shrink(0.0)
                    .child(Text::new(gutter).color(theme.gutter_color)),
            );
        }
        if doc.diff {
            line = line.child(
                View::new()
                    .width(2)
                    .flex_shrink(0.0)
                    .child(Text::new(format!("{marker} ")).color(color)),
            );
        }
        let text = texts
            .get(index)
            .cloned()
            .flatten()
            .unwrap_or_else(|| Text::new(if row.text.is_empty() { " " } else { &row.text }));
        line = line.child(
            View::new()
                .flex_grow(1.0)
                .min_width(0)
                .child(Text::new(text).color(color).dim_color(dim).wrap(wrap)),
        );
        view = view.child(line);
    }
    view.into()
}

#[cfg(feature = "highlight")]
struct Block {
    source: String,
    rows: Vec<Option<usize>>,
}

#[cfg(feature = "highlight")]
fn rebuild(rows: &[Row]) -> Vec<Block> {
    let empty = || Block {
        source: String::new(),
        rows: Vec::new(),
    };
    let mut blocks = Vec::new();
    let (mut old, mut new) = (empty(), empty());
    for (index, row) in rows.iter().enumerate() {
        if row.kind == Kind::Hunk {
            blocks.push(std::mem::replace(&mut old, empty()));
            blocks.push(std::mem::replace(&mut new, empty()));
        }
        for (block, include, mapped) in [
            // Context advances both parsers but is painted from the new side.
            (
                &mut old,
                matches!(row.kind, Kind::Removed | Kind::Context),
                row.kind == Kind::Removed,
            ),
            (
                &mut new,
                matches!(row.kind, Kind::Added | Kind::Context | Kind::Source),
                true,
            ),
        ] {
            if include {
                block.source.push_str(&row.text);
                block.source.push('\n');
                block.rows.push(mapped.then_some(index));
            }
        }
    }
    blocks.extend([old, new]);
    blocks.retain(|block| !block.rows.is_empty());
    blocks
}

#[cfg(feature = "highlight")]
fn paint_blocks(
    blocks: &[Block],
    language: &crate::highlight::Lang,
    theme: &str,
    count: usize,
) -> Vec<Option<Text>> {
    let mut texts = vec![None; count];
    for block in blocks {
        if let Some(lines) = crate::highlight::highlight(
            &mut crate::highlight::Cache::default(),
            language,
            &block.source,
            theme,
        ) {
            for (row, text) in block.rows.iter().zip(lines) {
                if let Some(row) = row {
                    texts[*row] = Some(text);
                }
            }
        }
    }
    texts
}

#[cfg(feature = "highlight")]
fn highlighted(
    cx: &mut Scope,
    props: &Code,
    theme: &CodeTheme,
    doc: &Document,
) -> Vec<Option<Text>> {
    use crate::highlight::{self, Lang, Work};
    use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};

    let language = props
        .language
        .clone()
        .map(Lang::Token)
        .or_else(|| props.path.clone().map(Lang::File));
    let key = (
        props.source.clone(),
        props.format,
        language.clone(),
        theme.theme.clone(),
    );
    let alive = cx.use_memo(key.clone(), || Arc::new(()));
    let id = cx.use_memo(key.clone(), highlight::block_id);
    let blocks = cx.use_memo((props.source.clone(), props.format), || {
        Rc::new(rebuild(&doc.rows))
    });
    let count = doc.rows.len();
    let sync = count <= 64;
    let texts = cx.use_memo(key, || {
        if sync && doc.error.is_none() {
            language
                .as_ref()
                .map(|language| paint_blocks(&blocks, language, &theme.theme, count))
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    });
    let results = cx.use_state(|| (id, Vec::<Option<Text>>::new()));
    let app = cx.use_app();
    let theme = theme.theme.clone();
    let fallback = doc.error.is_some();
    cx.use_effect(id, move || {
        if sync || fallback {
            return;
        }
        let Some(language) = language else {
            return;
        };
        results.set((id, vec![None; count]));
        let (tx, rx) = mpsc::channel::<highlight::ResultLine>();
        let rx = Arc::new(Mutex::new(rx));
        let queued = Arc::new(AtomicBool::new(false));
        let weak = Arc::downgrade(&alive);
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            use std::sync::atomic::Ordering;
            if queued.swap(true, Ordering::AcqRel) {
                return;
            }
            let rx = rx.clone();
            let queued = queued.clone();
            let weak = weak.clone();
            app.update(move || {
                queued.store(false, Ordering::Release);
                // Source changes and unmount invalidate queued UI callbacks too.
                if weak.strong_count() == 0 {
                    return;
                }
                results.update(|(current, texts)| {
                    if *current != id {
                        return;
                    }
                    for result in rx.lock().expect("code highlights").try_iter() {
                        if let Some(text) = texts.get_mut(result.id as usize) {
                            *text = result.spans.as_deref().map(highlight::text);
                        }
                    }
                });
            });
        });
        for block in blocks.iter() {
            let block_id = highlight::block_id();
            let unknown = Arc::new(AtomicBool::new(false));
            for (source, row) in block.source.split_inclusive('\n').zip(&block.rows) {
                highlight::submit(Work::Line {
                    block: block_id,
                    id: row.map_or(u64::MAX, |row| row as u64),
                    language: language.clone(),
                    theme: theme.clone(),
                    source: source.into(),
                    reply: tx.clone(),
                    wake: Some(wake.clone()),
                    unknown: unknown.clone(),
                    stream: Arc::downgrade(&alive),
                });
            }
            highlight::submit(Work::Close(block_id));
        }
    });
    if sync {
        texts
    } else {
        let (current, texts) = results.get();
        if current == id { texts } else { Vec::new() }
    }
}

#[cfg(all(test, feature = "highlight"))]
mod tests {
    use super::*;
    use crate::highlight::{self, Cache, Lang};

    #[test]
    fn hunk_rebuild_keeps_comment_state_on_both_sides() {
        let rows = diff::parse("@@ -1,3 +1,3 @@\n /* comment\n-old comment\n+new comment\n */\n@@ -20 +30 @@\n-let old = 1;\n+let new = 2;\n").unwrap();
        let blocks = rebuild(&rows);
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].source, "/* comment\nold comment\n*/\n");
        assert_eq!(blocks[1].source, "/* comment\nnew comment\n*/\n");
        assert_eq!(blocks[0].rows, [None, Some(2), None]);
        assert_eq!(blocks[1].rows, [Some(1), Some(3), Some(4)]);
        let language = Lang::Token("rust".into());
        let texts = paint_blocks(&blocks, &language, "ansi", rows.len());
        let expected = highlight::highlight(
            &mut Cache::default(),
            &language,
            "/* comment\nold comment\nnew comment\n*/\n",
            "ansi",
        )
        .unwrap();
        assert_eq!(texts[2], Some(expected[1].clone()));
        assert_eq!(texts[3], Some(expected[2].clone()));
        let isolated =
            highlight::highlight(&mut Cache::default(), &language, "old comment\n", "ansi")
                .unwrap();
        assert_ne!(texts[2], Some(isolated[0].clone()));
    }
}
