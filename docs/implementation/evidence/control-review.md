# Independent control, session, and output review

Reviewed on 2026-10-08. This pass added executable test definitions and inspected current source independently. **The reviewer did not run builds or tests.** Runtime validation and result publication belong to the integration owner. Passing results must not be inferred from this document.

## Actual defects reported to integrator

| ID | Priority | Source evidence and impact | Regression / required fix | Status |
| --- | --- | --- | --- | --- |
| WR01 | P1 | `output::trust` Add does not check local owner authority. An actor denied task administration by `PCTX_ACTOR` can still create execution trust. This promotes an agent request into owner authorization. | `concurrency::agent_cannot_authorize_execution_trust`; owner-guard trust mutation while allowing read-only plan. | Open; integrator notified |
| WR02 | P1 | Permitted `git status` inherits `core.fsmonitor` from repository configuration. Clearing `GIT_CONFIG_COUNT` does not suppress repository config. An unbound repository fsmonitor command can execute inside a trusted Git read operation. | `concurrency::git_status_does_not_run_unbound_repository_fsmonitor`; disable repo-driven execution helpers or require a complete binding profile. Fixture uses only a synthetic marker. | Open; integrator notified |
| WR03 | P1 | Control restore's schema whitelist includes only work tables. A backup after session initialization contains `pctx_*` tables and is rejected as an invalid archive. Simply accepting these tables would also leave old session acknowledgements/epochs reusable unless explicitly invalidated. | `work_control::restore_preserves_session_history_without_ack_or_epoch_reuse`; strict complete known-session table allowlist, session epochs incremented, active sessions suspended, acknowledgements cleared, historical emissions/events/capsules retained. | Implemented; runtime verification pending |
| WR04 | P1 | A test check can explicitly set `minimum_executed_tests=0`; record uses this value instead of enforcing the mandatory minimum one. A zero-test passed claim then qualifies as passed evidence, contrary to AC30. | `work_control::zero_minimum_cannot_make_zero_tests_pass`; record now enforces `max(1)` for every test check. | Implemented; runtime verification pending |
| WR05 | P2 | Session attach validates labels but directly persists runtime/native ID/role/account pool/adapter strings without masking. Known synthetic credential patterns can enter the durable control database through these fields. | Mask or reject known sensitive label content before persistence; inspect SQLite records in a synthetic credential test. | Open; integrator notified |

## Actual CLI-process tests added

`tests/concurrency.rs` uses `CARGO_BIN_EXE_pctx`, separate temporary roots, and per-process `PCTX_DATA_DIR`/actor environments:

- Two separate processes claim the same ready task simultaneously; exactly one winner, structured conflict, winner run preserved (AC24).
- Ten CLI readers race two index writers; readers see complete current results and the active generation remains ready (AC07).
- Progress report replay preserves its receipt and emits no extra event; explicit stale revision fails (AC32/35).
- Source mutation after check/review blocks completion; definition edit invalidates the submission (AC28/31).
- Source mutation between check begin and record leaves stale evidence that cannot satisfy a criterion or completion (AC28).
- Agent writes require the matching run capability and still cannot grant task administration authority.
- Agent cannot create execution trust; Git read execution cannot invoke an unbound repository hook.

`tests/core_security.rs` adds CR01/02/04/05/06/08/09/10 regressions: symlinked handoff writes, fragmented private-key masking, required-complete input failures, envelope partial/truncation, equals-form JSON CLI errors, newly excluded checkpoint paths, freshness validation, and explicit seed-limit omissions. CR03 deterministic edit-between-index-and-selection needs a test seam; CR07 walk-failure injection needs a portable fixture. Neither is falsely marked covered here.

## Review limits and remaining scope

The writer/reader test exercises real CLI processes but does not establish p95 performance, disk-full recovery, process-kill recovery, or Windows behavior. The claim race uses two processes and is not a load benchmark. Session source was being changed during integration; this review does not certify its completed implementation. Default owner mode is a same-OS-account trust convention, not a multi-user authentication boundary. Report replay before lease validation is intentional for lost-response recovery; receipt actor namespaces remain a separate gap. Environment provenance, parent-task gates, report attachments, work watch streams, resource supervision, and host admission remain incomplete and are not claimed verified.
