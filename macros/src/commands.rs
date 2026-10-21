use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result};

pub fn expand(input: DeriveInput) -> Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(Error::new_spanned(&input, "Commands requires an enum"));
    };
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &input.generics,
            "Commands does not support generic enums",
        ));
    }
    let name = &input.ident;
    let mut arms = Vec::new();
    for variant in &data.variants {
        match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {}
            _ => {
                return Err(Error::new_spanned(
                    variant,
                    "Commands requires each variant to have exactly one unnamed field",
                ));
            }
        }
        let ident = &variant.ident;
        arms.push(quote! {
            #name::#ident(field) => ::ink_rs::Element::from(field),
        });
    }
    Ok(quote! {
        impl ::core::convert::From<#name> for ::ink_rs::Element {
            fn from(command: #name) -> Self {
                match command {
                    #(#arms)*
                }
            }
        }
    })
}
