# Registered checks and shared resources

Registered checks execute an exact owner-trusted profile through the same supervisor and output store as `pctx run`. Task definitions describe evidence requirements; they cannot grant executable trust. A check result does not complete a task: acceptance, submission, current fingerprints, and independent review still apply. See [local work](work.md).

## Run an isolated check

This example requires `pctx` and Python 3 on `PATH`. It creates a temporary project and private data directory, executes one real assertion, and emits the standard report. It does not install hooks or contact a provider.

```sh
DEMO=$(mktemp -d)
mkdir "$DEMO/project" "$DEMO/data"
cd "$DEMO/project"
export PCTX_DATA_DIR="$DEMO/data"
export PCTX_ACTOR=owner
pctx init

cat > math_fixture.py <<'PY'
def double(value):
    return value * 2
PY
cat > check_fixture.py <<'PY'
import json, time
from math_fixture import double
started = int(time.time())
failed = 0
try:
    assert double(3) == 6
except AssertionError:
    failed = 1
print(json.dumps({
    "schema_version": 1, "check_key": "unit",
    "producer": "isolated-python-fixture", "source": "external_report",
    "exit_code": failed, "tests": 1, "passed": 1 - failed,
    "failed": failed, "errors": 0, "skipped": 0,
    "result": "failed" if failed else "passed",
    "started_at": started, "finished_at": int(time.time())
}))
raise SystemExit(failed)
PY
cat > .pctx/runner.toml <<'TOML'
schema_version = 1
[checks.unit]
argv = ["python3", "check_fixture.py"]
cwd = "."
reporter = "pctx-json-v1"
execution_timeout_ms = 5000
resources = []
heavy = false
script_inputs = ["math_fixture.py"]
env_allowlist = ["CI"]
env = { CI = "true" }
TOML
cat > "$PCTX_DATA_DIR/task.json" <<'JSON'
{
  "schema_version": 1,
  "title": "Verify the isolated doubling fixture",
  "scope": ["math_fixture.py", "check_fixture.py"],
  "acceptance": [{"id": "behavior", "description": "Doubling assertion passes", "evidence_check_keys": ["unit"]}],
  "checks": [{"key": "unit", "kind": "test", "minimum_executed_tests": 1,
              "output_paths": ["__pycache__/**", "reports/**"],
              "allowed_sources": ["runner_observed"]}]
}
JSON
pctx --format json task create --from-file "$PCTX_DATA_DIR/task.json" > "$PCTX_DATA_DIR/created.json"
TASK=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["data"]["task_id"])' "$PCTX_DATA_DIR/created.json")
pctx agent register --name fixture-worker --kind agent
pctx task ready "$TASK"
pctx task assign "$TASK" --agent fixture-worker
pctx --format json task start "$TASK" > "$PCTX_DATA_DIR/run.json"
RUN=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["data"]["run_id"])' "$PCTX_DATA_DIR/run.json")
pctx --format json check plan --task-id "$TASK" --key unit --run "$RUN" > "$PCTX_DATA_DIR/plan.json"
cat "$PCTX_DATA_DIR/plan.json"
```

Inspect the executable, scripts, environment keys, resources, and fingerprint before trusting the plan. Then:

```sh
FINGERPRINT=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["data"]["fingerprint"])' "$PCTX_DATA_DIR/plan.json")
pctx runner trust --key unit --expect-hash "$FINGERPRINT"
pctx --format json check run --task-id "$TASK" --key unit --run "$RUN" > "$PCTX_DATA_DIR/result.json"
cat "$PCTX_DATA_DIR/result.json"
OUT=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["data"]["execution"]["output_id"])' "$PCTX_DATA_DIR/result.json")
pctx output show "$OUT" --view full --stream stdout --lines 1:80
```

The child report's source is a claim. Only the supervised artifact-to-check path promotes it to `runner_observed`; arbitrary `check record` JSON cannot supply that provenance. A test check must execute at least one test even when its configured minimum is zero. Invalid counts, native exit mismatch, missing typed output, timeout, signal, capture loss, and changed inputs cannot produce valid passing evidence. An untyped or failed admission attempt is finalized honestly rather than left running. Report timestamps are Unix seconds.

The task lease defaults to 180 seconds. If it expires while reading the example, explicitly `pctx task resume "$TASK"`, start a new run, and replace `RUN`. Trust does not renew a lease. Changing scripts, declared script inputs, applicable package/build manifests, argv, cwd, environment policy, or executable hashes requires a fresh plan and trust. Environment values must use the bounded nonsecret allowlist: `LANG`, `LC_ALL`, `LC_CTYPE`, `TZ`, or `CI`.

## Resource and memory admission

For a heavy registered profile, set:

```toml
heavy = true
resources = ["heavy-compute"]
resource_backend = "pctx"

[checks.unit.memory]
source = "native"
minimum_available_bytes = 268435456
unknown = "deny"
```

These are entries in the same `[checks.unit]` profile, not a second duplicate table. Known build/test executable names require heavy admission. `heavy-compute` and `docs-publish` share one host slot; `aux-agent` has its own slot. Multi-resource acquisition uses a deterministic order and releases partial acquisitions on admission failure. Resource state defaults to the local data directory's host resource namespace. Operators sharing a host must use the same `PCTX_DATA_DIR` or explicitly agreed `PCTX_HOST_RESOURCE_DIR`; unrelated directories do not coordinate.

Linux observes `MemAvailable`; macOS observes free plus inactive pages as a reclaimable estimate. Neither is a guarantee that a future allocation will succeed. Unknown measurements defer resource admission. The owner can explicitly trust `unknown = "owner_override"`; output discloses use of that override and keeps the measurement unknown. `source = "fixture"` with `fixture_path` is an explicit synthetic test source, labelled as such, not native memory evidence.

```sh
pctx resource status --host current
pctx job cancel JOB-ID
```

Lease expiry and main CLI death never release a living execution group's slot. Cancellation signals the group and requires exit proof. Unknown ownership, inaccessible process identity, and orphaned descendants retain the slot. Status does not delete stale locks or perform TTL takeover.

## A configured legacy mutex

The legacy bridge supports an explicit existing local advisory mutex using `fs2-guardian-v1`:

```toml
resource_backend = "legacy"
[checks.unit.bridge]
protocol = "fs2-guardian-v1"
lock_path = "/absolute/path/to/the-existing-canonical.lock"
```

Use the same existing lock file and advisory locking semantics as the legacy process; never replace or unlink that inode to obtain a lock. The path, protocol, guardian executable/version/hash, and profile are bound by owner trust. A finite per-job guardian acquires and proves the canonical mutex before execution, receives the native child identity, and holds the mutex until the process group exits. It survives main CLI termination. Guardian failure during supervised execution causes a negative child observation and group cancellation.

An absent bridge, unknown protocol, or unavailable native boot/start identity disables this capability. Compatibility with a particular legacy installation must be verified separately in both directions. The isolated regression suite exercises mutual exclusion and parent death; it does not certify arbitrary third-party lock scripts. The hidden guardian command is an internal protocol endpoint, not an operator command.

## Auxiliary provider intents

A cloud or native request records intent without allocating a slot or launching a model:

```sh
pctx runner helper-request --task-id "$TASK" --key unit --run "$RUN" --mode cloud --scope math_fixture.py
pctx runner helper-status HELP-ID
pctx runner helper-cancel HELP-ID
```

Replace `HELP-ID` with the returned helper ID. A registered local provider uses exactly `resources = ["aux-agent"]`, `heavy = false`, and:

```toml
[checks.unit.auxiliary_provider]
kind = "local"
workspace = "/absolute/path/to/an-existing-registered-linked-worktree"
scope = ["math_fixture.py"]
```

The target must be a different registered workspace/root in the same project and coordination, with matching inert Git common-directory and backreference identity. Register it separately before planning; this command does not create or attach worktrees. Trust the resulting profile, then use `runner helper-request ... --mode local`. The explicit provider process is observed before a receipt becomes active, and managed local providers share the single auxiliary slot. External queued intents have no launch capability; an actual external provider needs its own verified admission integration. Release requires the exact output receipt: `pctx runner helper-release HELP-ID --evidence OUT-ID`. Provider output does not complete the task or prove model usage inside an opaque program.

Registered scope is a managed-process policy, not an OS sandbox. Internal trust does not replace host permission. Helper receipts are durable private workspace files and currently excluded from portable control backups. Actual external model/runtime transport and legacy installations need separate live capability verification. Registered supervision currently uses the Unix backend; Windows execution is unavailable rather than reported successful.

Canonical `check plan`, `check run`, `resource status --host current`, and `job cancel` commands call the same application services as their `runner` forms. A check plan may be reviewed before a task run exists; executing a check still requires the current run lease and exact owner trust. Registered output parsers are documented in [parsers](parsers.md); parser summaries alone never supply completion evidence.
