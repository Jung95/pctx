# Inactive backlog

Only PCTX01 is active. These observations do not authorize concurrent official tasks.

| Official task | Observation / required future action | Evidence |
| --- | --- | --- |
| PCTX18 | Packaging copies `target/release/pctx` even when Cargo builds under a custom target directory. Honor Cargo metadata's target_directory and verify archive binary equals intended measured binary. Patch prepared but removed from current active implementation. | `backlog/package-target-directory.patch`; actual script inspection, no new archive verification |
| PCTX44 / related approval and Pack owners | Task-linked ops_decisions are absent from ContextGet/receipt source identity; unchanged Git can hide approval record/expiry changes. Define bounded consumer/source-policy-scoped opaque metadata and expiry tombstones, final epoch/state revalidation and non-authoritative Pack provenance; no owner text/tokens/execution grant. | Read-only requirements review; context.rs source_versions, session.rs ReceiptSpec::packet, operations.rs Decision::Record; not implemented or verified |
| PCTX05/17 | Pre-existing eleventh immutable M30 completed on source f81d55e, body p95 1739.805458ms / 30 samples / 0 errors. Observed-host timing meets 2000ms; reference hardware, cold cache, full task/model/provider cost/quota remain unqualified. Preserve earlier misses. No task completion follows. | `evidence/evaluation-m30-eleventh.json`, samples/mission/source/build logs |

| PCTX14 | Actual `init` then `checkpoint list` before any index initialization returns DB_ERROR7. The global-position fixture prepares the index to isolate argument binding; it does not qualify successful empty checkpoint history on a newly initialized project. Inspect checkpoint_list/connect schema/read-state prerequisites against §13 and repair/verify the independent checkpoint contract when PCTX14 becomes active. | `evidence/pctx01-global-initial.log`; `src/storage.rs` checkpoint_list/connect; no checkpoint implementation change in PCTX01 |


PCTX03/06 inactive investigation: placing an explicit PCTX_DATA_DIR underneath
the indexed root caused the initial Outline fixture's index update to return
partial3 for its own SQLite database/WAL/SHM (UNSUPPORTED_ENCODING). See
`evidence/pctx01-outline-coverage-selected.log`. Determine the intended exclusion
contract for user-overridden local data paths when the owning task is active.
Current fixture keeps project and data directories as siblings; no automatic
exclusion behavior is implemented or claimed here.


PCTX36 inactive Linux investigation: unchanged source ede92857, actual
Linux aarch64 Ubuntu24.04 container with executable tmpfs and init reaper,
`actual_cli_concurrent_ticks_claim_one_occurrence_and_fixture_bridge_invokes_tick`
returned INDEX_BUSY7 at tests/schedule.rs:774. Original concurrency/budgets were
unchanged. The prior noexec environment run passed this test, so that pass is not
a repair; no schedule product/test change is made here. Preserve this failure
even if later whole runs pass incidentally. Evidence:
`evidence/pctx01-linux-environment-full.log`. Investigate the owning schedule/
coordination contract when active; any necessary common admission/deadline
correction must remain in the current PCTX01 scope.
