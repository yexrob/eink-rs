//! Controlled multiline editing with byte offsets and grapheme wrapping.

use std::{cell::RefCell, rc::Rc};

use ink_rs::{
    Callback, CursorPosition, Element, FlexDirection, KeyEventType, Percent, Props, Scope, Styled,
    Text, TextWrap, View, text_width,
};
use unicode_segmentation::UnicodeSegmentation;

use super::{InputTheme, input::grapheme_offsets, use_theme};

/// Text and a byte offset on an extended grapheme boundary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComposerState {
    /// The text owned by the host.
    pub value: String,
    /// The byte offset of the caret, including the end of the value.
    pub cursor: usize,
}

impl ComposerState {
    /// Creates a value with its cursor at the end.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self { value, cursor }
    }
}

/// An edit proposed by the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposerAction {
    /// Inserts text at the caret.
    Insert(String),
    /// Inserts a logical line break.
    Newline,
    /// Deletes the preceding grapheme.
    Backspace,
    /// Deletes the following grapheme.
    Delete,
    /// Moves to the preceding grapheme.
    Left,
    /// Moves to the following grapheme.
    Right,
    /// Moves to the preceding UAX #29 word start.
    WordLeft,
    /// Moves to the following UAX #29 word end.
    WordRight,
    /// Moves to the logical line start.
    LineStart,
    /// Moves to the logical line end.
    LineEnd,
    /// Moves to the preceding visual row.
    Up,
    /// Moves to the following visual row.
    Down,
    /// Kills text through the preceding word start.
    DeleteWord,
    /// Kills text through the logical line start.
    KillToStart,
    /// Kills text through the logical line end, or the newline at that end.
    KillToEnd,
    /// Inserts the kill buffer.
    Yank,
}

/// A proposal that the host may accept, rewrite or ignore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// The action that produced the proposal.
    pub action: ComposerAction,
    /// The state before the action.
    pub before: ComposerState,
    /// The proposed state.
    pub after: ComposerState,
}

/// A visual movement or history request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Moves toward the start.
    Up,
    /// Moves toward the end.
    Down,
}

/// The result of a headless edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reduced {
    /// The proposed text and caret.
    pub state: ComposerState,
    /// Replaces the kill buffer when text was killed.
    pub kill: Option<String>,
    /// Requests history at the first or last visual row.
    pub boundary: Option<Direction>,
}

/// A zero-based position in terminal cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Caret {
    /// The visual row.
    pub row: usize,
    /// The terminal column.
    pub column: usize,
}

struct Line {
    start: usize,
    end: usize,
}

fn width(grapheme: &str) -> usize {
    if grapheme == "\t" {
        4
    } else {
        text_width(grapheme)
    }
}

fn lines(value: &str, columns: usize) -> Vec<Line> {
    let columns = columns.max(1);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut column = 0;
    for (offset, grapheme) in value.grapheme_indices(true) {
        if grapheme == "\n" || grapheme == "\r\n" {
            rows.push(Line { start, end: offset });
            start = offset + grapheme.len();
            column = 0;
        } else {
            let cells = width(grapheme);
            if column + cells > columns && offset > start {
                rows.push(Line { start, end: offset });
                start = offset;
                column = 0;
            }
            column += cells;
        }
    }
    rows.push(Line {
        start,
        end: value.len(),
    });
    rows
}

fn cursor_boundary(value: &str, cursor: usize) -> usize {
    grapheme_offsets(value)
        .into_iter()
        .take_while(|offset| *offset <= cursor)
        .last()
        .unwrap_or(0)
}

fn caret(value: &str, cursor: usize, rows: &[Line]) -> Caret {
    let row = rows
        .iter()
        .rposition(|line| line.start <= cursor)
        .unwrap_or(0);
    let column = value[rows[row].start..cursor.min(rows[row].end)]
        .graphemes(true)
        .map(width)
        .sum();
    Caret { row, column }
}

/// Returns the visual row count and caret at the given text width.
///
/// Tabs occupy four columns. Width zero is treated as one. A cursor inside a
/// grapheme is clamped to its start; a cursor past the text is clamped to its end.
pub fn composer_caret(state: &ComposerState, columns: usize) -> (usize, Caret) {
    let rows = lines(&state.value, columns);
    (
        rows.len(),
        caret(
            &state.value,
            cursor_boundary(&state.value, state.cursor),
            &rows,
        ),
    )
}

fn word_left(value: &str, cursor: usize) -> usize {
    value
        .unicode_word_indices()
        .map(|(offset, _)| offset)
        .take_while(|offset| *offset < cursor)
        .last()
        .unwrap_or(0)
}

fn word_right(value: &str, cursor: usize) -> usize {
    value
        .unicode_word_indices()
        .map(|(offset, word)| offset + word.len())
        .find(|offset| *offset > cursor)
        .unwrap_or(value.len())
}

/// Proposes an edit without changing the supplied state or kill buffer.
///
/// Up and Down use grapheme widths and retain the column where the target row
/// permits it. Words follow UAX #29 and skip punctuation and whitespace.
/// Invalid cursors are clamped as in [`composer_caret`]. Insertions and deletions
/// place the caret on the first grapheme boundary at or after the edit.
pub fn reduce_composer(
    state: &ComposerState,
    action: &ComposerAction,
    columns: usize,
    kill: &str,
) -> Reduced {
    let mut next = state.clone();
    let offsets = grapheme_offsets(&next.value);
    let index = offsets.partition_point(|offset| *offset <= next.cursor) - 1;
    next.cursor = offsets[index];
    let cursor = next.cursor;
    let start = next.value[..cursor].rfind('\n').map_or(0, |at| at + 1);
    let end = next.value[cursor..]
        .find('\n')
        .map_or(next.value.len(), |at| cursor + at);
    let end = cursor_boundary(&next.value, end);
    let mut killed = None;
    let mut boundary = None;
    let mut replace = None;
    use ComposerAction::*;
    match action {
        Insert(text) => replace = Some((cursor..cursor, text.as_str())),
        Newline => replace = Some((cursor..cursor, "\n")),
        Yank => replace = Some((cursor..cursor, kill)),
        Left => next.cursor = offsets[index.saturating_sub(1)],
        Right => next.cursor = offsets[(index + 1).min(offsets.len() - 1)],
        WordLeft => next.cursor = word_left(&next.value, cursor),
        WordRight => next.cursor = word_right(&next.value, cursor),
        LineStart => next.cursor = start,
        LineEnd => next.cursor = end,
        Backspace => replace = Some((offsets[index.saturating_sub(1)]..cursor, "")),
        Delete => replace = Some((cursor..offsets[(index + 1).min(offsets.len() - 1)], "")),
        DeleteWord => replace = Some((word_left(&next.value, cursor)..cursor, "")),
        KillToStart => replace = Some((start..cursor, "")),
        KillToEnd => {
            let end = if cursor == end && end < next.value.len() {
                offsets[index + 1]
            } else {
                end
            };
            replace = Some((cursor..end, ""));
        }
        Up | Down => {
            let rows = lines(&next.value, columns);
            let position = caret(&next.value, cursor, &rows);
            let target = if *action == Up {
                position.row.checked_sub(1)
            } else {
                (position.row + 1 < rows.len()).then_some(position.row + 1)
            };
            if let Some(target) = target {
                let line = &rows[target];
                let mut column = 0;
                next.cursor = line.start;
                for (offset, grapheme) in next.value[line.start..line.end].grapheme_indices(true) {
                    if column + width(grapheme) > position.column {
                        break;
                    }
                    column += width(grapheme);
                    let end = line.start + offset + grapheme.len();
                    // A soft-wrap boundary belongs to the following row.
                    if rows.get(target + 1).is_none_or(|row| row.start != end) {
                        next.cursor = end;
                    }
                }
            } else {
                boundary = Some(if *action == Up {
                    Direction::Up
                } else {
                    Direction::Down
                });
            }
        }
    }
    if let Some((range, text)) = replace {
        if matches!(action, DeleteWord | KillToStart | KillToEnd) && !range.is_empty() {
            killed = Some(next.value[range.clone()].to_string());
        }
        let end = range.start + text.len();
        next.value.replace_range(range, text);
        next.cursor = grapheme_offsets(&next.value)
            .into_iter()
            .find(|offset| *offset >= end)
            .unwrap_or(next.value.len());
    }
    Reduced {
        state: next,
        kill: killed,
        boundary,
    }
}

/// A controlled prompt box with visual wrapping and an internal scroll window.
///
/// The host accepts or rewrites [`Edit::after`] through `value` and `cursor`.
/// Ignoring an edit leaves the displayed props unchanged. Events in one input
/// batch build on the preceding proposal; the next render restores the props.
///
/// The host owns history and its index through `on_history`. Completion hosts
/// intercept Up and Down in `on_edit` by action and handle boundary requests in
/// `on_history`; `on_caret` positions their popup. Tab belongs to the host's
/// input hook. The host also owns double-Escape behavior and focus arbitration
/// through `is_disabled`. This component does not join the focus ring.
///
/// Enter submits, except that a preceding backslash becomes a newline.
/// Shift+Enter, Alt+Enter and Ctrl+J insert newlines. Tabs display as four cells.
/// Prompt text occupies one line. A grapheme wider than the available box is
/// clipped without splitting it. Set the app's `exit_on_ctrl_c` to false to
/// handle Ctrl+C through `on_interrupt`.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = composer)]
pub struct Composer {
    /// The host's text.
    #[props(into)]
    pub value: String,
    /// The caret's byte offset on a grapheme boundary.
    pub cursor: usize,
    /// Text shown when the value is empty.
    #[props(into)]
    pub placeholder: String,
    /// Text before the first row; continuation rows indent by its width.
    #[props(into)]
    pub prompt: String,
    /// Disables input and hides the caret.
    pub is_disabled: bool,
    /// The maximum visible rows. Zero allows growth without a row limit.
    pub max_rows: usize,
    /// Receives changed text or cursor proposals.
    pub on_edit: Option<Callback<Edit>>,
    /// Receives the text when Enter submits.
    pub on_submit: Option<Callback<String>>,
    /// Receives Up or Down at the first or last visual row.
    pub on_history: Option<Callback<Direction>>,
    /// Receives Escape.
    pub on_escape: Option<Callback>,
    /// Receives Ctrl+C presses and repeats, excluding releases.
    pub on_interrupt: Option<Callback>,
    /// Receives the caret in visible box coordinates, including the prompt.
    pub on_caret: Option<Callback<Caret>>,
}

fn composer(cx: &mut Scope, props: &Composer) -> Element {
    let theme = use_theme::<InputTheme>(cx);
    let node = cx.use_node_ref();
    let metrics = cx.use_box_metrics(node);
    let window = cx.use_window_size();
    let box_width = if metrics.has_measured {
        metrics.width as usize
    } else {
        usize::from(window.columns)
    };
    let prompt_width = text_width(&props.prompt);
    let columns = box_width.saturating_sub(prompt_width + 1).max(1);
    let state = ComposerState {
        value: props.value.clone(),
        cursor: cursor_boundary(&props.value, props.cursor),
    };
    let optimistic = cx.use_memo((), || Rc::new(RefCell::new(ComposerState::default())));
    *optimistic.borrow_mut() = state.clone();
    let kill = cx.use_memo((), || Rc::new(RefCell::new(String::new())));
    let revision = cx.use_state(|| 0usize);
    let on_edit = props.on_edit.clone();
    let on_history = props.on_history.clone();
    let editing = optimistic.clone();
    let dispatch = Callback::new(move |(action, escape): (ComposerAction, bool)| {
        let before = editing.borrow().clone();
        let base = if escape {
            reduce_composer(&before, &ComposerAction::Backspace, columns, "").state
        } else {
            before.clone()
        };
        let reduced = reduce_composer(&base, &action, columns, &kill.borrow());
        if let Some(text) = reduced.kill {
            *kill.borrow_mut() = text;
        }
        if let Some(direction) = reduced.boundary
            && let Some(callback) = &on_history
        {
            callback.call(direction);
        }
        if reduced.state != before {
            *editing.borrow_mut() = reduced.state.clone();
            revision.update(|value| *value = value.wrapping_add(1));
            if let Some(callback) = &on_edit {
                callback.call(Edit {
                    action,
                    before,
                    after: reduced.state,
                });
            }
        }
    });
    let paste = dispatch.clone();
    let on_submit = props.on_submit.clone();
    let on_escape = props.on_escape.clone();
    let on_interrupt = props.on_interrupt.clone();
    cx.use_input(!props.is_disabled, move |input, key| {
        if key.event_type == Some(KeyEventType::Release) {
            return;
        }
        use ComposerAction::*;
        let action = if key.ctrl && input == "c" {
            if let Some(callback) = &on_interrupt {
                callback.call(());
            }
            return;
        } else if key.escape {
            if let Some(callback) = &on_escape {
                callback.call(());
            }
            return;
        } else if key.enter {
            if key.shift || key.meta {
                Newline
            } else {
                let state = optimistic.borrow().clone();
                if state.value[..state.cursor].ends_with('\\') {
                    dispatch.call((Newline, true));
                } else if let Some(callback) = &on_submit {
                    callback.call(state.value);
                }
                return;
            }
        } else if input == "\n" || (key.ctrl && input == "j") {
            Newline
        } else if (key.meta && input == "b") || (key.ctrl && key.left_arrow) {
            WordLeft
        } else if (key.meta && input == "f") || (key.ctrl && key.right_arrow) {
            WordRight
        } else if key.home || (key.ctrl && input == "a") {
            LineStart
        } else if key.end || (key.ctrl && input == "e") {
            LineEnd
        } else if key.left_arrow {
            Left
        } else if key.right_arrow {
            Right
        } else if key.up_arrow {
            Up
        } else if key.down_arrow {
            Down
        } else if key.backspace {
            Backspace
        } else if key.delete {
            Delete
        } else if key.ctrl && input == "w" {
            DeleteWord
        } else if key.ctrl && input == "u" {
            KillToStart
        } else if key.ctrl && input == "k" {
            KillToEnd
        } else if key.ctrl && input == "y" {
            Yank
        } else if !key.ctrl
            && !key.meta
            && !key.super_key
            && !key.hyper
            && !key.tab
            && !input.is_empty()
        {
            Insert(input.into())
        } else {
            return;
        };
        dispatch.call((action, false));
    });
    cx.use_paste(!props.is_disabled, move |text| {
        paste.call((
            ComposerAction::Insert(text.replace("\r\n", "\n").replace('\r', "\n")),
            false,
        ));
    });

    let placeholder = state.value.is_empty();
    let value = if placeholder {
        props.placeholder.as_str()
    } else {
        &state.value
    };
    let rows = lines(value, columns);
    let position = caret(value, state.cursor, &rows);
    let visible = if props.max_rows == 0 {
        rows.len()
    } else {
        props.max_rows.min(rows.len())
    };
    let first = cx.use_memo((), || Rc::new(RefCell::new(0usize)));
    let mut first = first.borrow_mut();
    *first = (*first)
        .min(position.row)
        .max((position.row + 1).saturating_sub(visible))
        .min(rows.len() - visible);
    let position = Caret {
        row: position.row - *first,
        column: position.column + prompt_width,
    };
    cx.use_node_cursor(
        node,
        (!props.is_disabled).then_some(CursorPosition {
            x: position.column,
            y: position.row,
        }),
    );
    let on_caret = props.on_caret.clone();
    cx.use_layout_effect(
        (metrics.has_measured, position, on_caret.clone()),
        move || {
            if metrics.has_measured
                && let Some(callback) = on_caret
            {
                callback.call(position);
            }
        },
    );
    let mut body = View::new()
        .node_ref(node)
        .width(Percent(100.0))
        .flex_direction(FlexDirection::Column);
    for (row, line) in rows.iter().enumerate().skip(*first).take(visible) {
        let mut text = Text::new(if row == 0 {
            props.prompt.clone()
        } else {
            " ".repeat(prompt_width)
        })
        .wrap(TextWrap::TruncateEnd);
        let mut cursor_drawn = false;
        for (offset, grapheme) in value[line.start..line.end].grapheme_indices(true) {
            let at_cursor = !props.is_disabled && line.start + offset == state.cursor;
            cursor_drawn |= at_cursor;
            text = text.child(
                Text::new(if grapheme == "\t" { "    " } else { grapheme })
                    .inverse(at_cursor && theme.cursor_inverse)
                    .dim_color(placeholder && !at_cursor && theme.placeholder_dim),
            );
        }
        if !props.is_disabled && row == position.row + *first && !cursor_drawn {
            text = text.child(Text::new(" ").inverse(theme.cursor_inverse));
        } else if line.start == line.end {
            text = text.child(" ");
        }
        body = body.child(text);
    }
    body.into()
}
