use cli_routes::Cli;
use eink_rs::cli::{
    self,
    clap::{CommandFactory, error::ErrorKind},
};
use ink_rs::testing::render;

#[test]
fn dispatch() {
    for (args, frame) in [
        (vec![], "Deploy website to us"),
        (
            vec!["deploy", "docs", "--region", "eu", "--preview"],
            "Deploy docs to eu (preview)",
        ),
        (vec!["d", "docs"], "Deploy docs to us"),
        (vec!["domains"], "Domains for website"),
        (vec!["dom", "--verbose"], "Domains for website in us"),
        (vec!["domains", "list"], "example.com"),
        (
            vec!["domains", "add", "example.org"],
            "Attach example.org to website",
        ),
        (vec!["team", "list"], "Deployment team: Ada, Lin"),
        (
            vec!["team", "projects", "list"],
            "Team projects: website, docs",
        ),
    ] {
        let element = cli::parse_from::<Cli>(std::iter::once("cli-routes").chain(args)).unwrap();
        assert_eq!(
            render(element).last_frame(),
            format!("Deployment console\n{frame}")
        );
    }
}

#[test]
fn group_without_index_requires_subcommand() {
    let error = cli::parse_from::<Cli>(["cli-routes", "team"])
        .err()
        .unwrap();
    assert_eq!(
        error.kind(),
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    );
    assert_eq!(error.exit_code(), 2);
    assert!(
        error
            .to_string()
            .contains("Usage: cli-routes team <COMMAND>")
    );
}

#[test]
fn index_metadata() {
    let command = <Cli as CommandFactory>::command();
    let domains = command.find_subcommand("domains").unwrap();
    assert_eq!(
        domains.get_about().unwrap().to_string(),
        "Manage deployment domains"
    );
    assert_eq!(domains.get_visible_aliases().collect::<Vec<_>>(), ["dom"]);
}
