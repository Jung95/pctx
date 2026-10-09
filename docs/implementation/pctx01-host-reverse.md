# PCTX01 reverse-order host control

This is a single changed-order diagnostic, not a repair or acceptance exemption.
It reverses the earlier open→closed order to closed→open, testing whether the later
phase simply passes irrespective of host state. Both phases preserve the original
four tests and limits. Host state is human-declared; time, global policy/cache state,
contention, remaining helpers and app reopening effects still prevent causal proof.
No administrator authentication or automatic app action is used.

1. With Codex running, open an external Terminal and execute:

   ```sh
   /opt/homebrew/bin/python3 /Users/dev/PCTX/docs/implementation/evidence/pctx01-startup-host-reverse-run.py.txt --ready /Users/dev/PCTX/docs/implementation/evidence/pctx01-startup-host-reverse-batch-ready.json --output-prefix pctx01-startup-host-reverse-comparison
   ```

2. Enter `EXTERNAL TERMINAL`. Follow the prompt: quit Codex normally with ⌘Q,
   keeping Terminal open. Enter `CLOSED`. All four original tests run once in phase B.
3. When the next prompt appears, reopen Codex normally without sending a task or
   starting other development jobs. In Terminal enter `OPEN`. Phase A runs once.
4. After completion, tell this chat the reverse comparison finished. Every log and
   result is retained under `docs/implementation/evidence/pctx01-startup-host-reverse-comparison-*`.

Do not repeat a consumed session or stop early because a test passed. If an error
appears, preserve it and report it. Unexpected runner/Terminal death during a test
invalidates cleanup assumptions; exact owned processes must be checked before a
new experiment. Script exit0 is bookkeeping; per-test exits own verdicts. No secrets
or environment values are stored. The original deadline/concurrency are unchanged.

[Preflight evidence](evidence/pctx01-startup-host-reverse-batch-controls.json) records
preparation only. PCTX01-G05-D05 remains product_failure;33/40details,0/10whole gates.
