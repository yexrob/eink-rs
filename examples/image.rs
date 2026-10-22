use eink_rs::image::Image;
use ink_rs::{Element, FlexDirection, Options, Scope, Styled, View, component};

fn images(cx: &mut Scope, _: &()) -> Element {
    let sources = cx.use_memo((), || std::env::args().skip(1).collect::<Vec<_>>());
    let app = cx.use_app();
    cx.use_input(true, move |text, _| {
        if text == "q" {
            app.exit();
        }
    });
    View::new()
        .flex_direction(FlexDirection::Column)
        .children(sources.iter().map(|src| Image {
            src: src.clone(),
            alt: src.clone(),
            ..Image::default()
        }))
        .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(images, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
