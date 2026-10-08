# Resume handoff

Full goal: implement the mandatory specification 0.6 and concrete follow-ups, with verified CLI, installation artifacts, English public documentation, honest measurements, commit/push. Do not close at this development milestone.

Workspace /Users/dev/PCTX, main, origin https://github.com/Jung95/pctx.git. Inspect `git log -1` and `origin/main` for the first verified development commit. Preserve the original source specification locally. Implementation scope does not authorize operational deployment, global hooks, OS schedules, paid model calls or user issue edits.

## Runtime

Use project-local tools; the direct CommandLineTools path avoids the system Git/SDK shim. Keep one heavy build/test active:

```sh
export CARGO_HOME=/Users/dev/PCTX/.toolchain/cargo
export RUSTUP_HOME=/Users/dev/PCTX/.toolchain/rustup
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export PATH="$CARGO_HOME/bin:/Library/Developer/CommandLineTools/usr/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
cargo test --locked --offline
```

Tests use isolated temporary project roots and PCTX_DATA_DIR. Real local platform evidence is macOS arm64 only.

## Current ownership and pending work

Main owns all shared modules and integrated code. Search agent handed back `src/watch.rs` and `tests/watch.rs`. Requirements agent handed back the evaluation scripts/fixtures/docs. Work-control agent is writing only `docs/cli/runner.md` and `docs/cli/filters.md`; its runner native identity implementation is handed back. Inspect actual collaboration/process state before edits. No active worktrees exist.

## Exact next actions

Final macOS formatting, all-target static checks and all 148 tests passed; `evidence/full-macos-validation.log` is the current gate. Runner 19 tests passed after native OS identity and durable guardian-attachment synchronization. No heavy job is currently active at this handoff; inspect actual process state before continuing.

The local `dist/pctx-0.1.0-dev-darwin-arm64.tar.gz` is generated; archive checksum/extraction/version, isolated install/exploration, same-build atomic reinstallation and binary removal preserving user data passed (`evidence/archive-install-validation.log`). Release smoke with corrected one-symbol session scope has run: `evidence/evaluation-smoke-bounded.json` shows five valid full/ack/delta flows and compact baseline rejection, plus one refresh for five simultaneous local Git queries. Verbose-output diagnostic preservation/record limits remain failing with negative net savings. Queue normative M/30 measurements only as observations, and fix output diagnostics without hiding omissions. The debug smoke report exposed slow per-record regex compilation and lost diagnostics; fixed secret patterns now compile once. Record negative byte savings and incomplete parser coverage; never claim normative/model-cost success from smoke. Next run the normative M corpus (10,000 files/200 MiB/30 repetitions) in the same single-job queue. Update trace mappings and commit/push the verified development milestone, excluding original Korean specification, toolchain, runtime data and private/generated distribution artifacts. Continue all remaining required contracts and platform/live verification; do not mark the overall goal complete.

No PCTX product child job is intentionally active outside isolated development fixtures. Check actual process state. GitHub login confirms Jung95; origin is the authorized repository. `evidence/watch-tests.log`, `restore-quota-tests.log` and `evaluation-smoke-debug.json` capture the latest partial results; older runner failure logs remain historical evidence.
