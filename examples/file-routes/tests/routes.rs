use eink_rs::cli;
use file_routes::{
    Cli,
    routes::{Route, market},
};
use ink_rs::testing::render;

#[test]
fn parsing_and_printing() {
    let route: Route = "/market/ETH/book".parse().unwrap();
    assert_eq!(
        route,
        Route::Market {
            route: market::Route::Pair {
                pair: market::pair::Pair("ETH".into()),
                route: market::pair::Route::Book,
            },
        }
    );
    assert_eq!(route.to_string(), "/market/ETH/book");
}

#[test]
fn route_words() {
    let element = cli::parse_from::<Cli>(["file-routes", "market", "ETH", "book"]).unwrap();
    assert_eq!(
        render(element).last_frame(),
        "\x1b[1mMarket desk | /market/ETH/book\x1b[22m\n\
         \x1b[2mm market | w wallet | Esc back (false) | q quit\x1b[22m\n\
         ETH | layout counter: 0\n\
         \x1b[2m1 chart | 2 book | c count | t pair\x1b[22m\n\
         ETH order book: bids | asks"
    );
}
