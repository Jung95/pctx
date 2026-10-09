//! Test-only inventory and cases derived from the real visible Clap tree.
use super::*;
use clap::{Arg, ArgAction};
use std::{
    any::TypeId,
    collections::{BTreeMap, BTreeSet},
};

type Inputs = BTreeMap<String, Vec<String>>;

fn parse(argv: &[String]) -> std::result::Result<Cli, clap::Error> {
    let matches = cli_help::annotate(cli_args::singular_globals(Cli::command()))
        .try_get_matches_from(std::iter::once("pctx").chain(argv.iter().map(String::as_str)))?;
    Cli::from_arg_matches(&matches)
}

fn numeric(arg: &Arg) -> Option<(&'static str, String, String)> {
    let id = arg.get_value_parser().type_id();
    macro_rules! types {
        ($($ty:ty),*) => { $(if id == TypeId::of::<$ty>() {
            return Some((stringify!($ty), <$ty>::MIN.to_string(), <$ty>::MAX.to_string()));
        })* };
    }
    types!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);
    None
}

fn sample(arg: &Arg) -> String {
    if let Some(value) = arg.get_possible_values().first() {
        value.get_name().into()
    } else if numeric(arg).is_some() {
        "1".into()
    } else {
        "parser-fixture".into()
    }
}

fn type_name(arg: &Arg) -> &'static str {
    if let Some((name, _, _)) = numeric(arg) {
        return name;
    }
    let id = arg.get_value_parser().type_id();
    if id == TypeId::of::<String>() {
        "string"
    } else if id == TypeId::of::<PathBuf>() {
        "path"
    } else if id == TypeId::of::<bool>() {
        "bool"
    } else if id == TypeId::of::<Format>() {
        "format"
    } else {
        panic!("Unclassified parser type: {id:?}");
    }
}

fn tokens(arg: &Arg, value: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(long) = arg.get_long() {
        out.push(format!("--{long}"));
    }
    if arg.get_action().takes_values() {
        out.push(value.into());
    }
    out
}

fn argv(route: &[String], command: &clap::Command, inputs: &Inputs) -> Vec<String> {
    let mut out = route.to_vec();
    for arg in command
        .get_arguments()
        .filter(|arg| arg.get_index().is_none())
    {
        if let Some(values) = inputs.get(arg.get_id().as_str()) {
            out.extend(values.iter().cloned());
        }
    }
    let mut positional: Vec<_> = command
        .get_arguments()
        .filter(|arg| arg.get_index().is_some())
        .collect();
    positional.sort_by_key(|arg| arg.get_index());
    for arg in positional {
        if let Some(values) = inputs.get(arg.get_id().as_str()) {
            if arg.is_last_set() {
                out.push("--".into());
            }
            out.extend(values.iter().cloned());
        }
    }
    out
}

fn case(
    cases: &mut Vec<Value>,
    route: &[String],
    dimension: &str,
    argument: &str,
    args: Vec<String>,
    accepted: bool,
) {
    let result = parse(&args);
    assert_eq!(
        result.is_ok(),
        accepted,
        "{route:?}/{dimension}/{argument}: {args:?}: {result:?}"
    );
    let error_kind = result.err().map(|e| format!("{:?}", e.kind()));
    let native_error_kind = error_kind.as_ref().map(|kind| {
        let mut observed = None;
        for format in ["json", "compact"] {
            let mut context: Vec<String> =
                ["--format", format, "--no-color", "--root", "parser-root"]
                    .into_iter()
                    .map(String::from)
                    .collect();
            if route.join(" ") != "pack create" {
                context.extend(["--output".into(), "parser-response".into()]);
            }
            context.extend(args.iter().cloned());
            let contextual = format!("{:?}", parse(&context).unwrap_err().kind());
            if kind == "DisplayHelpOnMissingArgumentOrSubcommand" {
                assert!(matches!(
                    contextual.as_str(),
                    "DisplayHelpOnMissingArgumentOrSubcommand" | "MissingSubcommand"
                ));
            } else {
                assert_eq!(
                    &contextual, kind,
                    "Unrelated contextual refusal: {context:?}"
                );
            }
            if let Some(previous) = &observed {
                assert_eq!(previous, &contextual);
            }
            observed = Some(contextual);
        }
        observed.unwrap()
    });
    cases.push(json!({"route":route.join(" "),"dimension":dimension,"argument":argument,
        "argv":args,"accepted":accepted,"error_kind":error_kind,"native_error_kind":native_error_kind}));
}

fn visit(
    command: &clap::Command,
    route: &mut Vec<String>,
    leaves: &mut Vec<Value>,
    cases: &mut Vec<Value>,
) {
    let children: Vec<_> = command
        .get_subcommands()
        .filter(|child| !child.is_hide_set() && child.get_name() != "help")
        .collect();
    if !children.is_empty() {
        case(
            cases,
            route,
            "missing_subcommand",
            "command",
            route.clone(),
            false,
        );
        for child in children {
            route.push(child.get_name().into());
            visit(child, route, leaves, cases);
            route.pop();
        }
        return;
    }
    let args: Vec<_> = command
        .get_arguments()
        .filter(|arg| !matches!(arg.get_id().as_str(), "help" | "version"))
        .collect();
    let mut seed = Inputs::new();
    for arg in &args {
        if arg.is_required_set() && !arg.is_global_set() {
            seed.insert(arg.get_id().to_string(), tokens(arg, &sample(arg)));
        }
    }
    // This authored conditional is not exposed by Clap's public reflection API.
    if route.join(" ") == "check run" {
        seed.insert(
            "registered_key".into(),
            vec!["--key".into(), "parser-fixture".into()],
        );
    }
    // Pack's artifact path shares the singular global binding; FromArgMatches
    // still requires it after global propagation. It is not a response file.
    if route.join(" ") == "pack create" {
        seed.insert(
            "output".into(),
            vec!["--output".into(), "parser-fixture".into()],
        );
    }
    case(cases, route, "seed", "", argv(route, command, &seed), true);
    if route.join(" ") == "pack create" {
        let mut inputs = seed.clone();
        inputs.remove("output");
        case(
            cases,
            route,
            "missing_artifact_output",
            "output",
            argv(route, command, &inputs),
            false,
        );
    }
    let mut schema = Vec::new();
    let mut conflict_pairs = BTreeSet::new();
    for arg in &args {
        let id = arg.get_id().as_str();
        let mut seed = seed.clone();
        // A mutation of the optional positional Check key must replace the
        // seeded named-key alternative, rather than fail on an unrelated conflict.
        for candidate in &args {
            if command
                .get_arg_conflicts_with(candidate)
                .iter()
                .any(|other| other.get_id().as_str() == id)
            {
                seed.remove(candidate.get_id().as_str());
            }
        }
        for other in command.get_arg_conflicts_with(arg) {
            seed.remove(other.get_id().as_str());
        }
        let possible: Vec<_> = arg
            .get_possible_values()
            .into_iter()
            .filter(|v| !v.is_hide_set())
            .map(|v| v.get_name().to_string())
            .collect();
        let conflicts: Vec<_> = command
            .get_arg_conflicts_with(arg)
            .into_iter()
            .map(|a| a.get_id().to_string())
            .collect();
        schema.push(json!({"id":id,"long":arg.get_long(),"index":arg.get_index(),"required":arg.is_required_set(),"global":arg.is_global_set(),"last":arg.is_last_set(),"action":format!("{:?}",arg.get_action()),"arity":arg.get_num_args().map(|n|[n.min_values(),n.max_values()]),"type":type_name(arg),"values":possible,"defaults":arg.get_default_values().iter().map(|v|v.to_str().unwrap()).collect::<Vec<_>>(),"conflicts":conflicts}));
        if arg.is_global_set() {
            continue;
        } // Existing global matrix remains authoritative.
        if arg.is_required_set() && seed.contains_key(id) {
            let mut inputs = seed.clone();
            inputs.remove(id);
            case(
                cases,
                route,
                "missing_required",
                id,
                argv(route, command, &inputs),
                false,
            );
        }
        if arg.get_action().takes_values() {
            let mut inputs = seed.clone();
            inputs.insert(id.into(), tokens(arg, ""));
            let accepted = possible.is_empty() && type_name(arg) == "string";
            case(
                cases,
                route,
                "explicit_empty_value",
                id,
                argv(route, command, &inputs),
                accepted,
            );
        }
        if arg.is_last_set() {
            let mut inputs = seed.clone();
            inputs.insert(
                id.into(),
                vec![
                    "--no-color".into(),
                    "--unknown-child-option".into(),
                    "".into(),
                ],
            );
            case(
                cases,
                route,
                "accepted_opaque_raw_payload",
                id,
                argv(route, command, &inputs),
                true,
            );
            let mut values = argv(route, command, &seed);
            values.retain(|v| v != "--");
            case(cases, route, "missing_raw_separator", id, values, false);
        }
        if let Some(long) = arg.get_long() {
            if !arg.get_action().takes_values() {
                let mut inputs = seed.clone();
                inputs.insert(id.into(), vec![format!("--{long}=true")]);
                case(
                    cases,
                    route,
                    "unexpected_flag_value",
                    id,
                    argv(route, command, &inputs),
                    false,
                );
            }
            let mut inputs = seed.clone();
            if arg.get_action().takes_values()
                && arg.get_num_args().is_some_and(|n| n.min_values() > 0)
            {
                // Put the valueless option after ordinary positionals but before
                // the raw '--' collection, so another positional cannot fill it.
                inputs.remove(id);
                let mut values = argv(route, command, &inputs);
                let end = values
                    .iter()
                    .position(|s| s == "--")
                    .unwrap_or(values.len());
                values.insert(end, format!("--{long}"));
                case(cases, route, "missing_value", id, values, false);
            }
            let mut inputs = seed.clone();
            let mut values = tokens(arg, &sample(arg));
            values.extend(values.clone());
            inputs.insert(id.into(), values);
            let append = matches!(arg.get_action(), ArgAction::Append | ArgAction::Count);
            case(
                cases,
                route,
                if append {
                    "accepted_repeat"
                } else {
                    "duplicate"
                },
                id,
                argv(route, command, &inputs),
                append,
            );
        }
        for value in &possible {
            let mut inputs = seed.clone();
            inputs.insert(id.into(), tokens(arg, value));
            case(
                cases,
                route,
                "accepted_enum",
                id,
                argv(route, command, &inputs),
                true,
            );
        }
        if !possible.is_empty() {
            let mut inputs = seed.clone();
            inputs.insert(id.into(), tokens(arg, "invalid-parser-enum"));
            case(
                cases,
                route,
                "invalid_enum",
                id,
                argv(route, command, &inputs),
                false,
            );
        }
        if let Some((ty, min, max)) = numeric(arg) {
            let invalid = if ty.starts_with('f') {
                vec!["invalid-number"]
            } else if ty.starts_with('i') {
                vec![
                    "invalid-number",
                    "999999999999999999999999999999999999",
                    "-999999999999999999999999999999999999",
                ]
            } else {
                vec!["invalid-number", "999999999999999999999999999999999999"]
            };
            for value in invalid {
                let mut inputs = seed.clone();
                inputs.insert(
                    id.into(),
                    vec![format!(
                        "--{}={value}",
                        arg.get_long().expect("numeric option")
                    )],
                );
                case(
                    cases,
                    route,
                    "invalid_numeric",
                    id,
                    argv(route, command, &inputs),
                    false,
                );
            }
            for value in [min, max] {
                let mut inputs = seed.clone();
                inputs.insert(
                    id.into(),
                    vec![format!("--{}={value}", arg.get_long().unwrap())],
                );
                case(
                    cases,
                    route,
                    &format!("accepted_{ty}_boundary"),
                    id,
                    argv(route, command, &inputs),
                    true,
                );
            }
            if ty.starts_with('f') {
                for value in ["NaN", "inf", "-inf"] {
                    let mut inputs = seed.clone();
                    inputs.insert(
                        id.into(),
                        vec![format!("--{}={value}", arg.get_long().unwrap())],
                    );
                    case(
                        cases,
                        route,
                        "accepted_float_nonfinite_parser_only",
                        id,
                        argv(route, command, &inputs),
                        true,
                    );
                }
            }
            if ty.starts_with('u') {
                let mut inputs = seed.clone();
                inputs.insert(id.into(), vec![format!("--{}=-1", arg.get_long().unwrap())]);
                case(
                    cases,
                    route,
                    "negative_unsigned",
                    id,
                    argv(route, command, &inputs),
                    false,
                );
            }
        }
        for other in command.get_arg_conflicts_with(arg) {
            let mut pair = [id, other.get_id().as_str()];
            pair.sort();
            if !conflict_pairs.insert(pair.join("/")) {
                continue;
            }
            for (side, absent) in [(*arg, other), (other, *arg)] {
                let mut inputs = seed.clone();
                inputs.remove(absent.get_id().as_str());
                inputs.insert(side.get_id().to_string(), tokens(side, &sample(side)));
                case(
                    cases,
                    route,
                    "accepted_conflict_alternative",
                    side.get_id().as_str(),
                    argv(route, command, &inputs),
                    true,
                );
            }
            let mut inputs = seed.clone();
            inputs.insert(id.into(), tokens(arg, &sample(arg)));
            inputs.insert(other.get_id().to_string(), tokens(other, &sample(other)));
            case(
                cases,
                route,
                "conflict",
                &format!("{id}/{}", other.get_id()),
                argv(route, command, &inputs),
                false,
            );
        }
    }
    if route.join(" ") == "check run" {
        let mut inputs = seed.clone();
        inputs.remove("registered_key");
        case(
            cases,
            route,
            "missing_conditional",
            "registered_key/key",
            argv(route, command, &inputs),
            false,
        );
        inputs.insert("key".into(), vec!["parser-fixture".into()]);
        case(
            cases,
            route,
            "accepted_conditional",
            "key",
            argv(route, command, &inputs),
            true,
        );
    }
    if !args.iter().any(|a| {
        a.is_last_set()
            || (a.get_index().is_some() && a.get_num_args().is_some_and(|n| n.max_values() > 1))
    }) {
        let mut values = argv(route, command, &seed);
        values.push("unexpected-parser-positional".into());
        // Optional single positionals can consume one additional value.
        let available = args
            .iter()
            .filter(|a| a.get_index().is_some() && !seed.contains_key(a.get_id().as_str()))
            .count();
        values.extend(std::iter::repeat_n(
            "unexpected-parser-positional".into(),
            available,
        ));
        case(cases, route, "extra_positional", "", values, false);
    }
    leaves.push(json!({"route":route.join(" "),"arguments":schema}));
}

#[test]
fn frozen_visible_parser_schema_and_cases() {
    let mut command = cli_args::singular_globals(Cli::command());
    command.build();
    let mut leaves = Vec::new();
    let mut cases = Vec::new();
    visit(&command, &mut Vec::new(), &mut leaves, &mut cases);
    let registry: Value = serde_json::from_str(include_str!(
        "../docs/implementation/evidence/pctx01-admission-registry.json"
    ))
    .unwrap();
    let mut expected: Vec<_> = registry["paths"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["leaf"] == true)
        .map(|p| p["path"].as_str().unwrap())
        .collect();
    expected.sort();
    let mut actual: Vec<_> = leaves
        .iter()
        .map(|p| p["route"].as_str().unwrap())
        .collect();
    actual.sort();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 140);
    assert_eq!(
        cases
            .iter()
            .filter(|c| c["dimension"] == "conflict")
            .count(),
        7,
        "All seven authored conflict pairs must be qualified"
    );
    let report = json!({"scope":"PCTX01-G02-D04 parser-only accepted controls and native refusal cases; globals reuse qualified evidence; producer semantics and required platforms separate","leaves":leaves,"cases":cases});
    if let Some(path) = std::env::var_os("PCTX_TEST_PARSER_SCHEMA") {
        std::fs::write(path, serde_json::to_vec(&report).unwrap()).unwrap();
    } else {
        let saved: Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/docs/implementation/evidence/pctx01-parser-cases.json"
            ))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            report, saved,
            "Regenerate and requalify changed schema/cases explicitly"
        );
    }
}
