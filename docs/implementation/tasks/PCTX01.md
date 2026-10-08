# PCTX01 — CLI, envelope and stable errors

Status: **implementing; only active official task**. Selection: lowest incomplete official task, no prerequisites. Baseline: specification §5/8/14/38/45, official §21 PCTX01 row and applicable common portions of AC01/13/18/63. This file is a completion audit, not a new reduced definition of the task.

| Fixed ID | Required gate | Current code/evidence | Remaining condition |
| --- | --- | --- | --- |
| PCTX01-G01 | Help/version before project access, nested command matrix | `src/main.rs`; `tests/cli_frontend.rs` exercises root help/version and selected nested help | All visible help paths and explicit schema/effect registry pass locally; exact root version/option-order proofs pass in pctx01-global-argument-verification.json; required native Linux/Windows executions and remaining producer argument matrix remain |
| PCTX01-G02 | Safe parser/format errors | OS-string argv; actual format option detection before child delimiter; actual CLI malformed/non-UTF8/JSON/plain error tests | Singular global duplicate/position/default/child-argv matrix passes locally (pctx01-global-argument-verification.json); complete remaining producer argument/type/error transport matrix and native Windows equivalent input qualification |
| PCTX01-G03 | Invalid arguments before effects | Shared Handoff name/input admission (pctx01-handoff-input-verification.json), Find/Structure and Read/Outline/Extract/Build/Graph producer grammar/path/selector/bounds validators (pctx01-argument-preflight-verification.json), preserving capacity-first exit2; NDJSON route guard; filesystem snapshot after source edit proves strict rejected queries leave generation/database/config/output untouched | Complete other route-specific semantic validators and format/output combinations; never replace producer policy rules |
| PCTX01-G04 | Complete envelope, JSON final bytes and budget errors | `domain.rs`, `render.rs`, existing CLI/render/security/budget tests; shared pre-effect JSON renderer/admitted-min/Unix closed-pipe proofs in pctx01-refusal-rendering-verification.json | Audit every supported representation and minimum/error boundary; AC13 context selection belongs to PCTX12/13 and stays incomplete independently |
| PCTX01-G05 | Stable exit/refusal/timeout/child propagation | Main/output/runner actual child/prelaunch matrix; registered CheckRun/Check Run/local HelperRequest processing errors and incomplete capture in pctx01-nested-execution-verification.json; retained startup failure logs | Complete frontend 0/1/2/signal and prelaunch error matrix on required platforms; preserve existing native containment ownership PCTX34/40 |
| PCTX01-G06 | Unsupported versus successful empty | Outline actual JSON/compact/Markdown complete unsupported6, supported empty0, mixed/parse partial3 and strict-refresh partial priority in pctx01-outline-coverage-verification.json | Complete remaining producer coverage matrix; no new analyzer implementation in PCTX01 |
| PCTX01-G07 | No-color and control-character suppression | Renderers escape unsafe content; parser diagnostics redact secrets/escape controls; no-color applied before root/nested help on actual Unix PTY | Native Windows console and complete error/representation matrix remain; local help/color/secret fixtures pass |
| PCTX01-G08 | Original finite-request deadline | Inventory read scope/actual128-package and256-claim original1s processing/default/CLI state proofs (`pctx01-inventory-deadline-verification.json`); Schedule original finite scope/read-only state/row/native observation proofs (`pctx01-schedule-deadline-verification.json`); Filter scope/shared stdin and actual original250ms input/record/pipe plus policy-classification proof (`pctx01-filter-deadline-verification.json`); Resource/runner/trust read scope and actual native identity/hash/FIFO/error proof (`pctx01-runner-read-verification.json`); common classifier; adapter Doctor/Verify/ProtocolFixture original read scope and native CLI/error/lock distinction proof; Work/quota read scope/SQL/row checks; query_deadline_cli9 and work_quota_deadline5 pass (CPU SQL VM interruption and reused-connection isolation included) | Work/quota opening/transaction contention qualified locally. SQL engine expiry qualified locally; opened auxiliary source-read propagation/expiry qualified with original-loss mutation detection; authored public quota10000-row aggregation expiry qualified with guard-loss mutation; actual auxiliary config-admission expiry and fingerprint chunk-read expiry qualified with independent tests and controlled loss; additional metadata/input races and other command CPU phases remain unverified; finite-leaf audit found no clear omission; downstream phase proof, adapter phase-race coverage, full budget matrix and required platforms remain. Child/watch separation preserved |
| PCTX01-G09 | Supported platforms | Native macOS execution only for current changes | Native Linux and Windows full required CLI matrix; cross-compilation/static fixtures are not native proof |
| PCTX01-G10 | Documentation, evidence and independent review | requirements/progress/handoff, this audit and actual CLI tests | Close every gate with exact-source execution evidence before `verified` or advancing to PCTX02 |

Checklist count: **0/10 whole gates closed**. This assigns stable IDs to the
existing ten rows; previous denominator10, current denominator10, no scope added.
All rows remain implementing because their mandatory residual conditions above
are open. Local subset qualifications do not close a whole gate. Native macOS
observations are recorded separately; required Linux/Windows remain unverified.
Each row uses the baseline sections above, existing evidence and exact remaining
condition; changes to the denominator require a specification-grounded record.

PCTX01 owns transport admission, envelope, representation, error/exit propagation
and common original-clock plumbing. Producer business contracts remain with their
responsible tasks: discovery PCTX02; policy PCTX03; search/structure PCTX05/08;
selection PCTX12/13; checkpoints PCTX14; session/ack PCTX29/30; runner containment
PCTX34/40; output artifacts/parsers PCTX40–44. Those tasks consume the common
contracts. A broken common gate may require a producer correction; it does not
transfer the producer's independent feature or whole acceptance ID to PCTX01.


AC01 non-Git discovery/registry feature ownership remains PCTX02; PCTX01 reuses existing flow evidence for common CLI behavior. AC63 native child containment stays with PCTX34/40 while PCTX01 must preserve the frontend status. Shared IDs do not excuse omitted conditions or permit whole-AC completion from this subset. Common timeout corrections required here may modify shared code, but must not add independent Work/adapter/schedule functionality.

Next local action after the current integration gate: help/color locally integrated; finish the finite-route/option-before-effect matrix and required native frontend exit tests. No next official task is selected. If an external platform is unavailable, complete local contracts first and record the exact needed native run; do not silently lower the gate or start unrelated work.


## Native startup isolation evidence

PCTX01 remains the only active official task (`implementing`). Read-only native startup isolation at unchanged base fc1849d18df11e9f21f1dad2bf9d550ac580f03d reproduces original1s expiry outside the PCTX supervisor: raw direct scripts fail with both empty and minimal PATH environments; reversed-order group controls show sub-millisecond spawn return followed by delayed first execution, and no-group-first scripts also produce two expiries. System /bin/echo passes128 attempts; a private copy fails all8 (6 expiry,2 early signal-status failures; signal number not recorded), so it is not an equivalent trust/location control. Correlated privacy-redacted syspolicyd scans are not bound to exact child paths/PIDs and do not prove causation. Passing later attempts are not repairs. Tiny inert drivers stop each worker after failure and root wait after kill can exceed1s; no production cleanup qualification. Evidence: pctx01-startup-isolation-verification.json. No production/test/budget changes, security-policy changes, interpreter substitution, full-gate rerun or Actions. At that evidence boundary the exact-source full was452PASS1FAIL; frontend and native Linux/Windows gates remained open. No next official task selected.


## Preceding PCTX01 representation admission and diagnostic delivery

PCTX01 remains the only active official task (`implementing`). Common Markdown admission reuses the exact supported registry after capacity checks and before semantic/project/stream/response-file dispatch; execute retains defense. Eleven valid unsupported routes with/without output require JSON INVALID_ARGUMENT2 and unchanged missing-root/data/output state, including watch/follow and run argv. Initial relative-output attempt returned7; the absolute fixture proves rejected init wrote its response file. Both are retained. Removing only common preflight yields actual0PASS1FAIL and exact source restoration. Actual plain parser closed stderr initially exits101; checked writes repair it toIO7 while normal delivered diagnostics stay2, with masking unchanged. Related37PASS; format and locked all-target Clippy PASS. First full71673 stops at261PASS3FAIL; final no-fail-fast full38079 is terminal101: 453PASS/2FAIL, retaining original native startup failures. Evidence: pctx01-representation-admission-verification.json. Independent review no scoped blocker. Unix pipe evidence does not qualify Windows; full frontend/phase/platform/startup gates remain open. No next official task or Actions.


## Preceding PCTX01 singular global argument binding

PCTX01 remains the only active official task (`implementing`). Singular root/format/output/timeout-ms/no-color options now reject same/cross-depth duplicates through a fresh shared command-line-only typed parser, preserving defaults, native paths, typed validation/possible-values metadata, repeated scopes and literal child argv. Ten actual duplicate cases refuse2 before missing-root/data/output effects; three global positions bind identical root/workspace and preserve accepted10000ms/zero-refusal semantics. Four root version/flag-order cases return exact compiled plain version/no effects. Pack/Create alone defers builder output requiredness until global propagation; nonoptional PathBuf presence still required and help explicit. Actual Pack CLI publishes artifacts at all3 depths with response stdout, rejects missing/duplicate output. Initial global0PASS2FAIL includes checkpoint-list DB_ERROR before index preparation; global fixture now prepares index and independent checkpoint issue is PCTX14 backlog, not fixed/qualified here. Initial parser2PASS1FAIL retains pre-group Pack output refusal; repaired3PASS. Related56PASS and added actual Pack1PASS; controlled guard loss actual0PASS1FAIL/exact restore. Format/locked all-target Clippy PASS. Final native full82923 terminal101: 459PASS/3 retained original startup FAIL. Evidence: pctx01-global-argument-verification.json. Independent scoped review no blocker. Fresh command required per parse; full frontend/phase/platform/startup gates remain open. No next official task or Actions.


## Current PCTX01 primary delivery and safe stream frames

PCTX01 remains the only active official task (`implementing`). Checked primary output covers help/version print, stream-error stderr, native hook stdout and response-file-error diagnostic; failures returnIO7 without successful absence or panic. Initial actual closed pipes0PASS4FAIL (help0, NDJSON refusal2, hook/file diagnostic101) retained. Hook/NDJSON use shared reversible JSON escaping without added envelope; compact columns escape hidden controls/newline/tab and preserve row boundaries. Watch2unitPASS including value roundtrip/write/flush error7; related28PASS and final delivery/metering13PASS. Actual hook test qualifies replay of already imported PermissionDenied under broken stdout: same key remains one receipt, not first-import failure/rollback. Actual saved-output metrics-lock plus closed-stderr warning preserves completed stdout/exit0, unchanged artifact and one invocation/no rerun; warning is best effort and measurement remains unknown. Format/locked all-target Clippy PASS. Final native full84654 terminal101: 464PASS/4 retained original startup FAIL. Evidence: pctx01-delivery-verification.json. Independent scoped review no blocker. Unix-only pipe proof, first-import timing, hook input/output preflight, actual child-exit matrix and full remaining frontend/phase/platform/startup gates stay open. No next official task or Actions.


## PCTX01 hook transport admission

Only PCTX01 remains active and incomplete. Native hook transport rejects
`--output` and `--from-file` before project/file access; shared input validation
also remains in the producer. First-import broken stdout returns IO_ERROR7;
same-key replay emits the recorded native object and retains one receipt.
Delivery failure does not imply rollback. See
[evidence manifest](../evidence/pctx01-hook-verification.json) for initial failure,
targeted/static/full results and source hashes. Independent scoped review found
no blocker. Native Unix pipes are not live Claude or native Linux/Windows proof.
The remaining PCTX01 frontend/phase/platform and original startup gates remain
mandatory; no next official task is selected or Actions dispatched.

Final native full46672 terminal101: 466 PASS/3 original startup FAIL; related35 PASS, format/locked all-target Clippy PASS. All jobs terminal; PCTX01 remains incomplete.


## PCTX01 manual-run frontend outcomes

Actual CLI cases now qualify child0/1/2/signal in both modes, project/trust
prelaunch refusal, missing-interpreter spawn failure, execution timeout, invalid
UTF8 partial capture and real output-publication lock contention. Signal/full
reread preserves outcome with no rerun; classification-guard loss detects6->7.
[Manifest](../evidence/pctx01-run-exit-verification.json) retains initial/interrupted
gates and fixture errors. Post-spawn unknown branch still needs fault-injection
proof; pure argument/budget/phase/native platform gates remain mandatory.
PCTX01 remains implementing, with no next official task.

Final native full97588 terminal101: 473 PASS/4 retained original startup FAIL. Related45 PASS; final format/locked all-target Clippy PASS. All jobs terminal; PCTX01 remains incomplete.


## PCTX01 Run refusal and capacity truth

Actual both-policy minimum−1/minimum/minimum+1, valid sub3000 producer refusal,
missing-project6 near minimum and pure capacity/format/timeout admission now retain
no-child facts and correct exit with final-byte checks/no jobs/output files.
[Evidence](../evidence/pctx01-run-budget-verification.json) retains initial failures.
Actual post-spawn presentation fallback, oversized-error variants and required
native platforms/startup gates remain unverified. Task remains implementing.

Initial full475PASS5FAIL retained (closed-pipe fixture ambiguity plus4 original
startup failures). Owned native reader control establishes exit2 versus7 when
inheritance changes; it does not bind the original failure cause. The isolated
single-case worker retains criterion7 and outer suite concurrency. Other raw
pipe fixtures and nested-CLI cleanup on worker timeout remain unqualified.

Final full58827 terminal101: 477 PASS/3 original startup FAIL; related62 PASS and final format/locked all-target Clippy PASS. All jobs terminal; PCTX01 remains incomplete.


## PCTX01 delivery fixture ownership

PCTX01 is the only active official task (`implementing`). All seven discovered
raw-pipe delivery cases now use one exact-case worker and verified close-on-exec
pipe descriptors; outer-suite concurrency and original product deadlines remain.
Worker output uses private files. Timeout cancellation signals the owned process
group, and WNOWAIT reserves the leader identity through the observer and reap.
A real native-hook CLI holds an O_RDWR FIFO: root-only cancellation cannot end
its input through EOF. Positive related coverage: 34 PASS. Controlled root-only
cancellation: 0 PASS/1 FAIL, remaining owned group cancelled, exact source restored.
Format and locked all-target Clippy PASS. Independent scoped review found no
blocker. See [evidence manifest](../evidence/pctx01-delivery-ownership-verification.json).

Normal fixture completion awaits its CLIs; it does not qualify early failed-root
cleanup. Panic cleanup is best effort, and kernel kill/wait latency is not a hard
bound. The control covers a directly enrolled CLI, not separate runner groups or
arbitrary descendants. Native Linux/Windows, original startup failures and other
PCTX01 gates remain open. No next official task or Actions selected.

Final native full25288 terminal101: 478 PASS/3 retained original startup FAIL. All jobs terminal; no live heavy job. PCTX01 remains incomplete.


## PCTX01 final serialized Run byte budget

Actual native children emit normalized bidi text that is admitted as one record,
but the safe JSON document exceeds 5000 bytes. The final-byte loop removes that
record within an explicit 5000-byte budget for JSON/compact and child/PCTX policies,
while preserving child0/2/signal15, complete capture and absence of processing
errors. Full saved reread retains normalized text and exact child/signal; five
invocations per case show no reread rerun. Related32 PASS; format/locked all-target
Clippy PASS. Disabling only final record trimming produces actual0PASS1FAIL/exit8
instead of expected0, then exact source restoration. The initial C1 fixture failed
its >budget guard because normalization removed C1; it is retained, as is an
invalid test-target selection that ran no tests. Independent review found no
scoped blocker. [Evidence](../evidence/pctx01-run-final-bytes-verification.json).

This qualifies record trimming, not metadata-only budget fallback or spawn-observer
failure injection. Production code is unchanged. Native Linux/Windows and the
remaining PCTX01 error/argument/phase/startup/platform gates remain mandatory.
Only PCTX01 remains active and incomplete; no next official task or Actions.
The prior main push approval boundary is unchanged; no rejected push was retried.

Final native full30370 terminal101: 479 PASS/3 original startup FAIL. All jobs terminal; PCTX01 incomplete.


## PCTX01 budget fallback execution truth

CLI oversized fallback now delegates its unchanged reduction to shared
`render::budget_fallback`; admission, byte measurement, final serialization and
child exit precedence remain with the CLI. Boundary injection into actual native
child0/1/2/signal, prelaunch denial, incomplete capture and publication-failure
observations verifies retained child state/handles and existing fatal codes.
Nested execution/retrieval and unattested/no fabricated-child tests also pass.
Related22 PASS; format/locked all-target Clippy PASS. Controlled losses each
0PASS1FAIL: child0 becomes missing, and denial5 becomes8. Exact source restored.
Initial missing import and crate-path build failures ran no tests and are retained.
Independent review found no scoped blocker.
[Evidence](../evidence/pctx01-budget-fallback-verification.json).

This is shared-boundary injection, not a naturally occurring CLI metadata fallback.
Universal fallback-size bounds, native spawn-observer failure injection, remaining
producer/phase/startup gates and native Linux/Windows remain open. Only PCTX01 is
active and incomplete. No next official task, Actions or rejected-push retry.

Final native full30035 terminal101: 480 PASS/4 original startup FAIL. All jobs terminal; PCTX01 incomplete.


## PCTX01 Outline unsupported versus empty coverage

Existing per-file unsupported structure now reaches the common envelope:
complete source scope/all-unsupported Outline6/error/coverageunsupported; mixed requests3/
partial; supported empty0 and syntax-partial3 remain. Actual JSON/compact/Markdown
matrix and empty text Find on unsupported .rb pass. No analyzer feature added.
Initial fixture self-indexed an in-root data override (partial3), retained in
backlog; sibling project/data fixture then proved actual unsupported0/complete
failure. Initial guard loss produces actual0PASS1FAIL. Final unsupported/refresh guard
losses each0PASS1FAIL with exact restoration. Initial related73 PASS; reviewed final related74 PASS including incomplete strict
refresh JSON/compact/Markdown. Final format/locked all-target Clippy PASS.
[Evidence](../evidence/pctx01-outline-coverage-verification.json).

Manual callback always-Ok and registered PID-before-publication paths are source
audited only; native registered failure injection remains unqualified. Remaining
frontend/producer/phase/startup and native Linux/Windows gates remain open.
Only PCTX01 is active and incomplete; no next task, Actions or rejected-push retry.

Initial full76840 terminal101: 482PASS3FAIL retained before the independent review fix. Final full43620 terminal101: 482 PASS/4 original startup FAIL; all jobs terminal, PCTX01 incomplete.

## PCTX01 nested execution processing state

Common CLI classification now reads nested execution for registered CheckRun,
Check/Run and local HelperRequest. Processing errors retain typed failure codes;
incomplete capture returns partial3. Valid observed failed reports remain
processing success0 with failed evidence. Manual child exit policy is unchanged.
Real linked-worktree helper and both check routes are covered; the original50ms
registered timeout and killed-guardian assertions now require error7 instead of
wrapper success. No runner feature, ledger or deadline changed.
[Evidence](../evidence/pctx01-nested-execution-verification.json) retains initial
failures, corrected fixture scope, independent review and two controlled losses.
Native callback-fault injection, remaining frontend/phase/startup gates and
native Linux/Windows remain open. PCTX01 stays implementing; no next task.

Related49 PASS, added helper1 PASS; two controlled losses each0PASS1FAIL/exact restoration. Final format/locked all-target Clippy PASS. Final native full17428 terminal101: 485 PASS/5 FAIL. Four original1s project_deadline failures and adapter version failure6-versusTIMEOUT7 are retained, without asserting one cause. All jobs terminal; PCTX01 incomplete.

## PCTX01 counterbalanced native startup observation

Read-only direct native diagnostic:8 workers,16 repetitions/cell, balanced
fresh/reused executable paths x group/no-group, original1s budget;512 attempts,
509 within-budget observations and3 expiries. Reused paths are separate per worker/
group with first use explicitly cold; no prewarm or retry qualification. Two
expiry rows have no bytes; one has both streams/EOF and later reaps0 despite
owned-group kill EPERM1. All remain failures; no common cause established.
[Evidence](../evidence/pctx01-startup-counterbalanced-verification.json) records exact
path/dev/inode/PID, timestamps, observed versus cancellation signal, all cells,
build failure/repair and independent review. No product/original-test changes or
full-suite rerun; latest full485PASS5FAIL remains. Cleanup/panic/launch equivalence,
cold-cache and policy causation are unqualified. Next causal observation must
separate first-read/EOF timing from exit-probe/parent observation gaps. PCTX01
stays implementing, no next official task, Actions or rejected-push retry.

## PCTX01 native read/exit observation timeline

Read-only512-attempt balanced diagnostic adds independent parent reader first-byte/
EOF and exit-probe/loop timestamps under original1s.510 within-budget/2expiry;
late-unexpired0. Both expired rows have no observed bytes and parent loop gaps
1609/1544us, last probes near999ms. These two rows do not exhibit a near1s parent
exit-observation gap; exact child progress/policy cause remains unproven. Reader
clock values are parent observations, zero means not observed, and post-cancel
status does not prove cancellation causation. Cleanup/panic/launch-equivalence
limits remain. [Evidence](../evidence/pctx01-startup-timeline-verification.json).
No product/original-test changes or unchanged full rerun; full485PASS5FAIL and
prior3-expiry diagnostic remain. PCTX01 stays implementing, no next official task.

## PCTX01 native callback processing errors

Actual library registered execution callback injection exposed monitor
CONFIG_CHANGED9 being reduced to default7. Common output run_inner now retains
original Error, carrying code/message/exit into the immediate response; later
save failure keeps its existing precedence. No runner/schema/guardian policy
change. Native spawn-callback IO7 and monitor CONFIG_CHANGED9 fixture confirms
body entry once, group identity, callback counts and direct-child reaping. This
is injected library callback qualification, not actual frontend guardian/ledger
publication-fault qualification. Saved artifact retains code rather than the full
typed Error; arbitrary descendant/resource reconciliation remains unverified.
[Evidence](../evidence/pctx01-callback-native-verification.json) preserves initial
0PASS1FAIL exit7-versus9, repaired1PASS and related89PASS before the test cleanup
guard. Final guard has owned/unreaped-root admission, best-effort/unbounded wait
and EINTR limitations; no injected panic qualification. PCTX01 stays implementing;
no next official task, Actions or rejected-push retry.

Final format/locked all-target Clippy PASS. Final native full60317 terminal101: 488 PASS/3 FAIL. Original concurrent startup, relative executable resolution and simultaneous-stream startup failures remain; no budget relaxation or retry promotion. All jobs terminal; PCTX01 incomplete.

## PCTX01 saved-output argument admission

Shared Output request validation now precedes CLI project discovery and producer
artifact loading. Show/Find/Render IDs, original full line bounds and literal/
limit rules are reused; compact stream/line selectors explicitly require full
view instead of silent ignoring (§14). Actual26-case JSON/compact CLI matrix
refuses2 before missing-root/data/absolute-response effects, and library rejects
invalid queries before missing artifacts. Related39 PASS. Two controlled losses
each0PASS1FAIL/exact restore: frontend filesystem7 versus2, producer OUTPUT_EXPIRED
versus INVALID_ARGUMENT. [Evidence](../evidence/pctx01-output-admission-verification.json).
[Concrete remaining admission gates](PCTX01-admission-audit.md): Checkpoint
name/glob before writer/manifest, then ContextGet mode/since/normalized scope before
session DB. These are PCTX01 common fixes only; no other official task activated.
Native platforms/startup and remaining completion gates stay open.

Final format/locked all-target Clippy PASS. Final native full64728 terminal101: 490 PASS/3 FAIL, retaining original concurrent startup, relative resolution and simultaneous-stream failures at unchanged budgets. All jobs terminal; PCTX01 incomplete.

## PCTX01 Checkpoint pure admission

Existing display-name/redaction/256-byte and relative-glob scope rules now share
storage checkpoint_arguments, called before producer writer/manifest; CLI pure
validation precedes discovery/response paths. Names remain display strings,
checkpoint dot/glob scopes remain supported; no new Checkpoint feature. Actual
12-case missing/initialized project and absent/existing absolute response matrix
refuses2 with unchanged tree/response. Library invalid requests create no writer,
and expired supplied clock remains original TIMEOUT. Valid display name/dot/glob
flows pass. Related12 PASS. Initial1PASS2FAIL retained (IO7 masks2); prepared
workspace initial0PASS1FAIL proves actual writer.lock creation. Two controlled
losses each0PASS1FAIL/exact restore (CLI refusal and writer-order regression).
[Evidence](../evidence/pctx01-checkpoint-admission-verification.json). Independent
static review no scoped blocker. Next common defect is ContextGet pure admission;
PCTX01 stays implementing, no other official task, Actions or rejected-push retry.

Final format/locked all-target Clippy PASS. Final native full97981 terminal101: 493 PASS/3 FAIL; original relative resolution/concurrent startup/simultaneous-stream failures retained without budget relaxation. All jobs terminal; PCTX01 incomplete.

## Current visible command admission register

G03 now has an actual visible enumeration:172 paths/140 leaves, stable C001–C140
in [complete register](PCTX01-command-admission-registry.md), not merely the
historical bounded A01–A11 groups. Residual inventory R01–R12 records existing
pure contracts after source review; the earlier outer Role evidence does not
qualify stronger downstream policy controls. R12 runner correction has local
native/static/mutation/independent review evidence in
[evidence](../evidence/pctx01-runner-admission-verification.json). Whole gates
remain closed0/10; native Linux/Windows and original startup failures are open.
No other official task is active. Next same-task correction is R01–R03 existing
Operations/Role admission; state/authority/input schema stays in producers.

## PCTX01 Report, Session and Ingest admission — 2026-10-08

Only PCTX01 remains active (`implementing`), fixed G03-R04–R06, specification
§14/§38. Existing Work Report stage/summary 4096 bytes / percentage 100 predicates and
library CheckRun key presence now run before storage. Session Suspend/Boundary
reuse existing reason grammar before snapshot/DB; ContextAck provenance is shared
before DB (already Clap-protected). QuotaIngest reuses its existing key label
before owner/input/DB, checking an original supplied expiry first. Valid report
receipt/lease/order, session epoch and observation/authority contracts remain
producer-owned; no independent business feature added.

Native macOS: nine invalid forms across 36 missing/initialized, JSON/compact,
absent/existing absolute response combinations refuse with exit 2 without project/data/output
effects. Six direct invalid/expired requests preserve storage and the original
Instant. Actual all five Report stages accept summary 4096 bytes / estimate 100 and replay
with identical data and no extra storage; Session Boundary/Suspend accept 1024-byte
reasons. Actual manual QuotaIngest accepts key 256, replays identically without new
storage and denies an unregistered actor with exit 5 without effects. Empty Report summary/
estimate 0 and both Ack provenances are shared-validator proofs only, not new native
Ack-flow qualification. Final related: 51 PASS; five controlled admission losses each
0 PASS / 1 FAIL, exact source restoration; format/locked all-target Clippy PASS. Scoped
independent read-only review found no blocker. Initial vector-index fixture panic
and subsequent actual baseline IO7 masking2 are retained separately.

Evidence: `../evidence/pctx01-report-admission-verification.json` and review/logs.
Fixed residual denominator remains12: local 7/12 (R01–R06,R12), whole required
platform 0/12. Whole PCTX01 gates remain 0/10. Linux/Windows remain unverified and
all four historical native startup conditions remain unresolved regardless of
incidental later passes. Next local boundary is R07 within PCTX01. No next official
task or Actions is selected.

Final native full44671 terminal101: 521 PASS / 4 FAIL. Failed tests: relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, query_program_resolution_uses_captured_command_path, simultaneous_streams_are_collected_and_overflow_never_infers_success, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget. All four historical startup conditions remain unresolved; original budgets unchanged. All jobs terminal; no heavy job live. PCTX01 remains incomplete (0/10 whole gates).
