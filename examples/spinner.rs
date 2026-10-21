// Ink UI examples/spinner.

use eink_rs::ui::Spinner;
use ink_rs::{Element, Options, Scope, View, component, rsx};

fn example(_cx: &mut Scope, _: &()) -> Element {
    rsx! {
        View(padding: 2) {
            Spinner(label: "Loading")
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
