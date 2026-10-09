# PCTX01-G05-D05 — three isolated parent-code candidates

2026-10-09; baseline d5e85f3. User explicitly requested trying multiple PCTX
code candidates. Only D05 active. This experiment does not close D05 or qualify
any platform; PCTX01 remains33/40 details and0/10 whole gates.

## Hypotheses and results

Each arm was independently compiled offline/locked and ran the four byte-identical
original startup tests exactly once, serially. All arms preserve the original
1000ms pre-operation deadline, fresh shebang fixture creation, 8workers×16planned
calls, direct OS spawn, cwd/environment, PGID=PID, streams/overflow and cleanup.
There was no fixture warmup, retry, service observation, administrator prompt,
security change, platform update or Actions run.

| Arm | Distinct hypothesis / change | Original tests passed | Concurrent startup outcome | Reported timed-out query roots |
| --- | --- | ---: | --- | ---: |
| Unmodified baseline | Re-establish failure during this comparison |3/4|exit101 / TIMEOUT|1|
| Once-only nonblocking setup | Repeated fcntl across8collectors contributes host overhead; set both pipes once inside owned cleanup scope before draining |3/4|exit101 / TIMEOUT|2|
| Sparse owned-child observation | proc_pidinfo sampling contributes interference; cadence50→200ms |3/4|exit101 / TIMEOUT|1|
| Pipe-readiness wait | Fixed5ms sleep delays collection; replace it with poll of non-EOF pipes capped5ms and the same absolute deadline |3/4|exit101 / TIMEOUT|1|

The other three tests passed in every arm: simultaneous streams/overflow,
captured PATH resolution, and relative PATH/cwd/non-executable entry behavior.
That does not close the historical startup failures or the whole task. Concurrent
panic can censor the planned128 calls; reported timeout counts are not a complete
128-call outcome census. Serial order and changing host/cache conditions confound
comparison. Counts do not establish a statistically worse/better candidate.

All five reported failed query roots had zero stdout/stderr bytes, no EOF and
no observed root exit at expiry. Rounded spawn_ms was0–1. Since elapsed_ms starts
around spawn rather than the original pre-operation deadline, a value999 is not
evidence of a shortened or renewed request budget. Native task observations had
zero user CPU and no running thread at their observation instants. These flags
do not identify a wait reason. No new stack/policy/request collection was made;
the exact policy-wait attribution belongs to the prior trace, not these PIDs.

## Decision

None of the three candidates resolved the reproduced concurrent startup failure.
All were rejected as D05 repairs and product source was restored byte-for-byte.
Do not integrate unrelated optimizations merely because individual tests passed.
These experiments reduce confidence in these specific parent-overhead remedies;
they do not prove all parent changes ineffective or establish the inner platform
cause. No unchanged candidate retry is justified.

Next remains the same D05: a new causal launch-path mechanism preserving all
contracts, or exact request-bound policy queue/compiler/file-scan attribution and
supported platform correction. Broadening trace scope or weakening acceptance
is not implied by this request. Full original acceptance and relevant lifecycle
regressions are mandatory before any future candidate is integrated.

## Verification and restoration

Passive /root/work_control review before execution found no invariant blocker;
required setup failures inside cleanup, omitted EOF poll descriptors, next deadline
checks and no interpretation of a single success as repair. All four builds and
the final restored normal-source build exited0. Product source matches original
SHA256 bf661f4b8d0ed472585244ada4ac0e946221de364c100039cd20aee56fd8d0d2;
test/lock pins unchanged. All21known build/test roots and all5reported timed-out
query roots were absent after collection. This is not a full descendant census.
Temporary candidate copies removed; patches retained for review. No original
acceptance suite beyond these four tests was claimed.

Post-execution passive review confirmed counts, restoration and causal limits.
The harness emergency timeout/kill path was not exercised; its post-kill
communicate is unbounded and is not qualified by this batch. Do not reuse that
path for a future bounded diagnostic without preparing and checking it first.

- [Per-arm build/run receipts and retained failure diagnostics](pctx01-startup-code-candidates-result.json)
- [Consumed once-per-arm scope](pctx01-startup-code-candidates-consumed.json)
- [Runner](pctx01-startup-code-candidates-run.py.txt)
- [Cleanup and limitations](pctx01-startup-code-candidates-cleanup.json)
- [Nonblocking candidate](pctx01-startup-code-once-nonblocking.patch)
- [Observation candidate](pctx01-startup-code-sparse-owned-observation.patch)
- [Readiness candidate](pctx01-startup-code-pipe-readiness-wait.patch)
