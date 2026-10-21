use ink_rs::{Color, Element, Text};

use crate::cells::column;

/// Renders paragraphs, lists, fences and the two supported inline styles.
pub(crate) fn render(source: &str) -> Element {
    let mut body = column();
    let mut fenced = false;
    for line in source.lines() {
        if let Some(language) = line.strip_prefix("```") {
            fenced = !fenced;
            if fenced {
                body = body.child(Text::new(format!("  {language}")).dim_color(true));
            }
        } else if fenced {
            body = body.child(Text::new(format!("  {line}")).color(Color::Cyan));
        } else {
            body = body.child(inline(if line.is_empty() { " " } else { line }));
        }
    }
    body.into()
}

fn inline(mut source: &str) -> Text {
    let mut text = Text::default();
    while !source.is_empty() {
        let bold = source.find("**").map(|at| (at, "**"));
        let code = source.find('`').map(|at| (at, "`"));
        let next = match (bold, code) {
            (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
            (a, b) => a.or(b),
        };
        let Some((start, marker)) = next else {
            return text.child(source);
        };
        text = text.child(&source[..start]);
        let rest = &source[start + marker.len()..];
        if let Some(end) = rest.find(marker) {
            text = text.child(if marker == "**" {
                Text::new(&rest[..end]).bold(true)
            } else {
                Text::new(&rest[..end]).color(Color::Cyan)
            });
            source = &rest[end + marker.len()..];
        } else {
            return text.child(&source[start..]);
        }
    }
    text
}
