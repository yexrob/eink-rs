//! Multiple selection with generic values and index-based focus.
//!
//! Unlike Ink UI, callbacks run on input changes, empty values are selectable,
//! and changed options reset in a layout effect before paint.

use std::ops::Range;

use ink_rs::{Callback, Element, Props, Scope, State};

use super::option_list::{self, OptionList, SelectOption, option_theme};
use super::use_theme;

/// A scrollable multiple-choice list. Space toggles and Enter submits.
#[derive(Clone, PartialEq, Props)]
#[props(component = multi_select::<T>)]
pub struct MultiSelect<T: Clone + PartialEq + 'static> {
    /// Available labeled values. Defaults to an empty list.
    pub options: Vec<SelectOption<T>>,
    /// Selection on mount and when options change. Defaults to an empty list.
    pub default_value: Vec<T>,
    /// Maximum visible rows on mount and reset. Defaults to 5.
    pub visible_option_count: usize,
    /// Text whose first case-sensitive match in each label is highlighted.
    /// Defaults to an empty string, which highlights nothing.
    #[props(into)]
    pub highlight_text: String,
    /// Disables input and hides the pointer. Defaults to false.
    pub is_disabled: bool,
    /// Receives the selected values after each toggle.
    pub on_change: Option<Callback<Vec<T>>>,
    /// Receives the selected values when Enter is pressed.
    pub on_submit: Option<Callback<Vec<T>>>,
}

impl<T: Clone + PartialEq + 'static> Default for MultiSelect<T> {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            default_value: Vec::new(),
            visible_option_count: 5,
            highlight_text: String::new(),
            is_disabled: false,
            on_change: None,
            on_submit: None,
        }
    }
}

option_theme!(
    MultiSelectTheme,
    "Colors, glyphs and spacing for a MultiSelect."
);

/// A multiple selection and its navigation state, without input or rendering.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiSelectState<T> {
    list: OptionList<T>,
    /// Selected values in toggle order, including defaults absent from options.
    pub value: Vec<T>,
}

impl<T: Clone + PartialEq> MultiSelectState<T> {
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

    /// Removes a selected focused value or appends an unselected focused value.
    pub fn toggle_focused_option(mut self) -> Self {
        if let Some(option) = self.list.focused_option() {
            if self.value.contains(&option.value) {
                let mut remaining = Vec::new();
                for value in self.value {
                    // Ink UI's removal uses a Set, preserving the first occurrence.
                    if value != option.value && !remaining.contains(&value) {
                        remaining.push(value);
                    }
                }
                self.value = remaining;
            } else {
                self.value.push(option.value.clone());
            }
        }
        self
    }
}

/// Holds multiple-selection state and resets it before paint when options change.
///
/// Changes to the default or visible count alone do not reset the state.
/// Apply the state's pure transitions through [`State::get`] and [`State::set`].
pub fn use_multi_select_state<T: Clone + PartialEq + 'static>(
    cx: &mut Scope,
    options: &[SelectOption<T>],
    default_value: Vec<T>,
    visible_option_count: usize,
) -> State<MultiSelectState<T>> {
    let initial = MultiSelectState {
        list: OptionList::new(options.to_vec(), visible_option_count),
        value: default_value,
    };
    let state = cx.use_state(|| initial.clone());
    cx.use_layout_effect(options.to_vec(), move || state.set(initial));
    state
}

fn multi_select<T: Clone + PartialEq + 'static>(cx: &mut Scope, props: &MultiSelect<T>) -> Element {
    let state = use_multi_select_state(
        cx,
        &props.options,
        props.default_value.clone(),
        props.visible_option_count,
    );
    let theme = use_theme::<MultiSelectTheme>(cx);
    let on_change = props.on_change.clone();
    let on_submit = props.on_submit.clone();
    cx.use_input(!props.is_disabled, move |input, key| {
        let previous = state.get();
        let mut next = previous.clone();
        if key.down_arrow {
            next = next.focus_next_option();
        }
        if key.up_arrow {
            next = next.focus_previous_option();
        }
        if input == " " {
            next = next.toggle_focused_option();
        }
        let changed = next.value != previous.value;
        let value = next.value.clone();
        state.set(next);
        if changed && let Some(callback) = &on_change {
            callback.call(value.clone());
        }
        if key.enter
            && let Some(callback) = &on_submit
        {
            callback.call(value);
        }
    });
    let current = state.get();
    option_list::render(
        &current.list,
        props.is_disabled,
        &props.highlight_text,
        false,
        theme.rows(),
        |value| current.value.contains(value),
    )
}
