use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::ext::IdentExt;
use syn::{Error, Ident, LitStr, Result};

struct Directory {
    path: PathBuf,
    entries: Vec<Entry>,
}

enum Entry {
    File(PathBuf),
    Directory(Directory),
}

struct Name {
    module: Ident,
    ty: Ident,
}

struct Plan {
    layout: bool,
    routes: Vec<Route>,
}

struct Route {
    name: Name,
    dynamic: bool,
    group: Option<Plan>,
}

/// Expands a directory literal into page modules and route enums.
pub(super) fn expand(input: LitStr) -> Result<TokenStream> {
    let dir = input.value();
    if std::env::var_os("OUT_DIR").is_none() {
        return Err(Error::new(
            input.span(),
            format!(
                "{dir}: OUT_DIR is unset; add a build.rs containing \
                 fn main() {{ println!(\"cargo::rerun-if-changed={dir}\"); }} \
                 so adding route files triggers a rebuild"
            ),
        ));
    }
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| Error::new(input.span(), format!("{dir}: CARGO_MANIFEST_DIR is unset")))?;
    let tree = read_directory(&PathBuf::from(manifest).join(dir))
        .map_err(|message| Error::new(input.span(), message))?;
    let plan = plan_directory(tree).map_err(|message| Error::new(input.span(), message))?;
    Ok(emit(&plan))
}

fn read_directory(path: &Path) -> std::result::Result<Directory, String> {
    let error = |error| format!("{}: cannot read route directory: {error}", path.display());
    let mut entries = fs::read_dir(path)
        .map_err(error)?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(error)?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut tree = Directory {
        path: path.to_owned(),
        entries: Vec::new(),
    };
    for entry in entries {
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if kind.is_dir() {
            tree.entries.push(Entry::Directory(read_directory(&path)?));
        } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            tree.entries.push(Entry::File(path));
        }
    }
    Ok(tree)
}

fn plan_directory(tree: Directory) -> std::result::Result<Plan, String> {
    let mut plan = Plan {
        layout: false,
        routes: Vec::new(),
    };
    let mut names = BTreeMap::new();
    let mut dynamic_directory: Option<PathBuf> = None;
    for entry in tree.entries {
        let (path, stem) = match &entry {
            Entry::File(path) => (path, path.file_stem()),
            Entry::Directory(dir) => (&dir.path, dir.path.file_name()),
        };
        let stem = stem.and_then(|stem| stem.to_str()).unwrap_or("");
        let is_file = matches!(&entry, Entry::File(_));
        if is_file && stem == "mod" {
            return Err(format!("{}: mod.rs is reserved", path.display()));
        }
        let parameter = stem.strip_prefix('[').and_then(|s| s.strip_suffix(']'));
        if is_file && parameter.is_some() {
            return Err(format!(
                "{}: dynamic segments must be directories",
                path.display()
            ));
        }
        if !is_file && stem == "index" {
            return Err(format!(
                "{}: index must be a file, not a directory",
                path.display()
            ));
        }
        let stem = parameter.unwrap_or(stem);
        if stem.starts_with('_') {
            if is_file && stem == "_layout" {
                plan.layout = true;
                continue;
            }
            return Err(format!(
                "{}: underscore-prefixed entries are reserved except for _layout.rs",
                path.display()
            ));
        }
        if parameter == Some("route") {
            return Err(format!(
                "{}: route is reserved for the child route field",
                path.display()
            ));
        }
        let name = plan_name(stem, path)?;
        if let Some(previous) = names.insert(stem.to_owned(), path.clone()) {
            return Err(format!(
                "{}: module name conflicts with {}",
                path.display(),
                previous.display()
            ));
        }
        if parameter.is_some() {
            if let Some(previous) = &dynamic_directory {
                return Err(format!(
                    "{}: only one dynamic directory is allowed per level; conflicts with {}",
                    path.display(),
                    previous.display()
                ));
            }
            dynamic_directory = Some(path.clone());
        }
        let dynamic = parameter.is_some();
        let group = match entry {
            Entry::File(_) => None,
            Entry::Directory(dir) => Some(plan_directory(dir)?),
        };
        plan.routes.push(Route {
            name,
            dynamic,
            group,
        });
    }
    if plan.routes.is_empty() {
        return Err(format!(
            "{}: directory has no pages or child directories; _layout.rs does not count",
            tree.path.display()
        ));
    }
    Ok(plan)
}

fn plan_name(stem: &str, path: &Path) -> std::result::Result<Name, String> {
    let valid = stem.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && stem.split('_').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    if !valid || matches!(stem, "self" | "super" | "crate") {
        return Err(format!(
            "{}: expected a lowercase snake_case name starting with a letter; \
             self, super and crate are reserved",
            path.display()
        ));
    }
    // Syn's keyword list predates the Rust 2024 reserved keyword `gen`.
    let module = if stem == "gen" {
        Ident::new_raw(stem, Span::call_site())
    } else {
        syn::parse_str::<Ident>(stem).unwrap_or_else(|_| Ident::new_raw(stem, Span::call_site()))
    };
    let mut pascal = String::new();
    for part in stem.split('_') {
        pascal.push(char::from(part.as_bytes()[0]).to_ascii_uppercase());
        pascal.push_str(&part[1..]);
    }
    Ok(Name {
        module,
        ty: Ident::new(&pascal, Span::call_site()),
    })
}

fn emit(plan: &Plan) -> TokenStream {
    let mut modules = Vec::new();
    let mut variants = Vec::new();
    for route in &plan.routes {
        let module = &route.name.module;
        let ty = &route.name.ty;
        let stem = module.unraw().to_string();
        let path = if route.dynamic {
            format!("/:{stem}")
        } else if route.group.is_none() && stem == "index" {
            "/".into()
        } else {
            // Static segments follow clap's kebab-case for words typed on the command line.
            format!("/{}", stem.replace('_', "-"))
        };
        let (attribute, fields) = match &route.group {
            None => {
                modules.push(quote! {
                    #[doc = concat!("The page at ", #path, ".")]
                    pub mod #module;
                });
                (quote! { #[route(#path, #module::#ty)] }, quote! {})
            }
            Some(group) => {
                let contents = emit(group);
                let directory = format!("[{stem}]");
                let path_attribute = route.dynamic.then(|| quote! { #[path = #directory] });
                let parameter = route.dynamic.then(|| quote! {
                    #[doc = concat!("The `:", #stem, "` path parameter.")]
                    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
                    pub struct #ty(
                        #[doc = "The path segment value."]
                        pub ::std::string::String,
                    );

                    impl ::core::str::FromStr for #ty {
                        type Err = ::core::convert::Infallible;

                        fn from_str(value: &str) -> ::core::result::Result<Self, Self::Err> {
                            Ok(Self(value.into()))
                        }
                    }

                    impl ::core::fmt::Display for #ty {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                            f.write_str(&self.0)
                        }
                    }
                });
                modules.push(quote! {
                    #[doc = concat!("The routes under ", #path, ".")]
                    #path_attribute
                    pub mod #module { #parameter #contents }
                });
                let field = route.dynamic.then(|| {
                    quote! {
                        #[doc = concat!("The `:", #stem, "` path parameter.")]
                        #module: #module::#ty,
                    }
                });
                (
                    quote! { #[route(#path)] },
                    quote! {
                        {
                            #field
                            #[doc = "The child route."]
                            route: #module::Route,
                        }
                    },
                )
            }
        };
        variants.push(quote! {
            #[doc = concat!("The route at ", #path, ".")]
            #attribute
            #ty #fields,
        });
    }
    let layout = plan.layout.then(|| {
        modules.push(quote! {
            #[doc = "The layout shared by this level's routes."]
            pub mod _layout;
        });
        quote! {
            #[layout(_layout::Layout)]
        }
    });
    quote! {
        #(#modules)*

        #[doc = "The routes available in this directory."]
        #[derive(::eink_rs::router::Routes, Clone, Debug, PartialEq)]
        #layout
        pub enum Route {
            #(#variants)*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(path: &str, entries: Vec<Entry>) -> Directory {
        Directory {
            path: path.into(),
            entries,
        }
    }

    fn file(path: &str) -> Entry {
        Entry::File(path.into())
    }

    fn child(path: &str) -> Entry {
        Entry::Directory(directory(path, vec![file(&format!("{path}/index.rs"))]))
    }

    #[test]
    fn names() {
        for (stem, module, ty) in [
            ("wallet", "wallet", "Wallet"),
            ("order_book", "order_book", "OrderBook"),
            ("book_v2", "book_v2", "BookV2"),
            ("type", "r#type", "Type"),
            ("async", "r#async", "Async"),
            ("gen", "r#gen", "Gen"),
            ("clap", "clap", "Clap"),
        ] {
            for entry in [
                file(&format!("routes/{stem}.rs")),
                child(&format!("routes/{stem}")),
                child(&format!("routes/[{stem}]")),
            ] {
                let plan = plan_directory(directory("routes", vec![entry]))
                    .unwrap_or_else(|error| panic!("{error}"));
                assert_eq!(plan.routes[0].name.module.to_string(), module);
                assert_eq!(plan.routes[0].name.ty.to_string(), ty);
                syn::parse2::<syn::File>(emit(&plan)).unwrap();
            }
        }
    }

    #[test]
    fn entry_mapping() {
        let plan = plan_directory(directory(
            "routes",
            vec![
                file("routes/_layout.rs"),
                file("routes/index.rs"),
                Entry::Directory(directory(
                    "routes/market",
                    vec![
                        child("routes/market/[pair]"),
                        file("routes/market/index.rs"),
                    ],
                )),
                file("routes/order_book.rs"),
                file("routes/wallet.rs"),
            ],
        ))
        .unwrap();
        assert!(plan.layout);
        assert_eq!(plan.routes.len(), 4);
        assert_eq!(plan.routes[0].name.module, "index");
        assert!(plan.routes[0].group.is_none());
        let market = plan.routes[1].group.as_ref().unwrap();
        assert!(!plan.routes[1].dynamic);
        assert!(!market.layout);
        assert!(market.routes[0].dynamic);
        assert!(market.routes[0].group.is_some());
        let output = emit(&plan);
        syn::parse2::<syn::File>(output.clone()).unwrap();
        let output = output.to_string();
        for expected in [
            quote! { pub mod _layout; },
            quote! { #[layout(_layout::Layout)] },
            quote! { #[derive(::eink_rs::router::Routes, Clone, Debug, PartialEq)] },
            quote! { #[route("/", index::Index)] Index },
            quote! { #[route("/wallet", wallet::Wallet)] Wallet },
            quote! { #[route("/order-book", order_book::OrderBook)] OrderBook },
            quote! { #[route("/market")] Market },
            quote! { route: market::Route },
            quote! { pub mod market },
            quote! { #[path = "[pair]"] pub mod pair },
            quote! { #[derive(Clone, Debug, PartialEq, Eq, Hash)] pub struct Pair },
            quote! { pub ::std::string::String },
            quote! { impl ::core::str::FromStr for Pair },
            quote! { type Err = ::core::convert::Infallible; },
            quote! { impl ::core::fmt::Display for Pair },
            quote! { #[route("/:pair")] Pair },
            quote! { pair: pair::Pair },
            quote! { route: pair::Route },
        ] {
            assert!(
                output.contains(&expected.to_string()),
                "{expected}: {output}"
            );
        }
        let raw = plan_directory(directory("routes", vec![child("routes/[type]")])).unwrap();
        let output = emit(&raw).to_string();
        for expected in [
            quote! { #[path = "[type]"] pub mod r#type },
            quote! { #[route("/:type")] Type },
            quote! { r#type: r#type::Type },
        ] {
            assert!(
                output.contains(&expected.to_string()),
                "{expected}: {output}"
            );
        }
    }

    #[test]
    fn invalid_directories() {
        let mut cases = Vec::new();
        for stem in [
            "Wallet",
            "order-book",
            "1wallet",
            "self",
            "super",
            "crate",
            "wallet_",
            "order__book",
            "[pair",
        ] {
            let path = format!("routes/{stem}");
            for entry in [file(&format!("{path}.rs")), child(&path)] {
                cases.push((
                    directory("routes", vec![entry]),
                    path.clone(),
                    "lowercase snake_case",
                ));
            }
        }
        cases.extend([
            (
                directory("routes", vec![child("routes/[]")]),
                "routes/[]".into(),
                "lowercase snake_case",
            ),
            (
                directory("routes", vec![file("routes/mod.rs")]),
                "routes/mod.rs".into(),
                "mod.rs is reserved",
            ),
            (
                directory("routes", vec![file("routes/[pair].rs")]),
                "routes/[pair].rs".into(),
                "dynamic segments must be directories",
            ),
            (
                directory("routes", vec![child("routes/index")]),
                "routes/index".into(),
                "index must be a file",
            ),
            (
                directory("routes", vec![file("routes/_page.rs")]),
                "routes/_page.rs".into(),
                "underscore-prefixed entries",
            ),
            (
                directory("routes", vec![child("routes/_layout")]),
                "routes/_layout".into(),
                "underscore-prefixed entries",
            ),
            (
                directory("routes", vec![child("routes/_private")]),
                "routes/_private".into(),
                "underscore-prefixed entries",
            ),
            (
                directory("routes", vec![child("routes/[_pair]")]),
                "routes/[_pair]".into(),
                "underscore-prefixed entries",
            ),
            (
                directory("routes", vec![file("routes/foo.rs"), child("routes/foo")]),
                "routes/foo".into(),
                "conflicts with routes/foo.rs",
            ),
            (
                directory(
                    "routes",
                    vec![child("routes/[pair]"), file("routes/pair.rs")],
                ),
                "routes/pair.rs".into(),
                "conflicts with routes/[pair]",
            ),
            (
                directory("routes", vec![child("routes/[pair]"), child("routes/pair")]),
                "routes/pair".into(),
                "conflicts with routes/[pair]",
            ),
            (
                directory(
                    "routes",
                    vec![child("routes/[asset]"), child("routes/[pair]")],
                ),
                "routes/[pair]".into(),
                "only one dynamic directory",
            ),
            (
                directory("routes", vec![child("routes/[route]")]),
                "routes/[route]".into(),
                "reserved for the child route field",
            ),
            (
                directory("routes", vec![]),
                "routes".into(),
                "no pages or child directories",
            ),
            (
                directory("routes", vec![file("routes/_layout.rs")]),
                "routes".into(),
                "no pages or child directories",
            ),
            (
                directory(
                    "routes",
                    vec![Entry::Directory(directory("routes/empty", vec![]))],
                ),
                "routes/empty".into(),
                "no pages or child directories",
            ),
        ]);
        for (tree, path, message) in cases {
            let error = match plan_directory(tree) {
                Ok(_) => panic!("expected an error for {path}"),
                Err(error) => error,
            };
            assert!(error.contains(&path), "{error}");
            assert!(error.contains(message), "{error}");
        }
    }
}
