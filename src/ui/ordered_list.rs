//! Each child is an item; there is no Ink UI `OrderedList.Item` type.
//! Unkeyed [`Element::empty`] children, including `None`, do not consume numbers.
//! Other children count as items, including components that render nothing.
//! Wrap multiple elements or nested lists in a column [`View`].

use ink_rs::{Element, FlexDirection, Props, Scope, Styled, Text, View, component};

use super::{Palette, use_theme};

#[derive(Clone, Default, PartialEq)]
struct ParentMarker(String);

/// A numbered list whose nested items include the enclosing item's marker.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = ordered_list)]
pub struct OrderedList {
    /// Items, with one element per item. Unkeyed empty elements are omitted.
    pub children: Vec<Element>,
}

/// Marker styling and spacing for an [`OrderedList`].
#[derive(Clone, Debug, PartialEq)]
pub struct OrderedListTheme {
    /// Whether markers are dimmed, true by default.
    pub marker_dim: bool,
    /// Cells between marker and content, one by default.
    pub gap: u16,
}

impl From<&Palette> for OrderedListTheme {
    fn from(_: &Palette) -> Self {
        Self {
            marker_dim: true,
            gap: 1,
        }
    }
}

fn ordered_list(cx: &mut Scope, props: &OrderedList) -> Element {
    let theme = use_theme::<OrderedListTheme>(cx);
    let ParentMarker(parent) = cx.use_context::<ParentMarker>().unwrap_or_default();
    let children: Vec<_> = props
        .children
        .iter()
        .filter(|child| **child != Element::empty())
        .collect();
    let digits = children.len().to_string().len();
    View::new()
        .flex_direction(FlexDirection::Column)
        .children(children.into_iter().enumerate().map(|(index, child)| {
            let number = index + 1;
            component(
                ordered_item,
                (
                    format!("{parent}{number:>digits$}."),
                    child.clone(),
                    theme.clone(),
                ),
            )
        }))
        .into()
}

fn ordered_item(
    cx: &mut Scope,
    (marker, child, theme): &(String, Element, OrderedListTheme),
) -> Element {
    cx.provide(ParentMarker(marker.clone()));
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
        .into()
}
