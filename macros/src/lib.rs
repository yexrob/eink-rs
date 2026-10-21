//! The Commands and Routes derives and the routes! macro for eink-rs.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod commands;
mod fs_routes;
mod routes;

/// Generates typed path matching, formatting, and rendering for a route enum.
///
/// Each variant has one `#[route("/path/:param")]`, with an optional page type
/// after the path. Named fields supply parameters and at most one child route.
/// `#[layout(Type)]` wraps an enum or a variant with a child route.
/// Tuple variants and generic enums are not supported.
#[proc_macro_derive(Routes, attributes(route, layout))]
pub fn derive_routes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    routes::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Generates `From<Enum> for ::ink_rs::Element` by converting each variant's field.
///
/// Each variant must contain one unnamed field that converts to an element.
/// Command props can derive `ink_rs::Props`; nested enums can derive `Commands`.
/// Structs, unions, and generic enums are not supported.
#[proc_macro_derive(Commands)]
pub fn derive_commands(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    commands::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Generates page modules and route enums from a directory.
///
/// The directory is relative to `CARGO_MANIFEST_DIR` and must match the inline
/// module containing this invocation. The package needs a build script that
/// prints `cargo::rerun-if-changed=<dir>`. The `eink_rs::router::routes` re-export
/// documents the layout and naming rules.
#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::LitStr);
    fs_routes::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
