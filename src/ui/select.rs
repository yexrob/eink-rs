//! Single selection with generic values and index-based focus.
//!
//! Unlike Ink UI, callbacks run on input changes, empty lists keep their value,
//! and changed options reset in a layout effect before paint.

use std::ops::Range;

use ink_rs::{Callback, Element, Props, Scope, State};

use super::option_list::{self, OptionList, SelectOption, option_theme};
use super::use_theme;

/// A scrollable single-choice list. Enter selects the focused option.
#[derive(Clone, PartialEq, Props)]
#[props(component = select::<T>)]
pub struct Select<T: Clone + PartialEq + 'static> {
    /// Available labeled values. Defaults to an empty list.
    pub options: Vec<SelectOption<T>>,
    /// Selection on mount and when options change. Defaults to none.
    pub default_value: Option<T>,
    /// Maximum visible rows on mount and reset. Defaults to 5.
    pub visible_option_count: usize,
    /// Text whose first case-sensitive match in each label is highlighted.
    /// Defaults to an empty string, which highlights nothing.
    #[props(into)]
    pub highlight_text: String,
    /// Disables input and hides the pointer. Defaults to false.
    pub is_disabled: bool,
    /// Receives a value when input changes the selection.
    pub on_change: Option<Callback<T>>,
}

impl<T: Clone + PartialEq + 'static> Default for Select<T> {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            default_value: None,
            visible_option_count: 5,
            highlight_text: String::new(),
            is_disabled: false,
            on_change: None,
        }
    }
}

option_theme!(SelectTheme, "Colors, glyphs and spacing for a Select.");

/// A single selection and its navigation state, without input or rendering.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectState<T> {
    list: OptionList<T>,
    /// Selected value, including defaults absent from the options.
    pub value: Option<T>,
}

impl<T: Clone + PartialEq> SelectState<T> {
    /// Returns the focused index, or none when the options are empty.
    pub fn focused_index(&self) -> Option<usize> {
        self.list.focused
    }

    /// Returns the half-open visible index range.
    pub fn visible_range(&self) -> Range<usize> {
        self.list.window.clone()
    }

    /// Returns the options in the visible window.
    pub fn visible_options(&self) -> &[SelectOption<T>] {
        self.list.visible_options()
    }

    /// Moves focus to the next option without wrapping.
    pub fn focus_next_option(mut self) -> Self {
        self.list = self.list.next();
        self
    }

    /// Moves focus to the previous option without wrapping.
    pub fn focus_previous_option(mut self) -> Self {
        self.list = self.list.previous();
        self
    }

    /// Selects the focused value, preserving the selection for empty options.
    pub fn select_focused_option(mut self) -> Self {
        if let Some(option) = self.list.focused_option() {
            self.value = Some(option.value.clone());
        }
        self
    }
}

/// Holds single-selection state and resets it before paint when options change.
///
/// Changes to the default or visible count alone do not reset the state.
/// Apply the state's pure transitions through [`State::get`] and [`State::set`].
pub fn use_select_state<T: Clone + PartialEq + 'static>(
    cx: &mut Scope,
    options: &[SelectOption<T>],
    default_value: Option<T>,
    visible_option_count: usize,
) -> State<SelectState<T>> {
    let initial = SelectState {
        list: OptionList::new(options.to_vec(), visible_option_count),
        value: default_value,
    };
    let state = cx.use_state(|| initial.clone());
    cx.use_layout_effect(options.to_vec(), move || state.set(initial));
    state
}

fn select<T: Clone + PartialEq + 'static>(cx: &mut Scope, props: &Select<T>) -> Element {
    let state = use_select_state(
        cx,
        &props.options,
        props.default_value.clone(),
        props.visible_option_count,
    );
    let theme = use_theme::<SelectTheme>(cx);
    let on_change = props.on_change.clone();
    cx.use_input(!props.is_disabled, move |_, key| {
        let previous = state.get();
        let mut next = previous.clone();
        if key.down_arrow {
            next = next.focus_next_option();
        }
        if key.up_arrow {
            next = next.focus_previous_option();
        }
        if key.enter {
            next = next.select_focused_option();
        }
        let changed = next.value != previous.value;
        let value = next.value.clone();
        state.set(next);
        if changed && let (Some(callback), Some(value)) = (&on_change, value) {
            callback.call(value);
        }
    });
    let current = state.get();
    option_list::render(
        &current.list,
        props.is_disabled,
        &props.highlight_text,
        theme.rows(),
        |value| current.value.as_ref() == Some(value),
    )
}
