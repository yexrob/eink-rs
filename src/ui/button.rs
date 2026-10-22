use ink_rs::{Callback, Color, Element, Props, Scope, Text, View};

use super::focus_ring::use_ring_hotkeys;
use super::{Claims, Palette, RingItem, use_ring_focus, use_theme};

/// A button's visual emphasis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Uses the palette's accent color.
    #[default]
    Primary,
    /// Uses the terminal's text color.
    Secondary,
}

/// Colors, brackets and hotkey separator for a Button.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonTheme {
    /// Primary button color. Defaults to the accent token.
    pub primary_color: Option<Color>,
    /// Secondary button color. Defaults to no color.
    pub secondary_color: Option<Color>,
    /// Opening bracket and spacing. Defaults to `"[ "`.
    pub opening: String,
    /// Closing bracket and spacing. Defaults to `" ]"`.
    pub closing: String,
    /// Separator after a hotkey. Defaults to `": "`.
    pub hotkey_separator: String,
    /// Inverts the focused button. Defaults to true.
    pub focus_inverse: bool,
}

impl From<&Palette> for ButtonTheme {
    fn from(palette: &Palette) -> Self {
        Self {
            primary_color: Some(palette.accent),
            secondary_color: None,
            opening: "[ ".into(),
            closing: " ]".into(),
            hotkey_separator: ": ".into(),
            focus_inverse: true,
        }
    }
}

/// A pressable label that joins the nearest focus ring.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = button)]
pub struct Button {
    /// The member key within the ring.
    #[props(into)]
    pub id: String,
    /// Text inside the button.
    #[props(into)]
    pub label: String,
    /// Text runs that win over `label` when not empty.
    pub children: Vec<Element>,
    /// A digit or lowercase ASCII letter that presses the button.
    pub hotkey: Option<char>,
    /// Omits brackets. Defaults to false.
    pub plain: bool,
    /// Visual emphasis. Defaults to primary.
    pub variant: ButtonVariant,
    /// Dims the button. Defaults to false.
    pub dim: bool,
    /// Requests focus when its active ring holds no key.
    pub auto_focus: bool,
    /// Disables presses. Defaults to false.
    pub is_disabled: bool,
    /// Runs on Enter, Space, or the hotkey.
    pub on_press: Option<Callback>,
}

fn button(cx: &mut Scope, props: &Button) -> Element {
    let node = cx.use_node_ref();
    let focused = use_ring_focus(
        cx,
        RingItem {
            key: Some(props.id.clone()),
            node,
            auto_focus: props.auto_focus,
            claims: Claims::None,
        },
    );
    let hotkeys = use_ring_hotkeys(cx);
    let theme = use_theme::<ButtonTheme>(cx);
    let hotkey = props
        .hotkey
        .filter(|key| key.is_ascii_digit() || key.is_ascii_lowercase());
    let on_press = props.on_press.clone();
    cx.use_input(
        !props.is_disabled && (focused || hotkeys),
        move |input, key| {
            if key.ctrl || key.meta || key.super_key || key.hyper {
                return;
            }
            let press = focused && (key.enter || input == " ");
            let hotkey = hotkeys && hotkey.is_some_and(|hotkey| input == hotkey.to_string());
            if (press || hotkey)
                && let Some(callback) = &on_press
            {
                callback.call(());
            }
        },
    );
    let mut text = Text::default()
        .color(match props.variant {
            ButtonVariant::Primary => theme.primary_color,
            ButtonVariant::Secondary => theme.secondary_color,
        })
        .dim_color(props.dim)
        .inverse(focused && !props.is_disabled && theme.focus_inverse);
    if !props.plain {
        text = text.child(theme.opening);
    }
    if let Some(hotkey) = hotkey {
        text = text.child(format!("{hotkey}{}", theme.hotkey_separator));
    }
    text = if props.children.is_empty() {
        text.child(props.label.as_str())
    } else {
        text.children(props.children.clone())
    };
    if !props.plain {
        text = text.child(theme.closing);
    }
    View::new().node_ref(node).child(text).into()
}
