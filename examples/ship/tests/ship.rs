use eink_rs::cli::parse_from;
use ink_rs::{
    Options,
    testing::{TestTerminal, mount, render},
};
use ship::Cli;

#[test]
fn every_route_renders() {
    for path in [
        "/",
        "/services",
        "/services/api",
        "/services/api/logs",
        "/services/api/scale",
        "/services/api/rollback",
        "/deploy",
        "/deploy/api",
        "/deploy/api/regions",
        "/deploy/api/review",
        "/deploy/api/run",
        "/settings",
        "/settings/account",
    ] {
        let argv = std::iter::once("ship").chain(path.split('/').filter(|word| !word.is_empty()));
        let app = render(parse_from::<Cli>(argv).unwrap());
        assert!(
            app.last_frame().contains(&format!("Route: {path}")),
            "{path}: {}",
            app.last_frame()
        );
    }
}

#[test]
fn status_exit_codes() {
    let terminal = TestTerminal::new(100, 24);
    let mut known = mount(
        parse_from::<Cli>(["ship", "status", "api"]).unwrap(),
        Options::default(),
        &terminal,
    );
    known.advance(std::time::Duration::ZERO);
    assert!(matches!(known.exit_result(), Some(Ok(None))));
    let mut unknown = mount(
        parse_from::<Cli>(["ship", "status", "nope"]).unwrap(),
        Options::default(),
        &terminal,
    );
    unknown.advance(std::time::Duration::ZERO);
    match unknown.exit_result() {
        Some(Err(error)) => assert_eq!(
            error.to_string(),
            "unknown service 'nope'; known services: api, web, worker"
        ),
        _ => panic!("unknown service must request the error exit that cli maps to 1"),
    }
}
