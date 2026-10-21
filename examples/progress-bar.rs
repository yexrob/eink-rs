// Ink UI examples/progress-bar.
// use_animation replaces setTimeout because ink-rs owns the timer lifecycle.
// Progress follows elapsed 50 ms frames, skipping missed ticks, rather than adding one per timeout.

use std::time::Duration;

use eink_rs::ui::ProgressBar;
use ink_rs::{AnimationOptions, Element, Options, Scope, View, component, rsx};

fn example(cx: &mut Scope, _: &()) -> Element {
    let finished = cx.use_state(|| false);
    let animation = cx.use_animation(AnimationOptions {
        interval: Duration::from_millis(50),
        active: !finished.get(),
    });
    let progress = animation.frame.min(100);
    cx.use_layout_effect(progress == 100, move || {
        if progress == 100 {
            finished.set(true);
        }
    });
    rsx! {
        View(padding: 2, width: 30) {
            ProgressBar(value: progress as f64)
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
