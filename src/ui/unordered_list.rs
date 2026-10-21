//! Each child is an item; there is no Ink UI `UnorderedList.Item` type.
//! Unkeyed [`Element::empty`] children, including `None`, are not items.
//! Wrap multiple elements or nested lists in a column [`View`].

use ink_rs::{Element, FlexDirection, Props, Scope, Styled, Text, View};

use super::{Palette, use_theme};

#[derive(Clone, Copy, Default, PartialEq)]
struct Depth(usize);

/// A list with a marker chosen by nesting depth.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = unordered_list)]
pub struct UnorderedList {
    /// Items, with one element per item. Unkeyed empty elements are omitted.
    pub children: Vec<Element>,
}

/// Markers and spacing for an [`UnorderedList`].
#[derive(Clone, Debug, PartialEq)]
pub struct UnorderedListTheme {
    /// Markers by depth, defaulting to `["─"]`. Deeper levels use the last marker.
    /// An empty list uses `─`.
    pub markers: Vec<String>,
    /// Whether markers are dimmed, true by default.
    pub marker_dim: bool,
    /// Cells between marker and content, one by default.
    pub gap: u16,
}

impl From<&Palette> for UnorderedListTheme {
    fn from(_: &Palette) -> Self {
        Self {
            markers: vec!["─".into()],
            marker_dim: true,
            gap: 1,
        }
    }
}

fn unordered_list(cx: &mut Scope, props: &UnorderedList) -> Element {
    let theme = use_theme::<UnorderedListTheme>(cx);
    let Depth(depth) = cx.use_context::<Depth>().unwrap_or_default();
    cx.provide(Depth(depth + 1));
    let marker = theme
        .markers
        .get(depth)
        .or_else(|| theme.markers.last())
        .map_or("─", String::as_str);
    View::new()
        .flex_direction(FlexDirection::Column)
        .children(
            props
                .children
                .iter()
                .filter(|child| **child != Element::empty())
                .map(|child| {
                    View::new()
                        .gap(theme.gap)
                        .child(
                            View::new()
                                .flex_shrink(0.0)
                                .child(Text::new(marker).dim_color(theme.marker_dim)),
                        )
                        .child(
                            View::new()
                                .flex_direction(FlexDirection::Column)
                                .min_width(0)
                                .child(child.clone()),
                        )
                }),
        )
        .into()
}
