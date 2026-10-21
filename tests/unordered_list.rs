//! Ported from Ink UI 2.0.0 test/unordered-list.tsx.

mod support;

use eink_rs::ui::{Palette, UnorderedList, UnorderedListTheme};
use ink_rs::{Element, FlexDirection, Scope, Styled, Text, View, component, rsx, testing::render};

fn list(nested: bool) -> Element {
    let green: Element = if nested {
        rsx! {
            View(flex_direction: FlexDirection::Column) {
                Text { "Green" }
                UnorderedList {
                    Text { "Light" }
                    Text { "Normal" }
                    Text { "Dark" }
                }
            }
        }
        .into()
    } else {
        Text::new("Green").into()
    };
    rsx! {
        UnorderedList {
            Text { "Red" }
            {green}
            Text { "Yellow" }
        }
    }
    .into()
}

fn themed(cx: &mut Scope, (markers, nested): &(Vec<String>, bool)) -> Element {
    cx.provide(UnorderedListTheme {
        markers: markers.clone(),
        ..UnorderedListTheme::from(&Palette::default())
    });
    list(*nested)
}

fn expected(marker: &str, nested_marker: Option<&str>) -> String {
    let line = |indent: &str, marker: &str, label: &str| {
        Text::new(indent)
            .child(Text::new(marker).dim_color(true))
            .child(format!(" {label}"))
    };
    let mut lines = vec![line("", marker, "Red"), line("", marker, "Green")];
    if let Some(nested_marker) = nested_marker {
        lines.extend(["Light", "Normal", "Dark"].map(|label| line("  ", nested_marker, label)));
    }
    lines.push(line("", marker, "Yellow"));
    support::frame(
        View::new()
            .flex_direction(FlexDirection::Column)
            .children(lines),
    )
}

#[test]
fn unordered_list() {
    let instance = render(list(false));
    assert_eq!(instance.last_frame(), expected("─", None));
}

#[test]
fn custom_marker() {
    let instance = render(component(themed, (vec!["+".into()], false)));
    assert_eq!(instance.last_frame(), expected("+", None));
}

#[test]
fn nested_list() {
    let instance = render(list(true));
    assert_eq!(instance.last_frame(), expected("─", Some("─")));
}

#[test]
fn custom_marker_in_nested_list() {
    let instance = render(component(themed, (vec!["+".into()], true)));
    assert_eq!(instance.last_frame(), expected("+", Some("+")));
}

#[test]
fn custom_markers_in_nested_lists() {
    let instance = render(component(themed, (vec!["─".into(), "+".into()], true)));
    assert_eq!(instance.last_frame(), expected("─", Some("+")));
}
