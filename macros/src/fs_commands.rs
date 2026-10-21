use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Span, TokenStream};
use quote::quote;
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
    index: bool,
    routes: Vec<Route>,
}

struct Route {
    name: Name,
    group: Option<Plan>,
}

/// Expands a directory literal into command modules and dispatch types.
pub(super) fn expand(input: LitStr) -> Result<TokenStream> {
    let dir = input.value();
    if std::env::var_os("OUT_DIR").is_none() {
        return Err(Error::new(
            input.span(),
            format!(
                "{dir}: OUT_DIR is unset; add a build.rs containing \
                 fn main() {{ println!(\"cargo::rerun-if-changed={dir}\"); }} \
                 so adding command files triggers a rebuild"
            ),
        ));
    }
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| Error::new(input.span(), format!("{dir}: CARGO_MANIFEST_DIR is unset")))?;
    let tree = read_directory(&PathBuf::from(manifest).join(dir))
        .map_err(|message| Error::new(input.span(), message))?;
    let plan = plan_directory(tree, true).map_err(|message| Error::new(input.span(), message))?;
    Ok(emit(&plan))
}

fn read_directory(path: &Path) -> std::result::Result<Directory, String> {
    let error = |error| format!("{}: cannot read command directory: {error}", path.display());
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

fn plan_directory(tree: Directory, root: bool) -> std::result::Result<Plan, String> {
    let mut plan = Plan {
        index: false,
        routes: Vec::new(),
    };
    let mut names = BTreeMap::new();
    for entry in tree.entries {
        let (path, stem) = match &entry {
            Entry::File(path) => (path, path.file_stem()),
            Entry::Directory(dir) => (&dir.path, dir.path.file_name()),
        };
        let stem = stem.and_then(|stem| stem.to_str()).unwrap_or("");
        let name = plan_name(stem, path)?;
        if matches!(&entry, Entry::File(_)) && stem == "mod" {
            return Err(format!("{}: mod.rs is reserved", path.display()));
        }
        if let Some(previous) = names.insert(stem.to_owned(), path.clone()) {
            return Err(format!(
                "{}: command name conflicts with {}",
                path.display(),
                previous.display()
            ));
        }
        if matches!(&entry, Entry::File(_)) && stem == "index" {
            if root {
                return Err(format!(
                    "{}: root index.rs is not supported; use the root component's None arm",
                    path.display()
                ));
            }
            plan.index = true;
        } else {
            let group = match entry {
                Entry::File(_) => None,
                Entry::Directory(dir) => Some(plan_directory(dir, false)?),
            };
            plan.routes.push(Route { name, group });
        }
    }
    if plan.routes.is_empty() {
        return Err(format!(
            "{}: directory has no commands; index.rs does not count as a command",
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
    if !valid || matches!(stem, "self" | "super" | "crate" | "clap") {
        return Err(format!(
            "{}: expected a lowercase snake_case name starting with a letter; \
             self, super, crate and clap are reserved",
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
        let (attribute, field) = match &route.group {
            None => {
                modules.push(quote! {
                    #[doc = concat!("The ", stringify!(#module), " command component.")]
                    pub mod #module;
                });
                (quote! {}, quote! { #module::#ty })
            }
            Some(group) => {
                let contents = emit(group);
                modules.push(quote! {
                    #[doc = concat!("The ", stringify!(#module), " command group.")]
                    pub mod #module { #contents }
                });
                if group.index {
                    (quote! {}, quote! { #module::Group })
                } else {
                    (
                        quote! { #[command(subcommand)] },
                        quote! { #module::Command },
                    )
                }
            }
        };
        // Clap ignores doc expressions, leaving command metadata to the user's types.
        variants.push(quote! {
            #[doc = concat!("Dispatches the ", stringify!(#module), " command.")]
            #attribute
            #ty(#field),
        });
    }
    let group = plan.index.then(|| {
        quote! {
            /// The component rendered when no subcommand is selected.
            pub mod index;

            #[doc = concat!("A command group with an index component.")]
            #[derive(clap::Args, Clone, PartialEq)]
            #[command(args_conflicts_with_subcommands = true)]
            pub struct Group {
                /// The arguments for the group's index component.
                #[command(flatten)]
                pub index: index::Index,
                /// The selected subcommand.
                #[command(subcommand)]
                pub command: Option<Command>,
            }

            impl ::core::convert::From<Group> for ::ink_rs::Element {
                fn from(group: Group) -> Self {
                    match group.command {
                        Some(command) => command.into(),
                        None => group.index.into(),
                    }
                }
            }
        }
    });
    quote! {
        use ::eink_rs::cli::clap;

        #(#modules)*

        #[doc = concat!("The commands available in this directory.")]
        #[derive(clap::Subcommand, Clone, PartialEq, ::eink_rs::cli::Commands)]
        pub enum Command {
            #(#variants)*
        }

        #group
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

    #[test]
    fn names() {
        for (stem, module, ty) in [
            ("deploy", "deploy", "Deploy"),
            ("super_deploy", "super_deploy", "SuperDeploy"),
            ("deploy_v2", "deploy_v2", "DeployV2"),
            ("type", "r#type", "Type"),
            ("async", "r#async", "Async"),
            ("gen", "r#gen", "Gen"),
        ] {
            let plan = plan_directory(
                directory("commands", vec![file(&format!("{stem}.rs"))]),
                true,
            )
            .unwrap_or_else(|error| panic!("{error}"));
            assert_eq!(plan.routes[0].name.module.to_string(), module);
            assert_eq!(plan.routes[0].name.ty.to_string(), ty);
        }
    }

    #[test]
    fn invalid_directories() {
        let mut cases = Vec::new();
        for stem in [
            "Deploy",
            "super-deploy",
            "_deploy",
            "1deploy",
            "self",
            "super",
            "crate",
            "clap",
            "deploy_",
            "super__deploy",
        ] {
            let path = format!("commands/{stem}.rs");
            cases.push((
                directory("commands", vec![file(&path)]),
                path,
                "lowercase snake_case",
            ));
        }
        cases.extend([
            (
                directory("commands", vec![file("commands/mod.rs")]),
                "commands/mod.rs".into(),
                "mod.rs is reserved",
            ),
            (
                directory("commands", vec![file("commands/index.rs")]),
                "commands/index.rs".into(),
                "root index.rs",
            ),
            (
                directory("commands", vec![]),
                "commands".into(),
                "no commands",
            ),
            (
                directory(
                    "commands",
                    vec![Entry::Directory(directory("commands/empty", vec![]))],
                ),
                "commands/empty".into(),
                "no commands",
            ),
            (
                directory(
                    "commands",
                    vec![Entry::Directory(directory(
                        "commands/domains",
                        vec![file("commands/domains/index.rs")],
                    ))],
                ),
                "commands/domains".into(),
                "no commands",
            ),
            (
                directory(
                    "commands",
                    vec![
                        file("commands/foo.rs"),
                        Entry::Directory(directory(
                            "commands/foo",
                            vec![file("commands/foo/list.rs")],
                        )),
                    ],
                ),
                "commands/foo".into(),
                "conflicts with commands/foo.rs",
            ),
            (
                directory(
                    "commands",
                    vec![Entry::Directory(directory("commands/Bad", vec![]))],
                ),
                "commands/Bad".into(),
                "lowercase snake_case",
            ),
        ]);
        for (tree, path, message) in cases {
            let error = match plan_directory(tree, true) {
                Ok(_) => panic!("expected an error for {path}"),
                Err(error) => error,
            };
            assert!(error.contains(&path), "{error}");
            assert!(error.contains(message), "{error}");
        }
    }
}
