use eink_rs::router::{Outlet, use_param, use_router};
use ink_rs::{Element, FlexDirection, Props, Scope, Text, View, rsx};

use super::{Pair, Route};
use crate::routes::{Route as AppRoute, market};

/// The pair layout with a counter shared by its pages.
#[derive(Default, PartialEq, Props)]
#[props(component = layout)]
pub struct Layout {}

fn layout(cx: &mut Scope, _: &Layout) -> Element {
    let pair = use_param::<Pair>(cx);
    let count = cx.use_state(|| 0);
    let router = use_router::<AppRoute>(cx);
    let current = pair.clone();
    cx.use_input(true, move |input, _| match input {
        "c" => count.update(|n| *n += 1),
        "1" => router.push(AppRoute::Market {
            route: market::Route::Pair {
                pair: current.clone(),
                route: Route::Index,
            },
        }),
        "2" => router.push(AppRoute::Market {
            route: market::Route::Pair {
                pair: current.clone(),
                route: Route::Book,
            },
        }),
        "t" => {
            if let AppRoute::Market {
                route: market::Route::Pair { route, .. },
            } = router.route()
            {
                let pair = if current.0 == "ETH" { "BTC" } else { "ETH" };
                router.push(AppRoute::Market {
                    route: market::Route::Pair {
                        pair: Pair(pair.into()),
                        route: route.clone(),
                    },
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
