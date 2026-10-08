//! Explicit command contracts: new leaves require review of their actual effects.
use clap::Command;

fn effect(path: &str) -> Option<&'static str> {
    Some(match path {
        "init" => "write: project configuration and workspace binding",
        "status"
        | "doctor"
        | "read"
        | "changes"
        | "trace"
        | "impact"
        | "refs"
        | "checkpoint list"
        | "cache stats"
        | "resource status"
        | "savings report"
        | "savings opportunities"
        | "task list"
        | "task show"
        | "agent list"
        | "agent show"
        | "check list"
        | "check show"
        | "role list"
        | "decision show"
        | "owner queue"
        | "quota report"
        | "quota plan"
        | "quota reconcile"
        | "schedule list"
        | "runner resource-status"
        | "runner helper-status" => "read: local state and observations",
        "find" | "query" | "outline" => {
            "read: source/metadata; --freshness strict may write derived index generations"
        }
        "extract" => "write: refresh derived index before source inspection",
        "build" => "write: refresh derived index and document lifecycle metadata",
        "index update" | "index rebuild" => "write: derived index generations",
        "index gc" => {
            "read by default or --dry-run; --apply writes by deleting eligible generations"
        }
        "checkpoint create" | "checkpoint delete" => "write: checkpoint manifests and registry",
        "handoff create" | "handoff update" => "write: explicit handoff documents",
        "handoff show" => "read: handoff; --validate inspects freshness",
        "board" => "read: board; --watch continues until interrupted; NDJSON emits event records",
        "activity" => {
            "read: activity; --follow continues until interrupted; NDJSON emits event records"
        }
        "repo status" => "read: fixed native Git queries; write: derived broker cache",
        "job cancel" | "runner job-cancel" | "runner helper-cancel" => {
            "execute: cancel owned process (queued helper may only record intent); write: cancellation state"
        }
        "run" | "check run" | "runner check-run" => {
            "execute: trusted child argv; write: captured output and execution evidence"
        }
        "output show" | "output find" | "output render" => {
            "read: saved output; write: successful delivery metering; never reruns the original command"
        }
        "trust plan" | "check plan" | "runner check-plan" => {
            "read: fingerprints/execution proposal; does not launch a child"
        }
        "trust add" | "runner trust" => "write: trusted binding; does not launch a child",
        "task create"
        | "task edit"
        | "task ready"
        | "task assign"
        | "task reassign"
        | "task start"
        | "task claim"
        | "task block"
        | "task pause"
        | "task resume"
        | "task cancel"
        | "task reopen"
        | "task submit"
        | "task review"
        | "task criterion accept" => {
            "write: task, lease or evidence records; does not launch or cancel native processes"
        }
        "task complete" => "write: completion records; --dry-run only reads completion gates",
        "agent register" | "agent report" | "agent heartbeat" => {
            "write: agent metadata and observations"
        }
        "check begin" | "check record" => "write: check records; does not launch a child",
        "control backup" => "write: explicit control archive",
        "control restore" => {
            "write: control namespace and invalidation; does not restore processes"
        }
        "role pause" | "role resume" => "write: persistent role policy; does not wake a model",
        "policy evaluate" | "policy report-evaluate" | "policy report-fingerprint" => {
            "read: policy evaluation/fingerprints; does not grant authority"
        }
        "policy attest-owner"
        | "policy release"
        | "policy exception-record"
        | "policy exception-revoke" => "write: policy records",
        "decision request" | "decision record" => "write: approval records",
        "message send" | "message ack" | "message resolve" => {
            "write: internal mailbox; no external delivery or model wake"
        }
        "inbox read" => "read: mailbox; no implicit acknowledgement",
        "session attach" | "session suspend" | "session boundary" => "write: session epoch/capsule",
        "session reconcile" => "read: session comparison; does not mutate epochs",
        "context get" => {
            "write: context receipt and selection metadata; no implicit acknowledgement"
        }
        "context ack" => "write: explicit context acknowledgement",
        "quota ingest" | "quota reserve" | "quota release" => {
            "write: quota observations/reservations; no paid provider call"
        }
        "runner helper-request" => {
            "execute and write by default (--mode local); other supported modes write intent without launch"
        }
        "runner helper-release" => {
            "write: release after emptiness proof; does not start or kill processes"
        }
        "runner bridge-guardian" => "execute: private native guardian protocol; internal interface",
        "filter validate" | "filter apply" | "filter explain" => {
            "read: bounded inert preview; does not launch a child"
        }
        "filter test" => "write: private fixture report; does not launch a child",
        "filter activate" => "write: verified filter binding",
        "pack plan" => "write: immutable private plan; context selection may refresh metadata",
        "pack create" => {
            "write: explicit source artifact; --output names the artifact and the response remains on stdout"
        }
        "pack inspect" | "pack verify" => "read: artifact and freshness evidence",
        "adapter claude doctor" => {
            "execute: benign claude --version capability probe; no model call"
        }
        "adapter claude plan" => "write: private installation plan",
        "adapter claude install" | "adapter claude uninstall" => {
            "write: managed local configuration and receipts"
        }
        "adapter claude verify" | "adapter claude protocol-fixture" => {
            "read: offline protocol validation; no live model inference"
        }
        "adapter claude statusline" => "write: imported observations",
        "adapter claude event" => {
            "write: imported session events; --hook accepts stdin only and emits native hook output on stdout; --from-file and --output are unavailable with --hook"
        }
        "schedule plan" => "read: proposal/fingerprints; no saved plan or native registration",
        "schedule inspect" => {
            "read: managed state; --observe-native executes a finite native status query"
        }
        "schedule install" | "schedule uninstall" => {
            "write: managed state; --apply-native executes OS registration/removal"
        }
        "schedule add" | "schedule update" | "schedule pause" | "schedule resume"
        | "schedule remove" | "schedule reconcile" | "schedule recover" => {
            "write: schedule definitions/occurrence/recovery records; does not launch jobs"
        }
        "schedule tick" => "execute: permitted jobs; write: occurrence receipts",
        "schedule run-loop" => {
            "execute: permitted jobs for explicit loop lifetime; write: receipts; --keep-awake requests native assertion"
        }
        "inventory scan" | "inventory profile" | "inventory audit" => {
            "read: inert inventory and proposals"
        }
        _ => return None,
    })
}

pub(super) fn annotate(mut command: Command) -> Command {
    fn visit(command: &mut Command, path: &str) {
        let leaf = !command.has_subcommands();
        let effects = if leaf {
            effect(path).unwrap_or("unclassified: command contract requires review")
        } else {
            "read/write/execute depend on the selected subcommand; see its --help"
        };
        *command = command.clone().after_help(format!(
            "Contract: standard JSON envelope schema 1.0 (payloads retain their own versions).\nEffects: {effects}.\nCommon options: --root, --format, --output, --timeout-ms and --no-color each accept one explicit occurrence across the command path. --output writes the requested output file (pack create: artifact instead of response; streaming routes reject it); project-backed operations may initialize local schema/cache. --help and --version do not access the project."
        ));
        for child in command.get_subcommands_mut() {
            let next = if path.is_empty() {
                child.get_name().to_owned()
            } else {
                format!("{path} {}", child.get_name())
            };
            visit(child, &next);
        }
    }
    visit(&mut command, "");
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn every_declared_leaf_has_an_explicit_effect_contract() {
        fn check(command: &Command, path: &str) {
            if !command.has_subcommands() {
                assert!(effect(path).is_some(), "Missing effect contract: {path}");
            }
            for child in command.get_subcommands() {
                check(
                    child,
                    &if path.is_empty() {
                        child.get_name().to_owned()
                    } else {
                        format!("{path} {}", child.get_name())
                    },
                );
            }
        }
        check(&crate::Cli::command(), "");
    }
}
