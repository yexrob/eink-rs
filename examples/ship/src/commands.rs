use eink_rs::cli::clap::{self, Args};
use ink_rs::{Element, Props, Scope, Static, Text};

use crate::{backend::use_backend, view};

/// A snapshot for one service or the fleet.
#[derive(Args, Clone, PartialEq, Props)]
#[props(component = status)]
pub struct Status {
    /// Service name; omit for the fleet.
    service: Option<String>,
}

fn status(cx: &mut Scope, props: &Status) -> Element {
    let backend = use_backend(cx);
    let result = match &props.service {
        Some(name) => backend
            .0
            .borrow()
            .service(name)
            .map(|service| vec![service]),
        None => Ok(backend.0.borrow().services()),
    };
    let error = result.as_ref().err().cloned();
    let app = cx.use_app();
    cx.use_effect(error.clone(), move || match error {
        Some(error) => app.exit_with_error(error),
        None => app.exit(),
    });
    match result {
        Ok(services) => view::column()
            .child(Text::new("SHIP / simulated snapshot").bold(true))
            .child(Text::new("SERVICE  VERSION  REPLICAS  STATUS"))
            .children(services.iter().map(|service| {
                Text::new(format!(
                    "{:<8} {:<8} {:<9} {}",
                    service.name,
                    service.version,
                    service.replicas,
                    service.health.label()
                ))
            }))
            .into(),
        Err(_) => Element::empty(),
    }
}

/// Log tail arguments for normal-screen output.
#[derive(Args, Clone, PartialEq, Props)]
#[props(component = logs)]
pub struct Logs {
    /// Service name.
    service: String,
    /// Number of initial lines.
    #[arg(short = 'n', default_value_t = 20)]
    count: usize,
    /// Append new lines until q, Esc or Ctrl-C.
    #[arg(short = 'f', long)]
    pub follow: bool,
}

fn logs(cx: &mut Scope, props: &Logs) -> Element {
    let backend = use_backend(cx);
    let error = backend.0.borrow().service(&props.service).err();
    let lines = cx.use_state(|| backend.0.borrow().logs(&props.service, None, props.count));
    let cursor = cx.use_state(|| {
        backend
            .0
            .borrow()
            .logs(&props.service, None, 1)
            .last()
            .map_or(0, |line| line.id)
    });
    let following = props.follow && error.is_none();
    let app = cx.use_app();
    let exiting = app.clone();
    let failure = error.clone();
    cx.use_effect((error.clone(), following), move || {
        if let Some(error) = failure {
            exiting.exit_with_error(error);
        } else if !following {
            exiting.exit();
        }
    });
    cx.use_input(following, move |input, key| {
        if input == "q" || key.escape {
            app.exit();
        }
    });
    let incoming = if following {
        backend
            .0
            .borrow()
            .logs(&props.service, Some(cursor.get()), usize::MAX)
    } else {
        vec![]
    };
    cx.use_layout_effect(incoming.clone(), move || {
        if let Some(last) = incoming.last() {
            cursor.set(last.id);
            lines.update(|lines| lines.extend(incoming));
        }
    });
    if error.is_some() {
        return Element::empty();
    }
    view::column()
        .child(Static::new(lines.get(), |line, _| {
            Text::new(view::log_text(line)).into()
        }))
        .child(
            following.then(|| {
                Text::new("Following simulated logs | q / Esc / Ctrl-C exit").dim_color(true)
            }),
        )
        .into()
}
