//! Typed in-app routing with a memory history and nested layouts.
//!
//! Enable `router` to derive [`Routes`] on a non-generic enum. Each variant has
//! one `#[route("/path/:name")]`; a second argument names its page type, otherwise
//! the variant name is the page type. Named fields matching parameters implement
//! `FromStr + Display + Clone + PartialEq + 'static`. One remaining field can be
//! a child route implementing [`Routes`]. Tuple variants are not supported.
//! Parameter displays must be nonempty path segments without `/`. [`routes!`]
//! generates the same enums from a directory of page files.
//!
//! Static segments match before parameters. For the same pattern, a leaf matches
//! before a child route; other ties retain declaration order. Parameter parse
//! failures fall through to the next candidate. Duplicate leaf patterns are
//! rejected, ignoring parameter names. `FromStr` ignores empty path segments.
//!
//! `#[layout(Type)]` on an enum wraps every variant at the same position. On a
//! variant it requires a child route and wraps that child. Pages and layouts
//! implement `Default + Into<Element>`. Layouts render their child with [`Outlet`].
//! Variant layouts see their parameters through [`use_param`]; enum layouts sit
//! outside those providers. Use parameter newtypes to distinguish values by type.
//!
//! Mount [`Router::new`] with an initial route and call [`use_router`] below it.
//! Navigation belongs in input handlers and effects, not render. The app binds
//! its own back key. The navigation key also reaches other input handlers that
//! were registered when the key arrived. A focused component that unmounts leaves
//! focus empty, so the new page's `auto_focus` applies. When a layout holds focus,
//! the page calls `focus(id)`. A page remounts when it is navigated back to;
//! layouts shared by the old and new route keep their state.
//!
//! With `cli`, flatten `RouteArgs` into a clap parser to use argv words as the
//! initial route's segments. Root subcommands take precedence over route words.

use std::any::type_name;
use std::fmt;

use ink_rs::{Callback, Element, Props, Scope, component};

pub use eink_rs_macros::Routes;

/// Generates page modules and route enums from a directory.
///
/// The path is relative to `CARGO_MANIFEST_DIR`. The macro belongs in an inline
/// module whose Rust module directory matches that path, such as this module in
/// `src/lib.rs` or `src/main.rs`:
///
/// ```ignore
/// pub mod routes {
///     eink_rs::router::routes!("src/routes");
/// }
/// ```
///
/// Each `.rs` file declares a public page type implementing
/// `Default + Into<Element>`. File stems use lowercase snake_case, page types
/// use PascalCase, and path segments use kebab-case: `order_book.rs` holds
/// `order_book::OrderBook` at `/order-book`.
/// Rust keywords use raw module identifiers: `type.rs` becomes `r#type::Type`.
/// `index.rs` holds `index::Index` at this level's `/`, including at the root.
/// Hidden entries and files without the `.rs` extension are skipped.
/// Entries are sorted by name before expansion; variants retain that order.
///
/// A static directory such as `market/` becomes an inline `market` module and
/// a `Market { route: market::Route }` variant at `/market`. Each level exposes
/// a public `Route` enum deriving [`Routes`], `Clone`, `Debug`, and `PartialEq`.
/// The derive gives static paths precedence over parameters.
///
/// A dynamic directory such as `[pair]/` becomes an inline `pair` module with
/// `#[path = "[pair]"]` and a `Pair { pair: pair::Pair, route: pair::Route }`
/// variant at `/:pair`. Its module contains a public `Pair(pub String)` newtype
/// deriving `Clone`, `Debug`, `PartialEq`, `Eq`, and `Hash`, with `FromStr`
/// (`Err = core::convert::Infallible`) and `Display` implementations.
/// Names inside brackets follow the same naming rules as static directories.
/// Pages under `[pair]/` read the parameter with `use_param::<super::Pair>(cx)`.
/// Parameter values must be nonempty path segments without `/`.
///
/// `_layout.rs` declares `_layout::Layout`, implementing `Default + Into<Element>`.
/// It adds `#[layout(_layout::Layout)]` to this level's enum and wraps every
/// route at that level. Layouts render the selected child through [`Outlet`].
/// A layout inside a dynamic directory can read that directory's parameter.
///
/// The calling package requires a `build.rs` that tracks the directory:
///
/// ```ignore
/// fn main() {
///     println!("cargo::rerun-if-changed=src/routes");
/// }
/// ```
///
/// This lets Cargo detect added and removed files. Expansion checks `OUT_DIR`
/// to detect a missing build script; it cannot check the script's output.
///
/// # Errors
///
/// Expansion reports the offending path for unreadable or missing directories,
/// a path that is not a directory, invalid names (including uppercase letters,
/// hyphens, leading digits, and empty snake_case parts), the reserved names
/// `self`, `super`, and `crate`, `mod.rs`, dynamic files such as `[pair].rs`,
/// an `index/` directory, or underscore-prefixed entries other than
/// `_layout.rs`.
///
/// Entries cannot share a module name, including `foo.rs` with `foo/` or
/// `pair.rs` with `[pair]/`. A level cannot contain two dynamic directories.
/// `[route]/` conflicts with the child field named `route`.
/// Each directory must contain a page or child directory; `_layout.rs` does not
/// count. An unset `OUT_DIR` reports the required build script line.
pub use eink_rs_macros::routes;

#[cfg(feature = "cli")]
mod args;
#[cfg(feature = "cli")]
pub use args::RouteArgs;

/// A typed route tree that matches paths and builds its element tree.
pub trait Routes: Clone + PartialEq + 'static {
    /// Matches all segments at this level, passing a suffix to a child route.
    ///
    /// An empty slice denotes this level's index. A failed parameter conversion
    /// rejects that candidate rather than preventing other candidates from matching.
    fn from_segments(segments: &[&str]) -> Option<Self>;

    /// Appends this route's segments, including child segments, to `segments`.
    ///
    /// The index appends nothing. Segments contain no `/` and are nonempty.
    /// Derived parents use this method to compose paths without duplicate slashes.
    fn write_segments(&self, segments: &mut Vec<String>);

    /// Builds this route's providers, layouts, and page without running hooks.
    ///
    /// Derived parents call this for child routes. Providers and layouts are
    /// component elements whose hooks run when ink-rs mounts or renders them.
    fn render(&self) -> Element;
}

/// A path that did not match a route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteError {
    /// The input path that did not match.
    pub path: String,
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no route matches `{}`", self.path)
    }
}

impl std::error::Error for RouteError {}

/// A component that owns a route history and provides navigation to descendants.
#[derive(Clone, PartialEq, Props)]
#[props(component = router)]
pub struct Router<R: Routes> {
    /// The route used when this component mounts; later changes are ignored.
    initial: R,
}

impl<R: Routes> Router<R> {
    /// Creates a router whose history starts with `initial`.
    pub fn new(initial: R) -> Self {
        Self { initial }
    }
}

fn router<R: Routes>(cx: &mut Scope, props: &Router<R>) -> Element {
    let history = cx.use_state(|| vec![props.initial.clone()]);
    let entries = history.get();
    let route = entries.last().expect("router history is nonempty").clone();
    let navigate = cx.use_callback(move |navigation| {
        history.update(|entries| match navigation {
            Navigation::Push(route) => {
                if entries.last() != Some(&route) {
                    entries.push(route);
                }
            }
            Navigation::Replace(route) => {
                *entries.last_mut().expect("router history is nonempty") = route;
            }
            Navigation::Back => {
                if entries.len() > 1 {
                    entries.pop();
                }
            }
        });
    });
    cx.provide(RouterHandle {
        route: route.clone(),
        depth: entries.len(),
        navigate,
    });
    route.render()
}

enum Navigation<R> {
    Push(R),
    Replace(R),
    Back,
}

/// A render-time route snapshot and a handle for navigation from events or effects.
#[derive(Clone, PartialEq)]
pub struct RouterHandle<R: Routes> {
    route: R,
    depth: usize,
    navigate: Callback<Navigation<R>>,
}

impl<R: Routes> RouterHandle<R> {
    /// Returns the route captured by the render that read this handle.
    pub fn route(&self) -> &R {
        &self.route
    }

    /// Reports whether the render-time history has an entry below this route.
    pub fn can_go_back(&self) -> bool {
        self.depth > 1
    }

    /// Pushes a route unless it equals the current history entry.
    pub fn push(&self, route: R) {
        self.navigate.call(Navigation::Push(route));
    }

    /// Replaces the current history entry without changing the history depth.
    pub fn replace(&self, route: R) {
        self.navigate.call(Navigation::Replace(route));
    }

    /// Pops the current history entry, leaving the bottom entry in place.
    pub fn back(&self) {
        self.navigate.call(Navigation::Back);
    }

    /// Parses and pushes a path, ignoring empty segments between slashes.
    pub fn push_path(&self, path: &str) -> Result<(), RouteError> {
        let segments: Vec<_> = path.split('/').filter(|s| !s.is_empty()).collect();
        let route = R::from_segments(&segments).ok_or_else(|| RouteError { path: path.into() })?;
        self.push(route);
        Ok(())
    }
}

/// Reads the nearest router for `R`.
///
/// Panics with the route type's name when no `Router<R>` is above this component.
pub fn use_router<R: Routes>(cx: &mut Scope) -> RouterHandle<R> {
    cx.use_context::<RouterHandle<R>>()
        .unwrap_or_else(|| panic!("no Router<{}> above this component", type_name::<R>()))
}

/// Reads the nearest route parameter of type `T`.
///
/// Panics with the parameter type's name when there is no provider above it.
pub fn use_param<T: Clone + 'static>(cx: &mut Scope) -> T {
    cx.use_context::<T>().unwrap_or_else(|| {
        panic!(
            "no route parameter {} above this component",
            type_name::<T>()
        )
    })
}

/// A component that renders the nearest layout's child, or an empty element.
///
/// An `Outlet` inside that child without a layout of its own renders nothing.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = outlet)]
pub struct Outlet {}

fn outlet(cx: &mut Scope, _: &Outlet) -> Element {
    let child = cx.use_context::<OutletChild>();
    // The child is consumed here, so a stray Outlet below cannot render it again.
    cx.provide(OutletChild(Element::empty()));
    child.map_or_else(Element::empty, |child| child.0)
}

#[derive(Clone, PartialEq)]
struct OutletChild(Element);

/// Wraps an element in a parameter provider for generated route implementations.
#[doc(hidden)]
pub fn provide_param<T: Clone + PartialEq + 'static>(value: T, child: Element) -> Element {
    component(parameter::<T>, (value, child))
}

fn parameter<T: Clone + PartialEq + 'static>(cx: &mut Scope, props: &(T, Element)) -> Element {
    cx.provide(props.0.clone());
    props.1.clone()
}

/// Wraps a layout in an outlet provider for generated route implementations.
#[doc(hidden)]
pub fn provide_outlet(layout: Element, child: Element) -> Element {
    component(layout_outlet, (layout, child))
}

fn layout_outlet(cx: &mut Scope, props: &(Element, Element)) -> Element {
    cx.provide(OutletChild(props.1.clone()));
    props.0.clone()
}
