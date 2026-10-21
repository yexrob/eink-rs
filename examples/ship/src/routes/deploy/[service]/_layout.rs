use eink_rs::router::{Outlet, use_param, use_router};
use ink_rs::{Callback, Element, Props, Scope, Styled, Text, View, component};

use crate::{
    backend::use_backend,
    routes::{Route, deploy},
    view,
};

/// A wizard boundary with a draft whose lifetime belongs to one service.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = layout)]
pub struct Layout {}

/// The current draft and the layout-owned editing callback.
#[derive(Clone, PartialEq)]
pub(crate) struct Draft {
    pub(crate) version: Option<String>,
    pub(crate) regions: Vec<String>,
    pub(crate) deployment: Option<u64>,
    pub(crate) edit: Callback<Edit>,
}

/// Draft changes issued by wizard pages.
pub(crate) enum Edit {
    Version(String),
    Regions(Vec<String>),
    Started(u64),
}

pub(crate) fn use_draft(cx: &mut Scope) -> Draft {
    cx.use_context().expect("deploy draft provider")
}

fn layout(cx: &mut Scope, _: &Layout) -> Element {
    let service = use_param::<super::Service>(cx);
    component(draft, service.0.clone()).key(service.0)
}

fn draft(cx: &mut Scope, service: &String) -> Element {
    let backend = use_backend(cx);
    let router = use_router::<Route>(cx);
    let version = cx.use_state(|| None);
    let regions = cx.use_state(Vec::new);
    let deployment = cx.use_state(|| None);
    let edit = cx.use_callback(move |edit| match edit {
        Edit::Version(value) => {
            version.set(Some(value));
            deployment.set(None);
        }
        Edit::Regions(value) => {
            regions.set(value);
            deployment.set(None);
        }
        Edit::Started(id) => deployment.set(Some(id)),
    });
    cx.provide(Draft {
        version: version.get(),
        regions: regions.get(),
        deployment: deployment.get(),
        edit,
    });
    let step = match router.route() {
        Route::Deploy {
            route: deploy::Route::Service { route, .. },
        } => match route {
            super::Route::Index => 0,
            super::Route::Regions => 1,
            super::Route::Review => 2,
            super::Route::Run => 3,
        },
        _ => unreachable!("deploy layout"),
    };
    match backend.0.borrow().service(service) {
        Err(error) => view::alert(error),
        Ok(_) => view::column()
            .child(Text::new(format!(
                "Deploy {service} | draft: {} / {}",
                version.get().as_deref().unwrap_or("choose version"),
                match regions.get() {
                    regions if regions.is_empty() => "choose regions".into(),
                    regions => regions.join(", "),
                }
            )))
            .child(
                View::new().gap(2).children(
                    ["1 version", "2 regions", "3 review", "4 run"]
                        .into_iter()
                        .enumerate()
                        .map(|(i, name)| Text::new(name).inverse(i == step)),
                ),
            )
            .child(Outlet::default())
            .into(),
    }
}
