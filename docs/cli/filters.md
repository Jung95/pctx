# Inert output filters

A filter transforms an existing, masked output view. It cannot execute commands, rewrite argv, grant permissions, change native termination, or turn a summary into passing check evidence. `filter apply` previews a supplied file; `output render` reuses a retained execution artifact. Neither reruns the original command.

## Author and preview a filter

Use an initialized disposable project and private `PCTX_DATA_DIR`, as in [the runner example](runner.md). Paths for definitions, input files, and fixtures must remain inside that project. Save this definition as `.pctx/filters/demo.toml`:

```sh
mkdir -p .pctx/filters
cat > .pctx/filters/demo.toml <<'TOML'
schema_version = 1
id = "demo"
version = "1.0.0"
priority = 100

[match]
program = "/usr/bin/printf"
argv_prefix = ["status"]
stream = "both"

[parse]
kind = "lines"

[render]
max_bytes = 8192
keep_head_lines = 6
keep_tail_lines = 6
show_omission_counts = true

[[rules]]
op = "protect"
pattern = '(?i)error|warning|summary'

[[rules]]
op = "drop"
pattern = '^progress'

[[rules]]
op = "deduplicate_exact"
show_count = true
TOML
printf 'progress 1\nwarning W42\nwarning W42\nsummary fixture done\n' > preview.log
pctx filter validate .pctx/filters/demo.toml
pctx filter apply --filter demo --input preview.log --child-exit 0
```

`--child-exit` on a preview is a supplied fixture claim, not an observed execution. The view reports `execution_started = false`, `test_result = "not_evaluated"`, omission counts, and original byte ranges. Exact duplicate records retain their occurrence count and ranges. Error/warning/signal/summary records receive built-in protection; explicit protection takes precedence over a drop or excerpt. If the budget cannot retain all protected records, `protected_omitted` discloses the loss.

Supported rules are strictly bounded:

| Operation | Fields | Effect |
| --- | --- | --- |
| `protect` | `pattern` | Mark matching records for preservation. |
| `drop` | `pattern` | Remove matching unprotected records from the selected stream. |
| `select_fields` | `fields` | Keep named top-level JSON fields on unprotected JSON records. |
| `group_by` | `fields` | Add a grouping label from top-level JSON fields. |
| `deduplicate_exact` | `show_count = true` | Combine exact stream/text duplicates while retaining counts and ranges. |
| `excerpt` | `keep_head_lines`, `keep_tail_lines` | Bound an excerpt while retaining protected records. `bounded_excerpt` is an accepted alias. |

`parse.kind` is `lines` or `json`; JSON parsing is per record. Protected JSON records retain their original fields, including diagnostic locations and severity. Malformed JSON is disclosed as partial when present in the rendered JSON view. Patterns use bounded Rust regular expressions; backreferences, code evaluation, shell templates, unknown keys, and unknown operations are rejected.

Definitions are limited to 64 KiB and 64 rules. Render budgets are 2,048–65,536 bytes, with at most 1,000 combined head/tail lines. Omission accounting and duplicate counts are mandatory. Explicit preview input is limited to 1 MiB; processing also bounds record size/count and runtime. Match selection uses the declared program, argument prefix, and priority. Equal highest priorities fail with `FILTER_AMBIGUOUS`.

## Create all required presentation fixtures

Activation requires all eleven scenarios, an exact expected view hash for each, and unchanged fixture inputs. The following generator creates only synthetic data. The private-key-shaped marker contains no key material and exists solely to exercise masking.

```sh
mkdir -p fixtures/demo
python3 - <<'PY'
import json, pathlib
root = pathlib.Path("fixtures/demo")
specs = [
    ("success", "summary success\n", 0, "exited", ["summary success"], 1),
    ("nonzero", "error E42\n", 1, "exited", ["E42"], 1),
    ("warning-only", "warning W42\n", 0, "exited", ["W42"], 1),
    ("empty", "", 0, "exited", [], 0),
    ("timeout", "warning interrupted\n", 1, "timed_out", ["interrupted"], 1),
    ("malformed", "{broken JSON\n", 1, "exited", [], 0),
    ("large", "progress repeated\n" * 4000, 0, "exited", [], 0),
    ("unicode", "summary λ\n", 0, "exited", ["λ"], 1),
    ("cr-progress", "progress 1\rprogress 2\rsummary done\n", 0, "exited", ["done"], 1),
    ("streams", "warning stdout\n", 1, "exited", ["stdout", "stderr"], 2),
    ("split-secret", "-----BEGIN PRIVATE KEY-----\nSYNTHETIC-NOT-A-KEY\n-----END PRIVATE KEY-----\n", 0, "exited", [], 0),
]
cases = []
for name, body, code, termination, keep, protected in specs:
    filename = name + ".txt"
    (root / filename).write_text(body, encoding="utf-8")
    case = dict(name=name, input=filename, child_exit=code,
                termination=termination, must_keep=keep,
                minimum_protected=protected, expected_view_hash="0" * 64)
    if name == "streams":
        (root / "streams-stderr.txt").write_text("error stderr\n")
        case["stderr"] = "streams-stderr.txt"
    cases.append(case)
(root / "manifest.json").write_text(json.dumps({
    "schema_version": 1, "executable": "/usr/bin/printf", "cases": cases
}, indent=2))
PY
pctx --format json filter test .pctx/filters/demo.toml --fixtures fixtures/demo > "$PCTX_DATA_DIR/demo-first-pass.json"
cat "$PCTX_DATA_DIR/demo-first-pass.json"
```

The initial hashes intentionally differ, so this first pass must not be activated. Review each scenario, preservation requirement, and observed view. For ordinary records, use `filter apply` to inspect the fixture; its preview termination is always `exited`, while the fixture test also exercises `timed_out`. Confirm that the synthetic secret is absent from rendered text and that diagnostics survive drops and excerpts.

After reviewing the expected behavior, explicitly record the reported hashes as golden expectations:

```sh
python3 - "$PCTX_DATA_DIR/demo-first-pass.json" <<'PY'
import json, pathlib, sys
result = json.load(open(sys.argv[1]))["data"]
observed = {case["case"]: case for case in result["cases"]}
manifest = pathlib.Path("fixtures/demo/manifest.json")
suite = json.loads(manifest.read_text())
for case in suite["cases"]:
    result = observed[case["name"]]
    assert result["scenario_valid"] and result["required_items_preserved"]
    case["expected_view_hash"] = result["view_hash"]
manifest.write_text(json.dumps(suite, indent=2))
PY
pctx --format json filter test .pctx/filters/demo.toml --fixtures fixtures/demo > "$PCTX_DATA_DIR/demo-tested.json"
cat "$PCTX_DATA_DIR/demo-tested.json"
```

Require `data.all_passed = true`; a successful CLI exit alone is insufficient. Replacing expected hashes is an explicit golden-file update, not proof that an arbitrary filter is correct. Fixtures validate presentation only and never execute the matched program.

## Activate and inspect the exact binding

```sh
FILTER_HASH=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1]))["data"]; assert d["all_passed"]; print(d["filter_hash"])' "$PCTX_DATA_DIR/demo-tested.json")
pctx filter activate demo --expect-hash "$FILTER_HASH"
pctx filter explain -- /usr/bin/printf status
```

Activation requires the local owner and a passing complete fixture report. The private binding includes the filter, workspace, policy, executable, prefix-script inputs, fixture manifest, fixture inputs, and fixture report hashes. Changes invalidate the binding and require fixture testing and activation again. Cloning or initializing a repository does not activate its filters. Filter activation also does not grant execution trust.

## Reread and rerender without executing again

Use an output ID from a real `pctx run` or registered check execution:

```sh
pctx output show OUT-ID --view full --stream stderr --lines 1:80
pctx output find OUT-ID --literal E42 --limit 5
pctx output render OUT-ID --filter builtin
pctx output render OUT-ID --filter demo
```

A custom render requires an activated, unchanged filter. This explicit render selects the filter by ID; `filter explain` separately inspects automatic argv matching. Rerendering preserves the execution's observed native exit, signal/timeout, capture completeness, and output identity. It does not run the executable or repeat its effects.

“Full” means retained, masked, uncompressed records, not secret-bearing original bytes or an unlimited export. It defaults to the first 80 records; request additional one-based `A:B` ranges to inspect more. A range may span at most 1,001 records. Inspect `capture_complete`, omission fields, and `raw_available` before treating retrieval as complete. Temporary artifacts expire after 24 hours and are subject to retention/storage limits; `pctx run --retain none -- ...` disables later rereads. Expired or unavailable output is reported explicitly rather than reconstructed by rerunning the command.

No filter output is a completion gate. Registered checks use their independently validated typed report and source fingerprint, even when the display is shorter. See [registered checks](runner.md) for those contracts and current platform/resource limits.
