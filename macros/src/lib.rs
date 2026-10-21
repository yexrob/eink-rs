//! Command dispatch derives for eink-rs.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod commands;

/// Generates `From<Enum> for ::ink_rs::Element` by converting each variant's field.
///
/// Each variant must contain one unnamed field that converts to an element.
/// Command props can derive `ink_rs::Props`; nested enums can derive `Commands`.
/// Structs, unions, and generic enums are not supported.
#[proc_macro_derive(Commands)]
pub fn commands(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    commands::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
