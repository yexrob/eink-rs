use ink_rs::{ColorLevel, Element, RenderOptions, render_to_string};

/// Renders an expected tree with the settings of `ink_rs::testing::render`
/// (100 columns, basic colors), so it compares with that instance's frames.
pub fn frame(element: impl Into<Element>) -> String {
    render_to_string(
        element,
        RenderOptions {
            columns: 100,
            color_level: ColorLevel::Basic,
        },
    )
}
