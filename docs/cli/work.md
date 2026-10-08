# Local work and verification

Save this task definition outside the project input scope, for example `/tmp/pctx-task.json`:

```json
{
  "schema_version": 1,
  "title": "Validate authentication behavior",
  "scope": ["src/**", "tests/**"],
  "acceptance": [{"id": "behavior", "description": "Regression check passes", "required": true, "weight": 1, "evidence_check_keys": ["unit"]}],
  "checks": [{"key": "unit", "kind": "test", "required": true, "input_scope": ["**"], "output_paths": ["reports/**", "target/**"], "minimum_executed_tests": 1, "success_exit_codes": [0], "allowed_sources": ["external_report"], "require_report_attachment": false}],
  "review_policy": {"required": true, "reviewer_kind": "human"}
}
```

Create a project binding and register the work. Replace returned task and run IDs in subsequent commands:

```sh
pctx init
pctx task create --from-file /tmp/pctx-task.json
pctx agent register --name worker --kind agent
pctx task ready T-1
pctx task assign T-1 --agent worker
pctx task start T-1 --workspace current
pctx agent report --run RUN-ID --lease-epoch 1 --report-seq 1 --idempotency-key work-1 --stage testing --summary "Regression implementation ready"
pctx check begin T-1 --key unit --run RUN-ID
```

Run your actual tests independently. Record their structured results in a declared output path, such as `reports/unit.json`. This example shows the schema, not evidence of a test execution:

```json
{
  "schema_version": 1,
  "check_key": "unit",
  "producer": "your-test-runner",
  "source": "external_report",
  "exit_code": 0,
  "tests": 1,
  "passed": 1,
  "failed": 0,
  "errors": 0,
  "skipped": 0,
  "result": "passed",
  "started_at": 1791417600,
  "finished_at": 1791417601,
  "environment": {"runner_version": "record-the-actual-version"}
}
```

Submit the result and review the exact source target:

```sh
pctx check record CHECK-ID --from-file reports/unit.json
pctx task criterion accept T-1 --criterion behavior --evidence CHECK-ID
pctx task submit T-1 --run RUN-ID
pctx task review T-1 --approve --note "Reviewed target and evidence"
pctx task complete T-1 --dry-run
pctx task complete T-1
pctx board
pctx activity --since-seq 0
```

External reports are claims from an explicit producer. PCTX validates their structure, counts, source policy, and source fingerprint; it does not retrospectively prove execution or lock ownership. A zero-test report, failure, changed input, changed definition, missing review, or old submission cannot satisfy the completion gate.

Local invocations default to the trusted owner operator. Set `PCTX_ACTOR` to an agent identity for restricted writes and present `PCTX_RUN_CAPABILITY` for that run. Run credentials are private local files and are excluded from backups and normal output. This prevents accidental stale-run mutation; it does not isolate adversaries sharing the same OS user.
