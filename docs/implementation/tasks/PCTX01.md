# PCTX01 — CLI, envelope and stable errors

Status: **implementing; only active official task**. Selection: lowest incomplete official task, no prerequisites. Baseline: specification §5/8/14/38/45, official §21 PCTX01 row and applicable common portions of AC01/13/18/63. This file is a completion audit, not a new reduced definition of the task.

| Required gate | Current code/evidence | Remaining condition |
| --- | --- | --- |
| Help/version before project access, nested command matrix | `src/main.rs`; `tests/cli_frontend.rs` exercises root help/version and selected nested help | All visible help paths and explicit schema/effect registry pass locally; exact root version/option-order proofs pass in pctx01-global-argument-verification.json; required native Linux/Windows executions and remaining producer argument matrix remain |
| Safe parser/format errors | OS-string argv; actual format option detection before child delimiter; actual CLI malformed/non-UTF8/JSON/plain error tests | Singular global duplicate/position/default/child-argv matrix passes locally (pctx01-global-argument-verification.json); complete remaining producer argument/type/error transport matrix and native Windows equivalent input qualification |
| Invalid arguments before effects | Shared Handoff name/input admission (pctx01-handoff-input-verification.json), Find/Structure and Read/Outline/Extract/Build/Graph producer grammar/path/selector/bounds validators (pctx01-argument-preflight-verification.json), preserving capacity-first exit2; NDJSON route guard; filesystem snapshot after source edit proves strict rejected queries leave generation/database/config/output untouched | Complete other route-specific semantic validators and format/output combinations; never replace producer policy rules |
| Complete envelope, JSON final bytes and budget errors | `domain.rs`, `render.rs`, existing CLI/render/security/budget tests; shared pre-effect JSON renderer/admitted-min/Unix closed-pipe proofs in pctx01-refusal-rendering-verification.json | Audit every supported representation and minimum/error boundary; AC13 context selection belongs to PCTX12/13 and stays incomplete independently |
| Stable exit/refusal/timeout/child propagation | Existing main/output/runner tests and retained startup failure logs | Complete frontend 0/1/2/signal and prelaunch error matrix on required platforms; preserve existing native containment ownership PCTX34/40 |
| Unsupported versus successful empty | Existing search/CLI producer coverage fixtures | Preserve and test common envelope propagation; no new analyzer implementation in PCTX01 |
| No-color and control-character suppression | Renderers escape unsafe content; parser diagnostics redact secrets/escape controls; no-color applied before root/nested help on actual Unix PTY | Native Windows console and complete error/representation matrix remain; local help/color/secret fixtures pass |
| Original finite-request deadline | Inventory read scope/actual128-package and256-claim original1s processing/default/CLI state proofs (`pctx01-inventory-deadline-verification.json`); Schedule original finite scope/read-only state/row/native observation proofs (`pctx01-schedule-deadline-verification.json`); Filter scope/shared stdin and actual original250ms input/record/pipe plus policy-classification proof (`pctx01-filter-deadline-verification.json`); Resource/runner/trust read scope and actual native identity/hash/FIFO/error proof (`pctx01-runner-read-verification.json`); common classifier; adapter Doctor/Verify/ProtocolFixture original read scope and native CLI/error/lock distinction proof; Work/quota read scope/SQL/row checks; query_deadline_cli9 and work_quota_deadline5 pass (CPU SQL VM interruption and reused-connection isolation included) | Work/quota opening/transaction contention qualified locally. SQL engine expiry qualified locally; opened auxiliary source-read propagation/expiry qualified with original-loss mutation detection; authored public quota10000-row aggregation expiry qualified with guard-loss mutation; actual auxiliary config-admission expiry and fingerprint chunk-read expiry qualified with independent tests and controlled loss; additional metadata/input races and other command CPU phases remain unverified; finite-leaf audit found no clear omission; downstream phase proof, adapter phase-race coverage, full budget matrix and required platforms remain. Child/watch separation preserved |
| Supported platforms | Native macOS execution only for current changes | Native Linux and Windows full required CLI matrix; cross-compilation/static fixtures are not native proof |
| Documentation, evidence and independent review | requirements/progress/handoff, this audit and actual CLI tests | Close every gate with exact-source execution evidence before `verified` or advancing to PCTX02 |

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
