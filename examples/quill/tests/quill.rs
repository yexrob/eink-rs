use std::time::Duration;

use eink_rs::cli::parse_from;
use ink_rs::{
    ColorLevel, Options, RenderOptions,
    testing::{TestTerminal, mount, render_with},
};
use quill::Cli;

#[test]
fn interactive_turn() {
    let mut app = render_with(
        parse_from::<Cli>(["quill", "-a", "untrusted", "fix the failing test"]).unwrap(),
        RenderOptions {
            columns: 100,
            color_level: ColorLevel::None,
        },
    );
    for index in 0..3 {
        app.advance(Duration::from_secs(5));
        assert!(
            app.last_frame().contains("Would you like to"),
            "approval {index}: {}",
            app.last_frame()
        );
        app.input("y");
    }
    app.advance(Duration::from_secs(10));
    let frame = app.last_frame();
    assert!(frame.contains("• Ran cargo test"), "{frame}");
    assert!(frame.contains("• Edited src/parse.rs (+2 -1)"), "{frame}");
    assert!(frame.contains("All 3 tests pass."), "{frame}");
    assert!(!frame.contains("Would you like to"), "{frame}");
    assert!(!frame.contains("Working ("), "{frame}");
}

#[test]
fn exec_exit_codes() {
    for (prompt, failure) in [("fix the failing test", false), ("fail", true)] {
        let terminal = TestTerminal::new(100, 40);
        let mut app = mount(
            parse_from::<Cli>(["quill", "exec", prompt]).unwrap(),
            Options::default(),
            &terminal,
        );
        app.advance(Duration::from_secs(12));
        match app.exit_result() {
            Some(Ok(None)) if !failure => {}
            Some(Err(error)) if failure => {
                assert_eq!(error.to_string(), "simulated agent stream failed");
            }
            result => panic!("unexpected exec exit for {prompt}: {result:?}"),
        }
    }
}
