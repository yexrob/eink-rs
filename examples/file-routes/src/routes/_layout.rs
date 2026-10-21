use eink_rs::router::{Outlet, use_router};
use ink_rs::{Element, FlexDirection, Props, Scope, Text, View, rsx};

use super::Route;

/// The shell shared by every route.
#[derive(Default, PartialEq, Props)]
#[props(component = shell)]
pub struct Layout {}

fn shell(cx: &mut Scope, _: &Layout) -> Element {
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
