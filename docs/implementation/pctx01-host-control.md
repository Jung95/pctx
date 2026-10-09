# PCTX01 prepared host comparison

This prepared diagnostic needs no macOS administrator authentication. It has not
run yet. PCTX01-G05-D05 remains a product failure; no acceptance gate is closed.
The purpose is to compare the same original tests while Codex is running and after
the operator quits it normally. Host state is declared by the operator; fixed order,
time, cache, contention and possible remaining helpers prevent a causal conclusion.

All readiness references are absolute; this command works from your home directory.
The earlier home-directory FileNotFoundError occurred before any test and consumed
no session. It is safe to start the corrected preparation with the same command.

1. Open an external Terminal, keeping Codex running. Run:

   ```sh
   /opt/homebrew/bin/python3 /Users/dev/PCTX/docs/implementation/evidence/pctx01-startup-host-control-run.py.txt --ready /Users/dev/PCTX/docs/implementation/evidence/pctx01-startup-host-control-batch-ready.json --output-prefix pctx01-startup-host-control-comparison
   ```

2. Enter `EXTERNAL TERMINAL` at the first prompt. Four original tests run once,
   sequentially, retaining each failure. Wait for the phase-A completion prompt.
3. Quit Codex normally and leave the external Terminal open. Enter `CLOSED` only
   after quitting; the same four tests run once in phase B. Any other text stops.
4. After both phases finish, reopen Codex to review the evidence. Results are saved
   under `docs/implementation/evidence/pctx01-startup-host-control-comparison-*`.

The runner checks source/binary hashes, preserves the original one-second budget,
8×16 concurrency and assertions, and consumes its session once. It stores no
credentials or environment values and does not control or observe other apps.
Do not rerun a consumed session or treat script exit0 as all tests passing. If the
Terminal or runner dies during an active test, cleanup is unverified: inspect exact
owned test processes before another experiment. Native cleanup remains cooperative.
Do not edit product source between phases. Stale input is refused, not rebuilt.

[Readiness](evidence/pctx01-startup-host-control-batch-ready.json) and
[preflight evidence](evidence/pctx01-startup-host-control-batch-controls.json) record
preparation only. No root helper, security-policy change or authentication is used.
