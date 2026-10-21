//! Memory routing with layouts, typed parameters, and argv as the initial path.
//!
//! Routes: `/`, `/wallet`, `/market/:pair`, and `/market/:pair/book`.
//! Keys: m opens ETH, w opens the wallet, 1/2 select chart/book, c increments
//! the market layout counter, t changes the pair, Esc goes back, and q exits.
//!
//! ```sh
//! cargo run --example router --features cli,router
//! cargo run --example router --features cli,router -- market ETH
//! cargo run --example router --features cli,router -- market ETH book
//! cargo run --example router --features cli,router -- wallet
//! ```

use std::{fmt, process::ExitCode, str::FromStr};

use eink_rs::cli::{
    self,
    clap::{self, Parser},
};
use eink_rs::router::{Outlet, RouteArgs, Router, Routes, use_param, use_router};
use ink_rs::{Element, FlexDirection, Props, Scope, Text, View, rsx};

#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = app)]
struct Cli {
    #[command(flatten)]
    route: RouteArgs<Route>,
}

fn app(_: &mut Scope, props: &Cli) -> Element {
    Router::new(props.route.route.clone()).into()
}

#[derive(Routes, Clone, PartialEq)]
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

#[derive(Routes, Clone, PartialEq)]
enum MarketRoute {
    #[route("/")]
    Chart,
    #[route("/book")]
    Book,
}

#[derive(Clone, PartialEq)]
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

#[derive(Default, PartialEq, Props)]
#[props(component = shell)]
struct Shell {}

fn shell(cx: &mut Scope, _: &Shell) -> Element {
    let router = use_router::<Route>(cx);
    let path = router.route().to_string();
    let back = router.can_go_back();
    let app = cx.use_app();
    cx.use_input(true, move |input, key| {
        if key.escape {
            router.back();
        }
        match input {
            "q" => app.exit(),
            "m" => router.push_path("/market/ETH").unwrap(),
            "w" => router.push(Route::Wallet),
            _ => {}
        }
    });
    rsx! {
        View(flex_direction: FlexDirection::Column) {
            Text(bold: true) { "Market desk | " {path} }
            Text(dim_color: true) { {format!("m market | w wallet | Esc back ({back}) | q quit")} }
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
    let router = use_router::<Route>(cx);
    let current = pair.clone();
    cx.use_input(true, move |input, _| match input {
        "c" => count.update(|n| *n += 1),
        "1" => router.push(Route::Market {
            pair: current.clone(),
            page: MarketRoute::Chart,
        }),
        "2" => router.push(Route::Market {
            pair: current.clone(),
            page: MarketRoute::Book,
        }),
        "t" => {
            if let Route::Market { page, .. } = router.route() {
                let pair = if current.0 == "ETH" { "BTC" } else { "ETH" };
                router.push(Route::Market {
                    pair: Pair(pair.into()),
                    page: page.clone(),
                });
            }
        }
        _ => {}
    });
    rsx! {
        View(flex_direction: FlexDirection::Column) {
            Text { {format!("{pair} | layout counter: {}", count.get())} }
            Text(dim_color: true) { "1 chart | 2 book | c count | t pair" }
            Outlet {}
        }
    }
    .into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = home)]
struct Home {}

fn home(_: &mut Scope, _: &Home) -> Element {
    Text::new("Choose a market with m or open the wallet with w.").into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = chart)]
struct Chart {}

fn chart(cx: &mut Scope, _: &Chart) -> Element {
    Text::new(format!("{} chart: _/\\__/\\_", use_param::<Pair>(cx))).into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = book)]
struct Book {}

fn book(cx: &mut Scope, _: &Book) -> Element {
    Text::new(format!("{} order book: bids | asks", use_param::<Pair>(cx))).into()
}

#[derive(Default, PartialEq, Props)]
#[props(component = wallet)]
struct WalletPage {}

fn wallet(_: &mut Scope, _: &WalletPage) -> Element {
    Text::new("Wallet: no account connected.").into()
}

fn main() -> ExitCode {
    cli::run::<Cli>()
}
