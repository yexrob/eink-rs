use std::cmp::Ordering;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Data, DeriveInput, Error, Fields, Ident, LitStr, Result, Token, Type};

#[derive(Debug, PartialEq)]
enum Segment {
    Static(String),
    Param(String),
}

fn parse_path(path: &LitStr) -> Result<Vec<Segment>> {
    let value = path.value();
    if !value.starts_with('/') {
        return Err(Error::new_spanned(path, "route paths must start with /"));
    }
    if value == "/" {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    value[1..]
        .split('/')
        .map(|segment| {
            if segment.is_empty() {
                return Err(Error::new_spanned(path, "route segments cannot be empty"));
            }
            if segment.chars().skip(1).any(|ch| ch == ':') {
                return Err(Error::new_spanned(
                    path,
                    "route segments cannot contain : past the first character",
                ));
            }
            match segment.strip_prefix(':') {
                Some("") => Err(Error::new_spanned(
                    path,
                    "route parameter names cannot be empty",
                )),
                Some(name) => {
                    if names.contains(&name) {
                        return Err(Error::new_spanned(
                            path,
                            "route parameter names cannot repeat",
                        ));
                    }
                    names.push(name);
                    Ok(Segment::Param(name.to_owned()))
                }
                None => Ok(Segment::Static(segment.to_owned())),
            }
        })
        .collect()
}

struct RouteAttr {
    path: LitStr,
    page: Option<Type>,
}

impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = input.parse()?;
        let page = if input.is_empty() {
            None
        } else {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        };
        Ok(Self { path, page })
    }
}

fn attribute<'a>(attrs: &'a [Attribute], name: &str) -> Result<Option<&'a Attribute>> {
    let mut found = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident(name)) {
        if found.is_some() {
            return Err(Error::new_spanned(
                attr,
                format!("duplicate #[{name}] attribute"),
            ));
        }
        found = Some(attr);
    }
    Ok(found)
}

struct Field {
    ident: Ident,
    binding: Ident,
    ty: Type,
}

struct Variant {
    ident: Ident,
    named: bool,
    segments: Vec<Segment>,
    params: Vec<Field>,
    child: Option<Field>,
    page: Type,
    layout: Option<Type>,
}

struct Plan {
    name: Ident,
    layout: Option<Type>,
    variants: Vec<Variant>,
}

fn same_pattern(a: &[Segment], b: &[Segment]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| match (a, b) {
            (Segment::Param(_), Segment::Param(_)) => true,
            _ => a == b,
        })
}

fn priority(a: &Variant, b: &Variant) -> Ordering {
    for (a, b) in a.segments.iter().zip(&b.segments) {
        match (a, b) {
            (Segment::Static(_), Segment::Param(_)) => return Ordering::Less,
            (Segment::Param(_), Segment::Static(_)) => return Ordering::Greater,
            (Segment::Static(a), Segment::Static(b)) if a != b => return Ordering::Equal,
            _ => {}
        }
    }
    if same_pattern(&a.segments, &b.segments) {
        a.child.is_some().cmp(&b.child.is_some())
    } else {
        Ordering::Equal
    }
}

fn plan(input: DeriveInput) -> Result<Plan> {
    let Data::Enum(data) = &input.data else {
        return Err(Error::new_spanned(&input, "Routes requires an enum"));
    };
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &input.generics,
            "Routes does not support generic enums",
        ));
    }
    if let Some(attr) = attribute(&input.attrs, "route")? {
        return Err(Error::new_spanned(attr, "#[route] belongs on a variant"));
    }
    let layout = attribute(&input.attrs, "layout")?
        .map(Attribute::parse_args)
        .transpose()?;
    let mut variants: Vec<Variant> = Vec::new();
    for variant in &data.variants {
        if matches!(variant.fields, Fields::Unnamed(_)) {
            return Err(Error::new_spanned(
                variant,
                "Routes does not support tuple variants",
            ));
        }
        let attr = attribute(&variant.attrs, "route")?.ok_or_else(|| {
            Error::new_spanned(
                variant,
                "each variant requires exactly one #[route] attribute",
            )
        })?;
        let route: RouteAttr = attr.parse_args()?;
        let segments = parse_path(&route.path)?;
        let layout = attribute(&variant.attrs, "layout")?
            .map(Attribute::parse_args)
            .transpose()?;
        let mut params = Vec::new();
        let mut child = None;
        for (index, field) in variant.fields.iter().enumerate() {
            for attr in &field.attrs {
                if attr.path().is_ident("route") || attr.path().is_ident("layout") {
                    return Err(Error::new_spanned(
                        attr,
                        "route and layout attributes do not belong on fields",
                    ));
                }
            }
            let ident = field.ident.clone().expect("named field");
            let name = ident.unraw().to_string();
            let planned = Field {
                ident,
                binding: format_ident!("__route_field_{index}"),
                ty: field.ty.clone(),
            };
            if segments
                .iter()
                .any(|s| matches!(s, Segment::Param(p) if p == &name))
            {
                params.push(planned);
            } else if child.is_none() {
                child = Some(planned);
            } else {
                return Err(Error::new_spanned(
                    field,
                    "at most one field outside the path can be a child route",
                ));
            }
        }
        for segment in &segments {
            if let Segment::Param(name) = segment
                && !params.iter().any(|f| f.ident.unraw() == name)
            {
                return Err(Error::new_spanned(
                    &route.path,
                    format!("parameter :{name} requires a named field"),
                ));
            }
        }
        if layout.is_some() && child.is_none() {
            return Err(Error::new_spanned(
                attribute(&variant.attrs, "layout")?.expect("layout attribute"),
                "a variant layout requires a child route",
            ));
        }
        if child.is_none()
            && variants
                .iter()
                .any(|v| v.child.is_none() && same_pattern(&v.segments, &segments))
        {
            return Err(Error::new_spanned(
                &route.path,
                "duplicate leaf route pattern (parameter names do not distinguish patterns)",
            ));
        }
        let ident = &variant.ident;
        variants.push(Variant {
            ident: ident.clone(),
            named: matches!(variant.fields, Fields::Named(_)),
            segments,
            params,
            child,
            page: route.page.unwrap_or_else(|| syn::parse_quote!(#ident)),
            layout,
        });
    }
    Ok(Plan {
        name: input.ident,
        layout,
        variants,
    })
}

// Choose the first unconstrained candidate, preserving declaration order between
// unrelated patterns without passing a partial ordering to slice::sort_by.
fn match_order(variants: &[Variant]) -> Vec<usize> {
    let mut remaining: Vec<_> = (0..variants.len()).collect();
    let mut order = Vec::new();
    while !remaining.is_empty() {
        let next = remaining
            .iter()
            .position(|&candidate| {
                !remaining.iter().any(|&other| {
                    priority(&variants[other], &variants[candidate]) == Ordering::Less
                })
            })
            .expect("route specificity is acyclic");
        order.push(remaining.remove(next));
    }
    order
}

pub fn expand(input: DeriveInput) -> Result<TokenStream> {
    let plan = plan(input)?;
    let name = &plan.name;
    let mut matchers = Vec::new();
    for index in match_order(&plan.variants) {
        let variant = &plan.variants[index];
        let count = variant.segments.len();
        let length = if variant.child.is_some() {
            quote! { __segments.len() >= #count }
        } else {
            quote! { __segments.len() == #count }
        };
        let checks = variant.segments.iter().enumerate().map(|(index, segment)| {
            match segment {
                Segment::Static(text) => quote! { if __segments[#index] != #text { return None; } },
                Segment::Param(name) => {
                    let field = variant.params.iter().find(|f| f.ident.unraw() == name).unwrap();
                    let Field { binding, ty, .. } = field;
                    quote! { let #binding = <#ty as ::core::str::FromStr>::from_str(__segments[#index]).ok()?; }
                }
            }
        });
        let child = variant.child.as_ref().map(|Field { binding, ty, .. }| quote! {
            let #binding = <#ty as ::eink_rs::router::Routes>::from_segments(&__segments[#count..])?;
        });
        let pattern = variant_pattern(variant);
        matchers.push(quote! {
            if #length {
                if let Some(__route) = (|| {
                    #(#checks)*
                    #child
                    Some(#pattern)
                })() { return Some(__route); }
            }
        });
    }
    let mut writers = Vec::new();
    let mut renderers = Vec::new();
    for variant in &plan.variants {
        let pattern = variant_pattern(variant);
        let writes = variant.segments.iter().map(|segment| match segment {
            Segment::Static(text) => {
                quote! { __segments.push(::std::string::String::from(#text)); }
            }
            Segment::Param(name) => {
                let binding = &variant
                    .params
                    .iter()
                    .find(|f| f.ident.unraw() == name)
                    .unwrap()
                    .binding;
                quote! { __segments.push(::std::string::ToString::to_string(#binding)); }
            }
        });
        let child_write = variant.child.as_ref().map(|Field { binding, ty, .. }| {
            quote! {
                <#ty as ::eink_rs::router::Routes>::write_segments(#binding, __segments);
            }
        });
        writers.push(quote! { #pattern => { #(#writes)* #child_write } });
        let mut element = match &variant.child {
            Some(Field { binding, ty, .. }) => {
                quote! { <#ty as ::eink_rs::router::Routes>::render(#binding) }
            }
            None => default_element(&variant.page),
        };
        if let Some(layout) = &variant.layout {
            let layout = default_element(layout);
            element = quote! { ::eink_rs::router::provide_outlet(#layout, #element) };
        }
        for Field { binding, ty, .. } in variant.params.iter().rev() {
            element = quote! { ::eink_rs::router::provide_param::<#ty>(::core::clone::Clone::clone(#binding), #element) };
        }
        // Each arm's closure is its own component type, so switching variants
        // remounts the branch while a parameter change keeps it.
        renderers.push(quote! {
            #pattern => ::ink_rs::component(
                |_, __element: &::ink_rs::Element| __element.clone(),
                #element,
            )
        });
    }
    let subject = if plan.variants.is_empty() {
        quote! { *self }
    } else {
        quote! { self }
    };
    let mut render = quote! { match #subject { #(#renderers,)* } };
    if let Some(layout) = &plan.layout {
        let layout = default_element(layout);
        render = quote! { ::eink_rs::router::provide_outlet(#layout, #render) };
    }
    Ok(quote! {
        impl ::eink_rs::router::Routes for #name {
            fn from_segments(__segments: &[&str]) -> Option<Self> {
                #(#matchers)*
                None
            }

            fn write_segments(&self, __segments: &mut ::std::vec::Vec<::std::string::String>) {
                match #subject { #(#writers,)* }
            }

            fn render(&self) -> ::ink_rs::Element { #render }
        }

        impl ::core::str::FromStr for #name {
            type Err = ::eink_rs::router::RouteError;

            fn from_str(__path: &str) -> ::core::result::Result<Self, Self::Err> {
                let __segments: ::std::vec::Vec<_> = __path.split('/').filter(|s| !s.is_empty()).collect();
                <Self as ::eink_rs::router::Routes>::from_segments(&__segments)
                    .ok_or_else(|| ::eink_rs::router::RouteError { path: __path.into() })
            }
        }

        impl ::core::fmt::Display for #name {
            fn fmt(&self, __formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                let mut __segments = ::std::vec::Vec::new();
                <Self as ::eink_rs::router::Routes>::write_segments(self, &mut __segments);
                ::core::write!(__formatter, "/{}", __segments.join("/"))
            }
        }
    })
}

fn variant_pattern(variant: &Variant) -> TokenStream {
    let ident = &variant.ident;
    if variant.named {
        let fields = variant
            .params
            .iter()
            .chain(&variant.child)
            .map(|Field { ident, binding, .. }| quote! { #ident: #binding });
        quote! { Self::#ident { #(#fields,)* } }
    } else {
        quote! { Self::#ident }
    }
}

fn default_element(ty: &Type) -> TokenStream {
    quote! { ::core::convert::Into::<::ink_rs::Element>::into(<#ty as ::core::default::Default>::default()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        for (path, expected) in [
            ("/", vec![]),
            (
                "/market/:pair",
                vec![
                    Segment::Static("market".into()),
                    Segment::Param("pair".into()),
                ],
            ),
            ("/wallet", vec![Segment::Static("wallet".into())]),
            (
                "/\u{5e02}\u{573a}",
                vec![Segment::Static("\u{5e02}\u{573a}".into())],
            ),
        ] {
            assert_eq!(
                parse_path(&LitStr::new(path, proc_macro2::Span::call_site())).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn ordering() {
        let plan = plan(syn::parse_quote! {
            enum Route {
                #[route("/:id")] Dynamic { id: String, page: Child },
                #[route("/market/:pair")] Market { pair: String },
                #[route("/market/new")] New,
                #[route("/:name")] Leaf { name: String },
                #[route("/other")] Other,
                #[route("/:x")] Another { x: String, page: Child },
            }
        })
        .unwrap();
        assert_eq!(match_order(&plan.variants), [2, 1, 4, 3, 0, 5]);
    }

    #[test]
    fn validation_errors() {
        for (source, message) in [
            ("struct Route;", "requires an enum"),
            ("enum Route<T> {}", "generic enums"),
            ("#[route(\"/\")] enum Route {}", "belongs on a variant"),
            (
                "#[layout(A)] #[layout(B)] enum Route {}",
                "duplicate #[layout]",
            ),
            ("enum Route { #[route(\"/\")] A(String) }", "tuple variants"),
            ("enum Route { A }", "exactly one #[route]"),
            (
                "enum Route { #[route(\"/\")] #[route(\"/a\")] A }",
                "duplicate #[route]",
            ),
            ("enum Route { #[route(\"a\")] A }", "start with /"),
            (
                "enum Route { #[route(\"/a/\")] A }",
                "segments cannot be empty",
            ),
            (
                "enum Route { #[route(\"//a\")] A }",
                "segments cannot be empty",
            ),
            (
                "enum Route { #[route(\"/a:b\")] A }",
                "past the first character",
            ),
            (
                "enum Route { #[route(\"/::a\")] A }",
                "past the first character",
            ),
            ("enum Route { #[route(\"/:\")] A }", "names cannot be empty"),
            (
                "enum Route { #[route(\"/:a/:a\")] A { a: String } }",
                "names cannot repeat",
            ),
            (
                "enum Route { #[route(\"/:a\")] A }",
                "requires a named field",
            ),
            (
                "enum Route { #[route(\"/\")] A { a: B, b: C } }",
                "at most one field",
            ),
            (
                "enum Route { #[route(\"/\")] #[layout(L)] A }",
                "requires a child route",
            ),
            (
                "enum Route { #[route(\"/\")] #[layout(L)] #[layout(M)] A { page: B } }",
                "duplicate #[layout]",
            ),
            (
                "enum Route { #[route(\"/\")] A, #[route(\"/\")] B }",
                "duplicate leaf route pattern",
            ),
            (
                "enum Route { #[route(\"/:a\")] A { a: u8 }, #[route(\"/:b\")] B { b: String } }",
                "duplicate leaf route pattern",
            ),
            (
                "enum Route { #[route(\"/\")] A { #[route(\"/\")] page: B } }",
                "do not belong on fields",
            ),
            (
                "enum Route { #[route(\"/\")] A { #[layout(L)] page: B } }",
                "do not belong on fields",
            ),
        ] {
            let error = expand(syn::parse_str(source).unwrap()).unwrap_err();
            assert!(error.to_string().contains(message), "{source}: {error}");
        }
    }
}
