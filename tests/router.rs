#![cfg(feature = "router")]

use std::fmt;
use std::str::FromStr;

use eink_rs::router::{Outlet, Router, Routes, use_param, use_router};
use ink_rs::{Element, FlexDirection, Props, Scope, Text, View, rsx, testing::render};

#[derive(Clone, Debug, PartialEq)]
struct Pair(String);

impl FromStr for Pair {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.into()))
    }
}

impl fmt::Display for Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Routes, Clone, Debug, PartialEq)]
#[layout(Shell)]
enum Route {
    #[route("/")]
    Home,
    #[route("/market/:pair")]
    #[layout(MarketFrame)]
    Market { pair: Pair, page: MarketRoute },
    #[route("/wallet", WalletPage)]
    Wallet,
}

#[derive(Routes, Clone, Debug, PartialEq)]
enum MarketRoute {
    #[route("/")]
    Chart,
    #[route("/book")]
    Book,
}

#[derive(Default, PartialEq, Props)]
#[props(component = shell)]
struct Shell {}

fn shell(cx: &mut Scope, _: &Shell) -> Element {
    let count = cx.use_state(|| 0);
    let router = use_router::<Route>(cx);
    let header = format!(
        "{} back:{} shell:{}",
        router.route(),
        router.can_go_back(),
        count.get()
    );
    cx.use_input(true, move |input, _| match input {
        "s" => count.update(|n| *n += 1),
        "h" => router.push(Route::Home),
        "m" => router.push_path("/market/ETH").unwrap(),
        "b" => router.push_path("/market/ETH/book").unwrap(),
        "c" => router.push_path("/market/BTC/book").unwrap(),
        "w" => router.push(Route::Wallet),
        "r" => router.replace(Route::Wallet),
        "d" => router.push(router.route().clone()),
        "x" => router.back(),
        _ => {}
    });
    rsx! {
        View(flex_direction: FlexDirection::Column) {
            Text { {header} }
            Outlet {}
        }
    }
    .into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = market_frame)]
struct MarketFrame {}

fn market_frame(cx: &mut Scope, _: &MarketFrame) -> Element {
    let pair = use_param::<Pair>(cx);
    let count = cx.use_state(|| 0);
    cx.use_input(true, move |input, _| {
        if input == "l" {
            count.update(|n| *n += 1);
        }
    });
    rsx! {
        View(flex_direction: FlexDirection::Column) {
            Text { {format!("market:{pair} count:{}", count.get())} }
            Outlet {}
        }
    }
    .into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = home)]
struct Home {}

fn home(cx: &mut Scope, _: &Home) -> Element {
    let count = cx.use_state(|| 0);
    cx.use_input(true, move |input, _| {
        if input == "p" {
            count.update(|n| *n += 1);
        }
    });
    Text::new(format!("home:{}", count.get())).into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = wallet)]
struct WalletPage {}

fn wallet(_: &mut Scope, _: &WalletPage) -> Element {
    Text::new("wallet").into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = chart)]
struct Chart {}

fn chart(cx: &mut Scope, _: &Chart) -> Element {
    Text::new(format!("chart:{}", use_param::<Pair>(cx))).into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = book)]
struct Book {}

fn book(_: &mut Scope, _: &Book) -> Element {
    BookReader::default().into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = book_reader)]
struct BookReader {}

fn book_reader(cx: &mut Scope, _: &BookReader) -> Element {
    Text::new(format!("book:{}", use_param::<Pair>(cx))).into()
}

#[derive(Routes, Clone, Debug, PartialEq)]
enum Matching {
    #[route("/:number", Home)]
    Number { number: u32 },
    #[route("/:word")]
    Fallback { word: String, page: Index },
    #[route("/new", Home)]
    New,
    #[route("/market/:pair", Home)]
    Pair { pair: Pair },
}

#[derive(Routes, Clone, Debug, PartialEq)]
enum Index {
    #[route("/", Home)]
    Index,
}

#[test]
fn parsing_and_printing() {
    for (path, expected) in [
        ("/", Route::Home),
        ("/wallet", Route::Wallet),
        (
            "/market/ETH",
            Route::Market {
                pair: Pair("ETH".into()),
                page: MarketRoute::Chart,
            },
        ),
        (
            "/market/ETH/book",
            Route::Market {
                pair: Pair("ETH".into()),
                page: MarketRoute::Book,
            },
        ),
    ] {
        let route: Route = path.parse().unwrap();
        assert_eq!(route, expected);
        assert_eq!(route.to_string(), path);
        assert_eq!(route.to_string().parse::<Route>().unwrap(), route);
    }
    let pair: Matching = "/market/ETH".parse().unwrap();
    assert_eq!(
        pair,
        Matching::Pair {
            pair: Pair("ETH".into())
        }
    );
    assert_eq!(pair.to_string(), "/market/ETH");
    assert_eq!(
        "//market//ETH/book/".parse::<Route>().unwrap().to_string(),
        "/market/ETH/book"
    );
    assert_eq!("/new".parse::<Matching>().unwrap(), Matching::New);
    assert_eq!(
        "/42".parse::<Matching>().unwrap(),
        Matching::Number { number: 42 }
    );
    assert_eq!(
        "/word".parse::<Matching>().unwrap(),
        Matching::Fallback {
            word: "word".into(),
            page: Index::Index
        }
    );
    let error = "/missing".parse::<Route>().unwrap_err();
    assert_eq!(error.path, "/missing");
    assert_eq!(error.to_string(), "no route matches `/missing`");
}

#[test]
fn history() {
    let mut app = render(Router::new(Route::Home));
    assert_eq!(app.last_frame(), "/ back:false shell:0\nhome:0");
    app.input("x");
    assert_eq!(app.last_frame(), "/ back:false shell:0\nhome:0");
    app.input("m");
    assert_eq!(
        app.last_frame(),
        "/market/ETH back:true shell:0\nmarket:ETH count:0\nchart:ETH"
    );
    app.input("d");
    app.input("r");
    assert_eq!(app.last_frame(), "/wallet back:true shell:0\nwallet");
    app.input("x");
    assert_eq!(app.last_frame(), "/ back:false shell:0\nhome:0");
    app.input("r");
    app.input("x");
    assert_eq!(app.last_frame(), "/wallet back:false shell:0\nwallet");
    app.rerender(Router::new(Route::Home));
    assert_eq!(app.last_frame(), "/wallet back:false shell:0\nwallet");
}

#[test]
fn layouts_and_page_lifetime() {
    let mut app = render(Router::new(Route::Home));
    app.input("s");
    app.input("p");
    assert_eq!(app.last_frame(), "/ back:false shell:1\nhome:1");
    app.input("m");
    app.input("l");
    app.input("b");
    assert_eq!(
        app.last_frame(),
        "/market/ETH/book back:true shell:1\nmarket:ETH count:1\nbook:ETH"
    );
    app.input("x");
    assert_eq!(
        app.last_frame(),
        "/market/ETH back:true shell:1\nmarket:ETH count:1\nchart:ETH"
    );
    app.input("x");
    assert_eq!(app.last_frame(), "/ back:false shell:1\nhome:0");
}

#[test]
fn parameters() {
    let mut app = render(Router::new("/market/ETH/book".parse::<Route>().unwrap()));
    assert_eq!(
        app.last_frame(),
        "/market/ETH/book back:false shell:0\nmarket:ETH count:0\nbook:ETH"
    );
    app.input("l");
    app.input("c");
    assert_eq!(
        app.last_frame(),
        "/market/BTC/book back:true shell:0\nmarket:BTC count:1\nbook:BTC"
    );
}

#[cfg(feature = "cli")]
mod argv {
    use super::*;
    use eink_rs::cli::{
        self,
        clap::{self, Parser, Subcommand},
    };
    use eink_rs::router::RouteArgs;
    use std::process::ExitCode;

    #[derive(Parser, Clone, PartialEq, Props)]
    #[props(component = app)]
    struct Cli {
        #[command(flatten)]
        route: RouteArgs<Route>,
        #[command(subcommand)]
        command: Option<Command>,
    }

    #[derive(Subcommand, Clone, PartialEq)]
    enum Command {
        Wallet,
    }

    fn app(_: &mut Scope, props: &Cli) -> Element {
        match props.command {
            Some(Command::Wallet) => Text::new("wallet command").into(),
            None => Router::new(props.route.route.clone()).into(),
        }
    }

    #[test]
    fn route_words() {
        let element = cli::parse_from::<Cli>(["app", "market", "ETH", "book"]).unwrap();
        assert_eq!(
            render(element).last_frame(),
            "/market/ETH/book back:false shell:0\nmarket:ETH count:0\nbook:ETH"
        );
        let index = cli::parse_from::<Cli>(["app"]).unwrap();
        assert_eq!(render(index).last_frame(), "/ back:false shell:0\nhome:0");
    }

    #[test]
    fn unmatched_exit_code() {
        let error = Cli::try_parse_from(["app", "missing"]).err().unwrap();
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
        assert!(error.to_string().contains("/missing"));
        assert_eq!(cli::run_from::<Cli>(["app", "missing"]), ExitCode::from(2));
    }

    #[test]
    fn subcommand_precedence() {
        let element = cli::parse_from::<Cli>(["app", "wallet"]).unwrap();
        assert_eq!(render(element).last_frame(), "wallet command");
    }
}
