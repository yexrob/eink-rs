use std::cell::RefCell;
use std::rc::Rc;

use eink_rs::ui::{Code, CodeFormat, CodeWrap};
use ink_rs::{Callback, ColorLevel, RenderOptions, testing::render_with};

fn plain(code: Code, columns: u16) -> String {
    render_with(
        code,
        RenderOptions {
            columns,
            color_level: ColorLevel::None,
        },
    )
    .last_frame()
    .into()
}

#[test]
fn source_gutter_aligns_across_digit_boundary() {
    let code = Code::default()
        .source("one\ntwo\nthree")
        .start_line(Some(9));
    assert_eq!(plain(code, 30), " 9 one\n10 two\n11 three");
}

#[test]
fn two_hunks_use_header_numbers_and_dim_context() {
    let code = Code::default().format(CodeFormat::Diff).source("diff --git a/x b/x\nindex 1234567..89abcde 100644\n--- a/x\n+++ b/x\n@@ -9,2 +20,2 @@\n same\n-old\n+new\n@@ -100 +200 @@\n-last\n+next\n\\ No newline at end of file\n");
    assert_eq!(
        plain(code.clone(), 40),
        "          diff --git a/x b/x\n          index 1234567..89abcde 100644\n          --- a/x\n          +++ b/x\n          @@ -9,2 +20,2 @@\n  9  20   same\n 10     - old\n     21 + new\n          @@ -100 +200 @@\n100     - last\n    200 + next\n          \\ No newline at end of file"
    );
    let app = render_with(
        code,
        RenderOptions {
            columns: 40,
            color_level: ColorLevel::Basic,
        },
    );
    let context = app
        .last_frame()
        .lines()
        .find(|line| line.contains("same"))
        .unwrap();
    assert!(context.contains("\x1b[2m"));
}

#[test]
fn narrow_wrap_and_truncation_keep_gutter_on_first_row() {
    let code = Code::default()
        .source("  abcdefghi\nend")
        .start_line(Some(9));
    assert_eq!(plain(code.clone(), 9), " 9   abcd\n   efghi\n10 end");
    assert_eq!(
        plain(code.wrap(CodeWrap::TruncateEnd), 9),
        " 9   abc\u{2026}\n10 end"
    );
}

#[test]
fn broken_diffs_fall_back_once_per_source() {
    for source in ["not a diff\ntext", "@@ -1,2 +1,2 @@\n context\n"] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let log = calls.clone();
        let code = Code::default()
            .source(source)
            .format(CodeFormat::Diff)
            .start_line(Some(5))
            .language(Some("rust".into()))
            .on_fallback(Some(Callback::new(move |error| {
                log.borrow_mut().push(error)
            })));
        let mut app = render_with(
            code.clone(),
            RenderOptions {
                columns: 40,
                color_level: ColorLevel::None,
            },
        );
        let expected = source
            .lines()
            .enumerate()
            .map(|(index, line)| format!("{} {line}", index + 5))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(app.last_frame(), expected);
        app.rerender(code.wrap(CodeWrap::TruncateEnd));
        app.settle();
        assert_eq!(calls.borrow().len(), 1);
    }
}

#[test]
fn basic_diff_has_no_backgrounds() {
    let code = Code::default()
        .format(CodeFormat::Diff)
        .source("@@ -1 +1 @@\n-old\n+new");
    let app = render_with(
        code,
        RenderOptions {
            columns: 30,
            color_level: ColorLevel::Basic,
        },
    );
    for sequence in app.last_frame().split("\x1b[").skip(1) {
        let attributes = sequence.split('m').next().unwrap();
        assert!(
            !attributes
                .split(';')
                .filter_map(|part| part.parse::<u8>().ok())
                .any(|value| (40..=49).contains(&value) || (100..=107).contains(&value))
        );
    }
    assert!(app.last_frame().contains("\x1b[31m"));
    assert!(app.last_frame().contains("\x1b[32m"));
}
