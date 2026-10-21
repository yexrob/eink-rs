use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches, error::ErrorKind};

use super::{RouteError, Routes};

/// Initial route arguments for a flattened clap parser.
///
/// The positional `[ROUTE]...` takes each word as a path segment; flags may sit
/// between the words. No words select the root index. An unmatched path
/// produces an `InvalidValue` error.
/// clap matches the first word, and a word right after a flag, against root
/// subcommand names; a match runs that subcommand, and the flattened route then
/// uses its index. The route type therefore needs an index when it shares a
/// parser with subcommands. Other words are segments, even when they name a
/// subcommand.
#[derive(Clone, PartialEq)]
pub struct RouteArgs<R: Routes> {
    /// The route selected by the positional arguments.
    pub route: R,
}

impl<R: Routes> Args for RouteArgs<R> {
    fn augment_args(command: Command) -> Command {
        command.arg(
            Arg::new("route")
                .value_name("ROUTE")
                .num_args(0..)
                .action(ArgAction::Append),
        )
    }

    fn augment_args_for_update(command: Command) -> Command {
        Self::augment_args(command)
    }
}

impl<R: Routes> FromArgMatches for RouteArgs<R> {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let segments: Vec<_> = matches
            .get_many::<String>("route")
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect();
        R::from_segments(&segments)
            .map(|route| Self { route })
            .ok_or_else(|| {
                clap::Error::raw(
                    ErrorKind::InvalidValue,
                    RouteError {
                        path: format!("/{}", segments.join("/")),
                    },
                )
            })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        if matches.contains_id("route") {
            *self = Self::from_arg_matches(matches)?;
        }
        Ok(())
    }
}
