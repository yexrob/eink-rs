//! A numbered Rust source and a unified diff at the window width.

use eink_rs::ui::{Code, CodeFormat};
use ink_rs::{Element, FlexDirection, Options, Scope, Styled, Text, View, component};

const SOURCE: &str = "fn greeting(name: &str) -> String {\n    format!(\"Hello, {name}!\")\n}\n";
const DIFF: &str = "--- a/greeting.rs\n+++ b/greeting.rs\n@@ -1,3 +1,3 @@\n fn greeting(name: &str) -> String {\n-    format!(\"Hello, {name}!\")\n+    format!(\"Welcome, {name}!\")\n }\n";

fn demo(cx: &mut Scope, _: &()) -> Element {
    let window = cx.use_window_size();
    View::new()
        .width(window.columns)
        .flex_direction(FlexDirection::Column)
        .gap(1)
        .child(Text::new("greeting.rs").bold(true))
        .child(
            Code::default()
                .source(SOURCE)
                .path(Some("greeting.rs".into()))
                .start_line(Some(1)),
        )
        .child(Text::new("Edit").bold(true))
        .child(
            Code::default()
                .source(DIFF)
                .path(Some("greeting.rs".into()))
                .format(CodeFormat::Diff),
        )
        .into()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ink_rs::render(component(demo, ()), Options::default()).wait_until_exit()?;
    Ok(())
}
