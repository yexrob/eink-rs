use ink_rs::{Element, Props, Scope};

use super::_layout::use_draft;
use crate::{backend::use_backend, view};

/// Observes backend work without owning its lifetime.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = run)]
pub struct Run {}

fn run(cx: &mut Scope, _: &Run) -> Element {
    let backend = use_backend(cx);
    let draft = use_draft(cx);
    let deployments = backend.0.borrow().deployments();
    match deployments.iter().find(|d| Some(d.id) == draft.deployment) {
        Some(deployment) => view::deployment_progress(deployment),
        None => view::alert("No deployment started. Open /deploy and complete the wizard."),
    }
}
