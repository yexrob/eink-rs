//! Single selection with generic values and index-based focus.
//!
//! Unlike Ink UI, callbacks run on input changes, empty lists keep their value,
//! and changed options reset in a layout effect before paint.

use std::ops::Range;

use ink_rs::{Callback, Element, Props, Scope, State, View};

use super::option_list::{self, OptionList, SelectOption, option_theme};
use super::{Claims, RingItem, use_ring_focus, use_theme};

/// A scrollable single-choice list. Enter selects the focused option.
#[derive(Clone, PartialEq, Props)]
#[props(component = select::<T>)]
pub struct Select<T: Clone + PartialEq + 'static> {
    /// The member key in the nearest ring. Defaults to a generated per-mount key.
    pub id: Option<String>,
    /// Requests focus when its active ring holds no key. Defaults to false.
    pub auto_focus: bool,
    /// Host-owned selection. When present, ignores `default_value`.
    pub value: Option<T>,
    /// Enables digits 1 through 9 to focus and select options. Defaults to false.
    pub number_hotkeys: bool,
    /// Available labeled values. Defaults to an empty list.
    pub options: Vec<SelectOption<T>>,
    /// Selection on mount and when options change. Defaults to none.
    pub default_value: Option<T>,
    /// Maximum visible options on mount and reset. Defaults to 5.
    pub visible_option_count: usize,
    /// Text whose first case-sensitive match in each label is highlighted.
    /// Defaults to an empty string, which highlights nothing.
    #[props(into)]
    pub highlight_text: String,
    /// Disables input and hides the pointer. Defaults to false.
    pub is_disabled: bool,
    /// Receives a value when input changes the selection.
    pub on_change: Option<Callback<T>>,
    /// Receives the focused value on every Enter or number hotkey.
    pub on_select: Option<Callback<T>>,
}

impl<T: Clone + PartialEq + 'static> Default for Select<T> {
    fn default() -> Self {
        Self {
            id: None,
            auto_focus: false,
            value: None,
            number_hotkeys: false,
            options: Vec::new(),
            default_value: None,
            visible_option_count: 5,
            highlight_text: String::new(),
            is_disabled: false,
            on_change: None,
            on_select: None,
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
    let node = cx.use_node_ref();
    let focused = use_ring_focus(
        cx,
        RingItem {
            key: props.id.clone(),
            node,
            auto_focus: props.auto_focus,
            claims: Claims::Arrows,
        },
    );
    let is_disabled = props.is_disabled || !focused;
    let state = use_select_state(
        cx,
        &props.options,
        props.value.clone().or_else(|| props.default_value.clone()),
        props.visible_option_count,
    );
    let initial_focus = props.value.as_ref().and_then(|value| {
        props
            .options
            .iter()
            .position(|option| &option.value == value)
    });
    cx.use_layout_effect(props.options.clone(), move || {
        if let Some(index) = initial_focus {
            let mut current = state.get();
            current.list = current.list.focus(index);
            state.set(current);
        }
    });
    let theme = use_theme::<SelectTheme>(cx);
    let on_change = props.on_change.clone();
    let on_select = props.on_select.clone();
    let controlled = props.value.clone();
    let number_hotkeys = props.number_hotkeys;
    cx.use_input(!is_disabled, move |input, key| {
        let previous = state.get();
        let mut next = previous.clone();
        let number = if number_hotkeys && !key.ctrl && !key.meta && !key.super_key && !key.hyper {
            input
                .as_bytes()
                .first()
                .filter(|_| input.len() == 1)
                .filter(|digit| (b'1'..=b'9').contains(digit))
                .map(|digit| (digit - b'1') as usize)
                .filter(|index| *index < next.list.options.len())
        } else {
            None
        };
        if key.down_arrow {
            next = next.focus_next_option();
        }
        if key.up_arrow {
            next = next.focus_previous_option();
        }
        if let Some(index) = number {
            next.list = next.list.focus(index);
        }
        let value = (key.enter || number.is_some())
            .then(|| {
                next.list
                    .focused_option()
                    .map(|option| option.value.clone())
            })
            .flatten();
        let changed = value
            .as_ref()
            .is_some_and(|value| Some(value) != controlled.as_ref().or(previous.value.as_ref()));
        if controlled.is_none()
            && let Some(value) = &value
        {
            next.value = Some(value.clone());
        }
        state.set(next);
        if changed && let (Some(callback), Some(value)) = (&on_change, value.clone()) {
            callback.call(value);
        }
        if let (Some(callback), Some(value)) = (&on_select, value) {
            callback.call(value);
        }
    });
    let current = state.get();
    View::new()
        .node_ref(node)
        .child(option_list::render(
            &current.list,
            is_disabled,
            &props.highlight_text,
            props.number_hotkeys,
            theme.rows(),
            |value| props.value.as_ref().or(current.value.as_ref()) == Some(value),
        ))
        .into()
}
