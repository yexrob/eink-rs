//! Ported from Ink UI 2.0.0 test/ordered-list.tsx.

mod support;

use eink_rs::ui::OrderedList;
use ink_rs::{FlexDirection, Styled, Text, View, rsx, testing::render};

fn expected(lines: &[(&str, &str, &str)]) -> String {
    support::frame(
        View::new()
            .flex_direction(FlexDirection::Column)
            .children(lines.iter().map(|(indent, marker, label)| {
                Text::new(*indent)
                    .child(Text::new(*marker).dim_color(true))
                    .child(format!(" {label}"))
            })),
    )
}

#[test]
fn ordered_list() {
    let instance = render(rsx! {
        OrderedList {
            Text { "Red" }
            Text { "Green" }
            Text { "Yellow" }
        }
    });
    assert_eq!(
        instance.last_frame(),
        expected(&[("", "1.", "Red"), ("", "2.", "Green"), ("", "3.", "Yellow")])
    );
}

#[test]
fn nested_ordered_list() {
    let instance = render(rsx! {
        OrderedList {
            Text { "Red" }
            View(flex_direction: FlexDirection::Column) {
                Text { "Green" }
                OrderedList {
                    Text { "Light" }
                    Text { "Normal" }
                    Text { "Dark" }
                }
            }
            Text { "Yellow" }
        }
    });
    assert_eq!(
        instance.last_frame(),
        expected(&[
            ("", "1.", "Red"),
            ("", "2.", "Green"),
            ("   ", "2.1.", "Light"),
            ("   ", "2.2.", "Normal"),
            ("   ", "2.3.", "Dark"),
            ("", "3.", "Yellow"),
        ])
    );
}

#[test]
fn indent_all_list_items_equally() {
    let instance = render(
        OrderedList::default().children(
            [
                "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten",
            ]
            .map(Text::new),
        ),
    );
    assert_eq!(
        instance.last_frame(),
        expected(&[
            ("", " 1.", "One"),
            ("", " 2.", "Two"),
            ("", " 3.", "Three"),
            ("", " 4.", "Four"),
            ("", " 5.", "Five"),
            ("", " 6.", "Six"),
            ("", " 7.", "Seven"),
            ("", " 8.", "Eight"),
            ("", " 9.", "Nine"),
            ("", "10.", "Ten"),
        ])
    );
}
