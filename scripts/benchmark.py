#!/usr/bin/env python3
"""Free, offline measurements of an actual PCTX binary; never launches models."""
import argparse
import concurrent.futures
import hashlib
import importlib.util
import json
import math
import os
import platform
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
SPEC = REPO / "docs/PCTX-implementation-spec-v0.6-en.md"
sys.dont_write_bytecode = True
MODULE = importlib.util.spec_from_file_location("pctx_corpus", HERE / "generate-corpus.py")
CORPUS = importlib.util.module_from_spec(MODULE)
MODULE.loader.exec_module(CORPUS)
DEFAULT = {"schema_version": 1, "profile": "M", "seed": 1048, "repetitions": 30,
           "context_budget_bytes": 12000, "extract_budget_bytes": 10000,
           "timeout_seconds": 150, "output_fixtures": True,
           "session_workloads": True, "local_git_broker": True,
           "cold_cache_control": "unknown", "reference_environment": {}}
TARGETS = {
    "initial_index": (90000, "maximum", "§18"),
    "incremental_20_files": (3000, "p95", "§18"),
    "metadata_search": (250, "p95", "§18"),
    "matched_symbol_search": (700, "p95", "§18"),
    "body_scan": (2000, "p95", "§18"),
    "strict_build": (15000, "p95", "§18"),
}


def sha_file(path):
    hasher = hashlib.sha256()
    with Path(path).open("rb") as source:
        for part in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(part)
    return hasher.hexdigest()


def run_process(argv, env, timeout):
    """Bound saved output while draining both pipes; wait4 measures this child only."""
    start = time.perf_counter_ns()
    proc = subprocess.Popen(argv, env=env, stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            start_new_session=os.name == "posix")
    buffers = [bytearray(), bytearray()]
    sizes = [0, 0]
    caps = [4 * 1024**2, 1024**2]
    def drain(index, pipe):
        try:
            for chunk in iter(lambda: pipe.read(65536), b""):
                sizes[index] += len(chunk)
                room = caps[index] - len(buffers[index])
                if room > 0:
                    buffers[index].extend(chunk[:room])
        finally:
            pipe.close()
    readers = [threading.Thread(target=drain, args=(i, pipe), daemon=True)
               for i, pipe in enumerate((proc.stdout, proc.stderr))]
    for reader in readers:
        reader.start()
    observed = {}
    def wait():
        if hasattr(os, "wait4"):
            _, status, usage = os.wait4(proc.pid, 0)
            proc.returncode = os.waitstatus_to_exitcode(status)
            observed["peak_rss_bytes"] = int(usage.ru_maxrss * (1 if platform.system() == "Darwin" else 1024)) or None
            observed["rss_source"] = "wait4.ru_maxrss_direct_child"
        else:
            proc.wait()
            observed["peak_rss_bytes"] = None
            observed["rss_source"] = "unavailable"
    waiter = threading.Thread(target=wait, daemon=True)
    waiter.start()
    waiter.join(timeout)
    timed_out = waiter.is_alive()
    if timed_out:
        if os.name == "posix":
            try:
                os.killpg(proc.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        else:
            proc.terminate()
        waiter.join(2)
        if waiter.is_alive():
            if os.name == "posix":
                try:
                    os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            else:
                proc.kill()
            waiter.join(5)
    for reader in readers:
        reader.join(2)
    result = {"elapsed_ms": (time.perf_counter_ns() - start) / 1e6,
              "returncode": proc.returncode, "timed_out": timed_out,
              "stdout_bytes": sizes[0], "stderr_bytes": sizes[1],
              "capture_complete": not any(t.is_alive() for t in readers) and all(n <= cap for n, cap in zip(sizes, caps)),
              **observed}
    try:
        result["json"] = json.loads(buffers[0]) if result["capture_complete"] else None
    except (ValueError, UnicodeError):
        result["json"] = None
    result["stderr_sha256"] = hashlib.sha256(buffers[1]).hexdigest()
    result["stdout_sha256"] = hashlib.sha256(buffers[0]).hexdigest()
    return result


def summary(samples):
    values = sorted(s["elapsed_ms"] for s in samples)
    def quantile(fraction):
        return values[max(0, math.ceil(len(values) * fraction) - 1)] if values else None
    return {"samples": len(values), "p50_ms": quantile(.5), "p95_ms": quantile(.95),
            "max_ms": max(values) if values else None,
            "errors": sum(s["returncode"] != 0 or s["json"] is None or s["timed_out"] for s in samples),
            "peak_rss_bytes": max((s["peak_rss_bytes"] for s in samples if s.get("peak_rss_bytes") is not None), default=None)}


def result(status, value=None, reason=None, **details):
    return {"status": status, "value": value, "reason": reason, **details}


def canonical_body(value):
    """Compare semantic data, keeping revisions/omissions/budgets; envelope excluded."""
    return json.dumps(value.get("data"), sort_keys=True, ensure_ascii=False, separators=(",", ":"))


class Harness:
    def __init__(self, cli, root, data_dir, config):
        self.cli, self.root, self.config = cli, root, config
        self.env = os.environ.copy()
        self.env["PCTX_DATA_DIR"] = str(data_dir)
        self.env["PCTX_ACTOR"] = "owner"
        self.samples = []

    def call(self, args, name="setup", expected=(0,)):
        sample = run_process([str(self.cli), "--root", str(self.root), "--format", "json", *args],
                             self.env, self.config["timeout_seconds"])
        sample["workload"] = name
        # Synthetic descriptors contain no credentials. Absolute binary paths are omitted.
        sample["arguments"] = [arg if not Path(arg).is_absolute() else "<explicit executable>" for arg in args]
        sample["expected_exit_codes"] = list(expected)
        self.samples.append(sample)
        return sample

    @staticmethod
    def data(sample):
        return (sample.get("json") or {}).get("data") or {}

    def repeat(self, args, name):
        self.call(args, name + "_warmup")
        return [self.call(args, name) for _ in range(self.config["repetitions"])]

    def trust_and_run(self, executable, arguments, expected=(0,)):
        argv = [str(executable), *(arguments if isinstance(arguments, list) else [arguments])]
        plan = self.call(["trust", "plan", "--", *argv])
        fingerprint = self.data(plan).get("fingerprint")
        if not fingerprint:
            return None
        bound = self.call(["trust", "add", "--expect-hash", fingerprint, "--", *argv])
        if bound["returncode"] != 0:
            return None
        return self.call(["run", "--budget-bytes", "8192", "--execution-timeout-ms", "10000", "--", *argv], "output_capture", expected=expected)


def output_cases(harness):
    executable = shutil.which("cat")
    if executable is None:
        return result("unknown", reason="Explicit noninteractive cat executable unavailable")
    cases = json.loads((REPO / "tests/evaluation/output-cases.json").read_text())["cases"]
    inputs = {
        "tiny": b"completed\n",
        "success": b"progress: 10%\n" * 40000 + b"completed 40000 records\n",
        "failures": b"progress: 10%\n" * 15000 + b"error BENCH_DIAG_001 failed\nwarning BENCH_DIAG_002 retained\n",
        "malformed": b'{"invalid report":\nerror BENCH_DIAG_003 malformed\n',
        "unicode_cr": "error BENCH_DIAG_004 retained\nprogress: 20%\rprogress: 40%\nUnicode: π 😀\n".encode(),
        "large_record": b"x" * (300 * 1024) + b"\n",
        "binary": b"\xff\xfe\x00 binary omitted\n",
        "missing": b"",
    }
    rows = []
    # Added after normative performance samples; these files do not change corpus denominators.
    (harness.root / "benchmark-output").mkdir()
    for case in cases:
        payload = inputs[case["kind"]]
        relative = "benchmark-output/" + case["id"] + ".txt"
        # The historical "missing" label selects a no-input nonzero fixture.
        # A missing path or binary corpus file can prevent manifest validation
        # before spawn, which is not a child-exit observation.
        nonzero_fixture = case["kind"] == "missing"
        input_path = harness.root / relative
        if not nonzero_fixture:
            input_path.write_bytes(payload)
        try:
            captured = harness.trust_and_run(
                Path(executable).resolve(),
                ["--invalid-option"] if nonzero_fixture else [relative],
                expected=(1, 2) if nonzero_fixture else (0,))
        finally:
            # Failed trust (especially binary UTF-8 refusal) must not poison
            # the next workload's source inventory. Never retain raw inputs.
            input_path.unlink(missing_ok=True)
        if captured is None:
            rows.append({"case": case["id"], "status": "unknown", "reason": "Execution trust not available"})
            continue
        data = harness.data(captured)
        output_id = data.get("output_id")
        if not output_id:
            rows.append({"case": case["id"], "status": "miss", "reason": "No observed execution artifact"})
            continue
        retrieval = harness.call(["output", "show", output_id, "--view", "full", "--lines", "1:1000"], "output_retrieval")
        found = None
        if case["required_diagnostic"]:
            found = harness.call(["output", "find", output_id, "--literal", case["required_diagnostic"]], "diagnostic_retrieval")
        if (harness.root / relative).exists():
            (harness.root / relative).unlink()
        repeated = harness.call(["output", "show", output_id, "--view", "full", "--lines", "1:1000"], "output_retrieval_repeat")
        views = [captured, retrieval, repeated] + ([found] if found else [])
        emitted = sum(v["stdout_bytes"] for v in views)
        baseline = data.get("redacted_bytes")
        row = {"case": case["id"], "status": "measured", "baseline_semantics": "redacted_uncompressed",
               "source_input_bytes": len(payload), "redacted_baseline_bytes": baseline,
               "compact_emitted_bytes": captured["stdout_bytes"],
               "retrieval_emitted_bytes": emitted - captured["stdout_bytes"],
               "total_emitted_bytes": emitted,
               "net_saved_bytes": baseline - emitted if isinstance(baseline, int) else None,
               "net_reduction_fraction": (baseline - emitted) / baseline if baseline else None,
               "input_stage": data.get("input_stage"), "child_exit_code": data.get("child_exit_code"),
               "child_exit_preserved": (isinstance(data.get("child_exit_code"), int) and data["child_exit_code"] != 0)
                   if nonzero_fixture else data.get("child_exit_code") == 0,
               "fixture_semantics": "cat_invalid_option_no_input" if nonzero_fixture else case["kind"],
               "expected_nonzero": nonzero_fixture,
               "expected_exact_child_exit": None if nonzero_fixture else 0,
               "capture_complete": data.get("capture_complete"), "parse_status": data.get("parse_status"),
               "omitted_bytes": data.get("omitted_bytes"), "provider_receipt": "unknown"}
        row["reread_same_records"] = harness.data(retrieval).get("records") == harness.data(repeated).get("records")
        row["artifact_identity_preserved"] = all(harness.data(v).get("execution_id", data.get("execution_id")) == data.get("execution_id") for v in views)
        row["source_removed_before_repeated_reread"] = not nonzero_fixture
        row["source_reread_check_applicable"] = not nonzero_fixture
        row["reread_succeeded_without_source"] = repeated["returncode"] == 0 if not nonzero_fixture else None
        row["diagnostic_found"] = bool(harness.data(found).get("records")) if found else None
        # Absence of a fabricated pass is measured; successful diagnosis/fix by models is not.
        row["presentation_not_test_pass"] = data.get("task_completion") == "not_evaluated" and data.get("test_result") != "passed"
        rows.append(row)
    verbose = [row for row in rows if row["case"] in ("verbose_success", "multiple_failures") and isinstance(row.get("redacted_baseline_bytes"), int)]
    baseline_total = sum(row["redacted_baseline_bytes"] for row in verbose)
    emitted_total = sum(row["total_emitted_bytes"] for row in verbose)
    aggregate = {"redacted_baseline_bytes": baseline_total, "total_emitted_bytes": emitted_total,
                 "net_saved_bytes": baseline_total - emitted_total,
                 "net_reduction_fraction": (baseline_total - emitted_total) / baseline_total if baseline_total else None,
                 "supported_parser_bundle": all(row.get("parse_status") == "complete" for row in verbose) if verbose else None,
                 "target_reduction_fraction": .5,
                 "mandatory_diagnostic_misses": sum(row.get("diagnostic_found") is False for row in rows),
                 "observed_presentation_false_passes": sum(row.get("presentation_not_test_pass") is False for row in rows if row.get("status") == "measured"),
                 "model_diagnostic_fix_success_rate": None}
    aggregate["normative_status"] = "unknown" if len(verbose) != 2 or not aggregate["supported_parser_bundle"] else "met" if aggregate["net_reduction_fraction"] >= .5 and not aggregate["mandatory_diagnostic_misses"] and not aggregate["observed_presentation_false_passes"] else "miss"
    return result("measured", rows, aggregate=aggregate, tokenizer_tokens=None, provider_cost=None,
                  diagnostic_fix_success_rate=None,
                  reason="Emitted bytes include repeated original retrieval; model receipt is unobserved")


def sessions(h, mission):
    # This workload measures a declared single-symbol task, rather than making
    # every source in the corpus a required receipt under the same byte limit.
    scope = [mission["expected_paths"][0]]
    budget = h.config["context_budget_bytes"]
    fixture = {"scope": scope, "query": mission["query"],
               "expected_symbol": mission["expected_symbol"], "budget_bytes": budget,
               "purpose": "five sessions receive one declared symbol task and acknowledge unchanged context"}
    task_file = h.root / "benchmark-task.json"
    task_file.write_text(json.dumps({"schema_version": 1,
        "title": "Inspect " + mission["expected_symbol"],
        "description": "Inspect the declared symbol in " + scope[0] + "; observation-only coordination fixture.",
        "scope": scope, "acceptance": [], "checks": []}))
    task = h.data(h.call(["task", "create", "--from-file", str(task_file)])).get("task_id")
    if not task:
        return result("unknown", reason="Could not create isolated task", fixture=fixture)
    rows = []
    session = context_id = None
    for i in range(5):
        name = f"benchmark-agent-{i}"
        h.call(["agent", "register", "--name", name])
        attached = h.call(["session", "attach", "--agent", name, "--runtime", "manual"])
        session = h.data(attached).get("session_id")
        if not session:
            rows.append({"status": "miss", "reason": "No session attachment"})
            continue
        full = h.call(["context", "get", "--task-id", task, "--session", session, "--mode", "full", "--budget-bytes", str(budget)])
        context_id = h.data(full).get("context_id")
        if not context_id:
            rows.append({"status": "miss", "reason": "No full context",
                         "error_codes": [e.get("code") for e in (full.get("json") or {}).get("errors", [])],
                         "full_emitted_bytes": full["stdout_bytes"], "budget_bytes": budget})
            continue
        h.call(["context", "ack", context_id, "--session", session, "--epoch", "1"])
        delta = h.call(["context", "get", "--task-id", task, "--session", session, "--mode", "delta", "--since", context_id, "--budget-bytes", str(budget)])
        body = h.data(delta)
        rows.append({"status": "met" if (body.get("unchanged") is True or body.get("unchanged_count", 0) > 0) and not any(body.get(k) for k in ("added", "changed", "removed", "invalidated")) else "miss",
                     "full_emitted_bytes": full["stdout_bytes"], "delta_emitted_bytes": delta["stdout_bytes"],
                     "local_saved_bytes": full["stdout_bytes"] - delta["stdout_bytes"]})
    # New epochs must not inherit prior acknowledgments.
    if session is None or context_id is None:
        return result("unknown", rows, reason="No validated session baseline", fixture=fixture)
    boundary = h.call(["session", "boundary", "--session", session, "--reason", "benchmark-compact"])
    rejected = h.call(["context", "get", "--task-id", task, "--session", session, "--mode", "delta", "--since", context_id, "--budget-bytes", str(budget)], expected=(9,))
    error_codes = [e.get("code") for e in (rejected.get("json") or {}).get("errors", [])]
    return result("measured", rows, fixture=fixture, compact_baseline_rejected="BASELINE_MISMATCH" in error_codes,
                  boundary_applied=boundary["returncode"] == 0,
                  recovery_tokens_to_first_valid_action=None, model_wakes=None,
                  reason="Local packet correctness/bytes only; no model success or quota measurement")


def broker(h):
    if not shutil.which("git"):
        return result("unknown", reason="Git executable unavailable")
    before = h.data(h.call(["cache", "stats"])).get("upstream_refreshes")
    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:
        observed = list(pool.map(lambda _: h.call(["repo", "status", "--fields", "branch,dirty,head"], "five_process_shared_query"), range(5)))
    after = h.data(h.call(["cache", "stats"])).get("upstream_refreshes")
    revisions = {h.data(row).get("source_revision") for row in observed}
    scope = {h.data(row).get("permission_scope") for row in observed}
    count = after - before if isinstance(before, int) and isinstance(after, int) else None
    supported = all(h.data(row).get("source_status") in ("ok_nonempty", "ok_empty") for row in observed)
    return result("met" if supported and count == 1 and len(revisions) == len(scope) == 1 else "miss" if supported else "unknown",
                  count, source_revisions=list(revisions), permission_scopes=list(scope),
                  external_issue_calls=None, reason="Five actual CLI processes; local Git only")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True, help="Existing compiled executable; never builds")
    parser.add_argument("--config", type=Path, default=REPO / "tests/evaluation/default.json")
    parser.add_argument("--profile", choices=CORPUS.PROFILES)
    parser.add_argument("--repetitions", type=int)
    parser.add_argument("--smoke", action="store_true", help="100 files/51200 bytes/one repetition; never normative acceptance")
    parser.add_argument("--output", type=Path, required=True, help="New evidence directory; never overwrites")
    parser.add_argument("--keep-corpus", action="store_true", help="Retain private temporary inputs for investigation")
    parser.add_argument("--plan-only", action="store_true", help="Seal evaluation mission without invoking CLI/models")
    args = parser.parse_args()
    config = json.loads(args.config.read_text())
    unknown = set(config) - set(DEFAULT)
    if unknown or config.get("schema_version") != 1:
        parser.error(f"Unsupported config keys/schema: {sorted(unknown)}")
    config = {**DEFAULT, **config}
    if args.profile:
        config["profile"] = args.profile
    if args.repetitions is not None:
        config["repetitions"] = args.repetitions
    if args.smoke:
        config.update(profile="S", repetitions=1)
    if config["profile"] not in CORPUS.PROFILES or not isinstance(config["repetitions"], int) or not 1 <= config["repetitions"] <= 1000:
        parser.error("Invalid profile/repetitions")
    if not 1 <= config["timeout_seconds"] <= 3600:
        parser.error("Invalid timeout")
    if not 3000 <= config["context_budget_bytes"] <= 1024**2 or not 512 <= config["extract_budget_bytes"] <= 1024**2:
        parser.error("Invalid output budgets")
    if config["cold_cache_control"] not in ("unknown", "externally_controlled"):
        parser.error("cold_cache_control requires unknown or externally_controlled")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    mission = json.loads((REPO / "tests/evaluation/live-mission.json").read_text())
    canonical = json.dumps({"config": config, "mission": mission}, sort_keys=True, separators=(",", ":")).encode()
    (output / "mission.json").write_text(json.dumps({"plan_sha256": hashlib.sha256(canonical).hexdigest(),
                                                    "config": config, "mission": mission,
                                                    "integrity_is_not_owner_authorization": True}, indent=2) + "\n")
    if args.plan_only:
        print(json.dumps({"status": "plan_only", "model_launch": False, "paid_calls": 0}))
        return
    cli = args.cli.resolve(strict=True)
    if not cli.is_file() or not os.access(cli, os.X_OK):
        parser.error("CLI must be an existing executable")
    private_tmp = Path("/private/tmp") if Path("/private/tmp").is_dir() else Path(tempfile.gettempdir()).resolve()
    workspace = Path(tempfile.mkdtemp(prefix=".pctxbench-", dir=private_tmp)).resolve()
    os.chmod(workspace, 0o700)
    report = {"schema_version": 1, "status": "running", "config": config,
              "specification_sha256": sha_file(SPEC), "binary_sha256": sha_file(cli),
              "binary_name": cli.name, "started_at_unix": time.time(),
              "platform": {"system": platform.system(), "release": platform.release(),
                           "machine": platform.machine(), "cpu_logical_count": os.cpu_count(),
                           "memory_bytes": None, "filesystem": None, "storage_type": None,
                           "network": "not_used", "python": platform.python_version()},
              "limits": {"model_calls": 0, "provider_tokens": None, "provider_cost": None,
                         "subscription_quota": None, "context_selection_only_ms": None,
                         "reference_hardware_equivalent": None,
                         "real_task_success_noninferiority": None}}
    h = None
    try:
        generation_start = time.perf_counter()
        labels = CORPUS.generate(workspace / "project", config["profile"], config["seed"],
                                 100 if args.smoke else None, 51200 if args.smoke else None)
        report["corpus_generation_seconds"] = time.perf_counter() - generation_start
        report["corpus"] = {k: v for k, v in labels.items() if k not in ("files", "queries", "aliases")}
        (output / "corpus-labels.json").write_text(json.dumps(labels, ensure_ascii=False, indent=2) + "\n")
        root = workspace / "project"
        h = Harness(cli, root, workspace / "private-data", config)
        version = run_process([str(cli), "--version"], h.env, config["timeout_seconds"])
        report["binary_version_output_sha256"] = version["stdout_sha256"]
        initialized = h.call(["init"])
        if initialized["returncode"] != 0:
            raise RuntimeError("Actual CLI initialization failed; inspect samples")
        glossary = root / ".pctx/glossary.toml"
        glossary.write_text("[aliases]\n" + "\n".join(f"{json.dumps(key, ensure_ascii=False)} = {json.dumps(value)}" for key, value in labels["aliases"].items()) + "\n")
        first = h.call(["index", "update"], "initial_index")
        report["initial_index"] = summary([first])
        report["initial_index"]["indexed_files_observed"] = h.data(first).get("files")
        report["initial_index"]["skipped_observed"] = h.data(first).get("skipped")
        report["cold_cache"] = {"status": "externally_controlled" if config["cold_cache_control"] == "externally_controlled" else "unknown",
                                "new_index_and_new_process": True,
                                "os_cache_eviction_performed": False,
                                "reason": "Generation writes warm OS pages; new process/index is not a cold filesystem cache"}
        exact = next(q for q in labels["queries"] if q["category"] == "exact_identifier")
        report["warm"] = {}
        for name, command in [
            ("metadata_search", ["find", exact["query"], "--kind", "symbol", "--freshness", "off", "--limit", "20"]),
            ("matched_symbol_search", ["find", exact["query"], "--kind", "symbol", "--freshness", "matched", "--limit", "20"]),
            ("body_scan", ["find", "NotPresentInCorpus", "--kind", "text", "--limit", "20"]),
            ("strict_build", ["build", "--task", exact["query"], "--seed", exact["expected_paths"][0], "--budget-bytes", str(config["context_budget_bytes"])]),
        ]:
            observed = h.repeat(command, name)
            report["warm"][name] = summary(observed)
            if name == "strict_build":
                bodies = [canonical_body(s["json"]) for s in observed if s["json"] and s["returncode"] == 0]
                report["context_checks"] = {
                    "successful_samples": len(bodies), "deterministic_body": len(set(bodies)) == 1 if len(bodies) >= 2 else None,
                    "all_outputs_within_budget": all(s["stdout_bytes"] <= config["context_budget_bytes"] for s in observed),
                    "budget_bytes": config["context_budget_bytes"], "source_bytes_baseline": labels["files"][next(i for i,r in enumerate(labels["files"]) if r["path"] == exact["expected_paths"][0])]["bytes"],
                    "mean_context_emitted_bytes": sum(s["stdout_bytes"] for s in observed) / len(observed)}
        records = {row["path"]: row for row in labels["files"]}
        query_rows = []
        for query in labels["queries"]:
            observed = h.call(["find", query["query"], "--kind", query["kind"], "--limit", "20", "--freshness", "matched"], "labeled_search")
            items = h.data(observed).get("items")
            measured = observed["returncode"] == 0 and isinstance(items, list)
            paths = [item.get("path") for item in items] if isinstance(items, list) else []
            expected = query["expected_paths"]
            hits = set(paths) & set(expected)
            wrong_hash = [item.get("path") for item in items or [] if item.get("path") in records and item.get("file_hash") != records[item["path"]]["sha256"]]
            rank = next((i + 1 for i, path in enumerate(paths) if path in expected), None)
            query_rows.append({"id": query["id"], "category": query["category"], "measured": measured,
                               "expected_paths": expected, "observed_paths": paths,
                               "recall_at_20": len(hits) / len(expected) if expected and measured else None,
                               "reciprocal_rank": 1 / rank if rank and measured else 0 if expected and measured else None,
                               "empty_correct": not paths if not expected and measured else None,
                               "wrong_hash_paths": wrong_hash, "elapsed_ms": observed["elapsed_ms"]})
        report["search_queries"] = query_rows
        report["search_metrics"] = {}
        for category in sorted({q["category"] for q in query_rows}):
            group = [q for q in query_rows if q["category"] == category]
            recalls = [q["recall_at_20"] for q in group if q["recall_at_20"] is not None]
            reciprocal = [q["reciprocal_rank"] for q in group if q["reciprocal_rank"] is not None]
            report["search_metrics"][category] = {"queries": len(group), "measured": sum(q["measured"] for q in group),
                "recall_at_20": sum(recalls) / len(recalls) if recalls else None,
                "mrr": sum(reciprocal) / len(reciprocal) if reciprocal else None,
                "wrong_hash_results": sum(len(q["wrong_hash_paths"]) for q in group)}
        seed = root / exact["expected_paths"][0]
        outline = h.call(["outline", exact["expected_paths"][0]], "outline")
        symbols = [sym for f in h.data(outline).get("files", []) for sym in f.get("symbols", [])]
        chosen = next((sym for sym in symbols if sym.get("name") == exact["expected_symbol"]), None)
        symbol_check = {"status": "unknown", "reason": "No expected exact symbol"}
        if chosen:
            observed = h.call(["read", "--symbol", chosen["id"]], "symbol_read")
            value = h.data(observed)
            span = value.get("range", {})
            original = seed.read_bytes()
            reference = original[span.get("start_byte", 0):span.get("end_byte", 0)].decode()
            symbol_check = {"status": "met" if observed["returncode"] == 0 and value.get("file_hash") == hashlib.sha256(original).hexdigest() and value.get("text") == reference and exact["expected_symbol"] in reference else "miss",
                            "range": span, "expected_symbol": exact["expected_symbol"]}
        report["symbol_read"] = symbol_check
        extracted = h.call(["extract", "--location", exact["expected_paths"][0] + ":1", "--budget-bytes", str(config["extract_budget_bytes"])], "extract")
        items = h.data(extracted).get("items", [])
        report["extract_check"] = {"status": "met" if extracted["returncode"] == 0 and any(exact["expected_symbol"] in (item.get("content") or "") for item in items) and extracted["stdout_bytes"] <= config["extract_budget_bytes"] else "miss",
                                   "source_hash_matches": all(item.get("file_hash") == records.get(item.get("path"), {}).get("sha256") for item in items),
                                   "within_budget": extracted["stdout_bytes"] <= config["extract_budget_bytes"]}
        unsupported = next(q for q in labels["queries"] if q["category"] == "unsupported_language")
        unsupported_result = h.call(["outline", unsupported["expected_paths"][0]], "unsupported_outline")
        report["unsupported_not_empty"] = any(row.get("coverage", {}).get("status") == "unsupported" for row in h.data(unsupported_result).get("files", []))
        modified = []
        for repetition in range(config["repetitions"]):
            for row in labels["files"][:20]:
                path = root / row["path"]
                content = bytearray(path.read_bytes())
                # Same byte count; changes remain in comment bodies and retain declarations.
                position = row["header_bytes"] + 3
                replacement = f"{repetition:08x}".encode()
                if content[position:position+8] == replacement:
                    replacement = b"f" + replacement[1:]
                content[position:position+8] = replacement
                path.write_bytes(content)
            observed = h.call(["index", "update"], "incremental_20_files")
            observed["source_files_changed"] = 20
            modified.append(observed)
        report["incremental_20_files"] = summary(modified)
        # Stale exact symbol may not return mismatched code after a real same-size edit.
        if chosen:
            data = bytearray(seed.read_bytes()); data[-4] = ord("z") if data[-4] != ord("z") else ord("y"); seed.write_bytes(data)
            stale = h.call(["read", "--symbol", chosen["id"]], "stale_symbol_read", expected=(4,))
            report["stale_symbol_rejected"] = stale["returncode"] == 4 and any(error.get("code") == "STALE_INDEX" for error in (stale["json"] or {}).get("errors", []))
        garbage_collection = h.call(["index", "gc", "--apply"], "index_retention_cleanup")
        report["index_retention_cleanup"] = {"returncode": garbage_collection["returncode"], "data": h.data(garbage_collection)}
        measured_db_paths = list((workspace / "private-data").rglob("index.sqlite3"))
        report["index_bytes_before_later_functional_workloads"] = sum(path.stat().st_size + (Path(str(path)+"-wal").stat().st_size if Path(str(path)+"-wal").exists() else 0) for path in measured_db_paths) if measured_db_paths else None
        if config["local_git_broker"]:
            # A new isolated unborn Git repository: fixed argv, no hooks/network.
            git = shutil.which("git")
            if git:
                git_env = h.env.copy(); git_env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
                subprocess.run([git, "-c", "core.hooksPath=" + os.devnull, "init", "-q", str(root)], env=git_env, check=True, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            report["shared_query"] = broker(h)
        if config["session_workloads"]:
            report["sessions"] = sessions(h, exact)
        if config["output_fixtures"]:
            report["output"] = output_cases(h)
        report["measurements"] = {}
        eligible = labels["normative_shape"] and config["profile"] == "M" and config["repetitions"] >= 30
        for name, (limit, statistic, reference) in TARGETS.items():
            observation = report.get(name) or report["warm"].get(name, {})
            value = observation.get("max_ms" if statistic == "maximum" else "p95_ms")
            report["measurements"][name] = result("unknown" if value is None else "met" if value <= limit and not observation.get("errors") else "miss", value,
                target_ms=limit, statistic=statistic, specification=reference,
                normative_corpus_and_repetition_eligible=eligible,
                reference_hardware_equivalent="unknown", scope="observed_host_not_release_certification")
        peaks = [sample["peak_rss_bytes"] for sample in h.samples if sample.get("peak_rss_bytes") is not None]
        peak = max(peaks) if peaks else None
        report["measurements"]["cli_peak_memory"] = result("unknown" if peak is None else "met" if peak <= 512 * 1024**2 else "miss", peak, target_bytes=512 * 1024**2, includes_harness=False, includes_descendant_jobs=False)
        # Actual index path is located privately; report size, never the absolute path.
        db_bytes = report.get("index_bytes_before_later_functional_workloads")
        report["measurements"]["active_db_and_wal"] = result("unknown" if db_bytes is None else "met" if db_bytes <= 400 * 1024**2 else "miss", db_bytes, target_bytes=400 * 1024**2, retained_generations="not_independently_verified")
        exact_metrics = report["search_metrics"]["exact_identifier"]
        report["measurements"]["exact_identifier_file_recall_at_20"] = result("unknown" if exact_metrics["measured"] != exact_metrics["queries"] else "met" if exact_metrics["recall_at_20"] == 1 and exact_metrics["wrong_hash_results"] == 0 else "miss", exact_metrics["recall_at_20"], target=1.0, specification="§19", scope="labeled_synthetic_files_not_all_symbols")
        deterministic = report["context_checks"]["deterministic_body"]
        report["measurements"]["AC15_context_determinism"] = result("unknown" if deterministic is None else "met" if deterministic and report["context_checks"]["successful_samples"] == config["repetitions"] else "miss", deterministic)
        report["measurements"]["context_selection_serialization_only"] = result("unknown", reason="CLI strict build combines hashing/selection/serialization; no separate800ms observation")
        report["measurements"]["wrapper_additional_latency"] = result("unknown", reason="No matched direct-child timing subtraction;50ms target not inferred from total latency")
        report["measurements"]["masked_1mib_postprocessing"] = result("unknown", reason="No isolated postprocessing instrumentation;100ms target not inferred")
        report["measurements"]["model_input_token_reduction"] = result("unknown", reason="No provider usage or model execution")
        report["measurements"]["model_task_success_noninferiority"] = result("unknown", reason="No real task/model trials;AC82 not verified by bytes alone")
        report["measurements"]["new_session_recovery_token_reduction"] = result("unknown", reason="Local packet bytes are not observed recovery tokens")
        report["measurements"]["idle_model_wakes"] = result("unknown", reason="No elapsed idle scheduler/model transport experiment")
        report["status"] = "measured_not_complete_product_validation"
    except Exception as error:
        report["status"] = "harness_failed"
        report["error"] = {"type": type(error).__name__, "message": "Harness stopped; inspect captured sample states. No success is inferred."}
    finally:
        report["finished_at_unix"] = time.time()
        if h is not None:
            (output / "samples.json").write_text(json.dumps(h.samples, ensure_ascii=False, indent=2) + "\n")
        report["temporary_corpus_retained"] = args.keep_corpus
        (output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
        table = ["# PCTX Offline Evaluation", "", "Status: `" + report["status"] + "`", "", "This report measures local CLI behavior and emitted bytes. Provider tokens, billed cost, subscription quota, and model task success remain unknown.", "", "| Metric | Observed | Target | Result |", "| --- | --- | --- | --- |"]
        for name, metric in report.get("measurements", {}).items():
            table.append(f"| {name} | {metric['value']} | {metric.get('target_ms', metric.get('target_bytes', 'unknown'))} | {metric['status']} |")
        table.extend(["", "Observed targets are not release certification: corpus/repetition eligibility and hardware equivalence are separately reported. See report.json and samples.json for errors and coverage."])
        (output / "report.md").write_text("\n".join(table) + "\n")
        if not args.keep_corpus:
            shutil.rmtree(workspace)
    print(json.dumps({"status": report["status"], "report": str(output / "report.json")}))
    if report["status"] == "harness_failed":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
