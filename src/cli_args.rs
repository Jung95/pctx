//! Singular global options have one meaning across the entire command path.
use clap::{
    builder::{
        BoolValueParser, EnumValueParser, PathBufValueParser, RangedU64ValueParser,
        TypedValueParser,
    },
    parser::ValueSource,
};
use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    sync::{Arc, Mutex},
};

type Seen = Arc<Mutex<BTreeSet<&'static str>>>;
#[derive(Clone)]
struct Singular<P> {
    parser: P,
    name: &'static str,
    seen: Seen,
}
impl<P: TypedValueParser> TypedValueParser for Singular<P> {
    type Value = P::Value;
    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        self.parse_ref_(cmd, arg, value, ValueSource::CommandLine)
    }
    fn parse_ref_(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &OsStr,
        source: ValueSource,
    ) -> Result<Self::Value, clap::Error> {
        if source == ValueSource::CommandLine {
            let mut seen = self.seen.lock().map_err(|_| {
                clap::Error::raw(
                    clap::error::ErrorKind::InvalidValue,
                    "Cannot validate global options",
                )
                .with_cmd(cmd)
            })?;
            if !seen.insert(self.name) {
                return Err(clap::Error::raw(
                    clap::error::ErrorKind::ArgumentConflict,
                    format!("--{} may be supplied only once", self.name),
                )
                .with_cmd(cmd));
            }
        }
        self.parser.parse_ref_(cmd, arg, value, source)
    }
    fn parse_(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: OsString,
        source: ValueSource,
    ) -> Result<Self::Value, clap::Error> {
        self.parse_ref_(cmd, arg, &value, source)
    }
    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_>> {
        self.parser.possible_values()
    }
}
fn guarded<P>(parser: P, name: &'static str, seen: &Seen) -> Singular<P> {
    Singular {
        parser,
        name,
        seen: seen.clone(),
    }
}
fn install(command: clap::Command, seen: &Seen, path: &str) -> clap::Command {
    command
        .mut_args(|arg| match arg.get_id().as_str() {
            "root" => arg.value_parser(guarded(PathBufValueParser::new(), "root", seen)),
            "output" => {
                // Inherited values arrive after leaf validation. The nonoptional
                // Pack field still requires presence after global propagation.
                let arg = if path == "pack create" {
                    arg.required(false).help("Required artifact path: supply exactly one --output anywhere along the command path")
                } else {
                    arg
                };
                arg.value_parser(guarded(PathBufValueParser::new(), "output", seen))
            }
            "format" => arg.value_parser(guarded(
                EnumValueParser::<crate::Format>::new(),
                "format",
                seen,
            )),
            "timeout_ms" => arg.value_parser(guarded(
                RangedU64ValueParser::<u64>::new(),
                "timeout-ms",
                seen,
            )),
            "no_color" => arg.value_parser(guarded(BoolValueParser::new(), "no-color", seen)),
            _ => arg,
        })
        .mut_subcommands(|child| {
            let child_path = format!("{path} {}", child.get_name());
            install(child, seen, child_path.trim())
        })
}
/// Build a fresh guard for each parse. Global clones and Pack Create's shared
/// output argument use the same guard; defaults and producer options do not.
pub fn singular_globals(command: clap::Command) -> clap::Command {
    install(command, &Seen::default(), "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, FromArgMatches};
    fn parse(values: &[&str]) -> Result<crate::Cli, clap::Error> {
        let matches = singular_globals(crate::Cli::command())
            .try_get_matches_from(std::iter::once("pctx").chain(values.iter().copied()))?;
        crate::Cli::from_arg_matches(&matches)
    }
    #[test]
    fn defaults_and_fresh_parses_do_not_count_as_duplicate_inputs() {
        for _ in 0..3 {
            let cli = parse(&["checkpoint", "list"]).unwrap();
            assert!(matches!(cli.format, crate::Format::Compact));
            assert!(!cli.no_color);
            assert!(cli.root.is_none() && cli.output.is_none() && cli.timeout_ms.is_none());
            let cli = parse(&[
                "checkpoint",
                "--format",
                "json",
                "list",
                "--root",
                "project",
                "--timeout-ms",
                "123",
                "--no-color",
            ])
            .unwrap();
            assert!(matches!(cli.format, crate::Format::Json));
            assert!(cli.no_color);
            assert_eq!(cli.root, Some("project".into()));
            assert_eq!(cli.timeout_ms, Some(123));
        }
    }
    #[test]
    fn producer_repetition_and_literal_child_options_preserve_exact_values() {
        let cli = parse(&["find", "needle", "--scope", "one", "--scope", "two"]).unwrap();
        let crate::Command::Find(request) = cli.command else {
            panic!("Expected Find")
        };
        assert_eq!(request.scopes, ["one", "two"]);
        let cli = parse(&[
            "--format",
            "json",
            "run",
            "--",
            "fixture",
            "--format=json",
            "--format=compact",
            "--root",
            "literal",
            "--timeout-ms",
            "0",
        ])
        .unwrap();
        let crate::Command::Run(request) = cli.command else {
            panic!("Expected Run")
        };
        assert_eq!(
            request.argv,
            [
                "fixture",
                "--format=json",
                "--format=compact",
                "--root",
                "literal",
                "--timeout-ms",
                "0"
            ]
        );
    }
    #[test]
    fn pack_artifact_output_and_global_output_share_one_singular_binding() {
        for position in 0..3 {
            let mut values = vec![
                "pack",
                "create",
                "--plan",
                "fixture",
                "--expect-hash",
                "hash",
            ];
            values.splice(position..position, ["--output", "artifact.pack"]);
            let cli = parse(&values).unwrap();
            assert_eq!(cli.output, Some("artifact.pack".into()));
            let crate::Command::Pack {
                command: crate::pack::PackCommand::Create { output, .. },
            } = cli.command
            else {
                panic!("Expected Pack Create")
            };
            assert_eq!(output, std::path::PathBuf::from("artifact.pack"));
        }
        let missing = parse(&[
            "pack",
            "create",
            "--plan",
            "fixture",
            "--expect-hash",
            "hash",
        ])
        .unwrap_err();
        assert_eq!(
            missing.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let error = parse(&[
            "--output",
            "first.pack",
            "pack",
            "create",
            "--plan",
            "fixture",
            "--expect-hash",
            "hash",
            "--output",
            "second.pack",
        ])
        .unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
