# Implementation plan

Baseline: all 45 sections of `docs/PCTX-implementation-spec-v0.6.md` and the user goal objective. `requirements.md` owns the item/AC mapping. This plan establishes dependencies and integration boundaries; it does not claim completed features. All groups start `not_started`; the main integration owner updates statuses from reviewed code and execution evidence.

## Serial official-task rule

The latest user instruction controls task selection. Activate exactly one official
PCTX task, prioritizing an explicit user selection; otherwise choose the lowest
incomplete task whose prerequisites are satisfied. Compare requirements, progress,
handoff and actual code before implementation. Do not advance until every mandatory
implementation, acceptance, documentation and platform gate is verified.
Related common fixes are allowed; unrelated findings remain inactive backlog.
A necessary unfinished prerequisite must be identified explicitly, completed alone,
then return to the original task. An external blocker requires its exact condition
and required action. The latest user instruction supersedes the older external-blocker exception:
if external conditions block completion, report the exact condition and required
action, preserve the current task, and do not select another task. Current selection:
PCTX01, implementing, no next official task.

## Ownership and common boundaries

The integration owner alone edits Cargo configuration/lockfiles, common Domain types, shared schemas/migrations, CLI routing and publication metadata. Delegated work has exclusive file ownership and proposes shared contract changes before implementation. Current documentation analysis owns only `requirements.md`, `plan.md`, and `decisions.md`. Progress, handoff, source specification and product code are outside that assignment.

Domain provides typed IDs, authorized relative paths, verified byte/range/hash records, three independent trust/freshness/coverage axes, revisions, fingerprints, errors, receipts and artifact provenance. Application services execute command ordering and shared business rules. CLI and adapters only validate transport arguments and render results. No SQL in CLI, direct unapproved reader in parser/context, human text as internal success state, duplicate runner or second ack ledger (§5,24).

Index storage is workspace-scoped and derivative. Control storage is coordination-scoped and durable. Host resource claims live at host scope. Common policy gates all file reads, metadata, output/artifact retrieval, exports and packs. A single artifact interface publishes immutable manifests/report/output/pack metadata with hash, lifetime, completeness and current-policy validation; the data classes retain distinct lifetimes (§9,17,27,33,40,44).

## Current physical integration map

The integration owner confirmed an initial flat Rust layout. Logical ownership above maps to actual files as follows; future extraction into files/crates must preserve these boundaries.

| File | Current responsibility |
| --- | --- |
| `src/domain.rs` | Shared IDs, errors, response envelope and domain records |
| `src/project.rs` | Configuration/registry, project/workspace/coordination identification and DB connection setup |
| `src/reader.rs` | Policy authorization, safe verified file reads and inventory |
| `src/storage.rs` | Index generation, checkpoint and handoff persistence |
| `src/search.rs` | Search and language structure lookup |
| `src/work.rs` | Durable local task/agent/check/review state services |
| `src/context.rs` | Planned context selection/serialization integration |
| `src/main.rs` | CLI argument/output routing and initial Application facade |

The initial facade must keep domain rules callable without CLI rendering. The remaining broker/session/budget/operations/output/filter/graph/pack services are proposed boundaries, not assertions of existing files. Schema and DB connection changes still have one integration owner.

## Dependency groups and integration gates

| Group | Work scope | Required input / integration output | Gate / intended evidence |
| --- | --- | --- | --- |
| G0 foundations | PCTX01–04,06,19,59–62 | Environment inventory; Domain/JSON/error/config contracts; explicit root/registry and separate databases; safe reader | Envelope/argument/offline/root/path/future-schema fixtures; no repository-triggered execution |
| G1 first exploration | PCTX05,07–12,14–16,58 | Verified bytes and ready generations → search/outline/current reads/scoped rules/budgeted build/checkpoint/handoff | Actual CLI init → index → find → outline/read → build; AC01–21 and invariant failures |
| G2 durable local collaboration | PCTX20–27,65 | Control migrations + shared revision/actor contracts → task/agent/run/events/check/submission/review/board/backup | Actual offline create → assign/claim → report → check → review → complete → board; AC23–38 including concurrent claim/CAS |
| G3 shared continuity | PCTX28–32; early manual PCTX37 | Ready index/control/session contracts → broker/snapshots/singleflight, brief/full/ack/delta/inbox, usage and deterministic capsule | Five-process upstream dedup; new epoch refuses ack; forced interruption restores without model call; AC39–49 |
| G4 operational policy | PCTX33–36,38,61,63,65 | Actor policy, trusted action bindings, host job identity → decisions/roles/resources/runner/PR gates/schedule registry/inventory/drift | AC45–57,60–62; live child retains lock; pause/silence wins; DST and WIP race fixtures |
| G5 execute, compress, retrieve | PCTX40–44,47,64 | One trusted runner + artifact store + verified parser input → output receipts, builtin typed parsers/filter fixture CLI, diagnostic extraction/Boolean/adaptive context | Registered unit check → compact failure → full same-record retrieval → extract source; AC63–74,76–77,81 |
| G6 adapters and later contracts | remaining PCTX37; PCTX45–46,49–53,66 | Stable Application services → capability-scoped Claude integration, static graph, multi-root, metadata archives, source packs, read API, safe external outbox | AC58–59,75,78–80 plus AX01–04,15,18; fixture/live coverage separately recorded |
| G7 measured release | PCTX13 interface,17–18,39,48,54–57 | Integrated flows, corpus and measurement provenance → install/update/remove/docs/licenses/archives and honest quality/cost reports | AC22,82; AX05–09; actual platform runners and fixture/live adapter evidence distinct |

Order within a group follows the item dependencies in `requirements.md`. Groups are integration checkpoints, not a promise to implement unrelated items simultaneously. G2 can proceed once foundations are stable while G1 matures, but shared files remain single-owned. G3 brings the manual Claude and basic usage path forward as §38 requires; protected hooks wait for G4. G6 is part of the full goal, even where §3 assigns v0.2. Optional extensions do not obstruct mandatory byte-budget/local/manual paths.

## Contracts to freeze before delegation

1. Domain/API: identifier formats, envelope `schema_version`, stable error→exit mapping, compact/JSON/NDJSON modes, extension fields and child-exit exception; bounded final serialization and deterministic ordering.
2. Reader/policy: `AuthorizedPath`, scope/root ID, current union-deny policy, before/after hash validation, encoded filenames, inert external document input exception and metadata redaction.
3. Storage: separate migration versioning; WAL/SQLite fix version, ready publication, writer lock wait, pinned snapshot lifetime, transaction CAS/event/receipt atomicity and rollback/rebuild behavior.
4. Coordination: task versus run state, assignment uniqueness, capability delivery file, lease epoch and report sequence, submission/definition/target/evidence fingerprints, trusted review/owner provenance.
5. Artifact/runner: one supervisor, argv/cwd/environment trust, executable/script hash, host job/boot/process identity, concurrent streams and redaction-before-disk, fail-closed unknown child, artifact retention/expiry and reread without rerun.
6. Session/broker: scoped cache key and refresh generation, one ledger for emissions/delivery/acks, context fingerprint including all source revisions, tombstones, cumulative usage observations, mailbox effect idempotency.
7. Operations/transport: profile versus user trust versus durable policy, grant action binding, silent/pause precedence, schedule occurrence/timezone/misfire semantics, external intent reconciliation with honest unknown state.

Freeze these in schemas and failure fixtures, then implement through the shared Application layer. Adapter reports distinguish code implemented, protocol fixture passed, installed capability detected and live account verified. Credentials absent block only live verification that requires them.

## Verification and release progression

Use temporary project repositories, isolated data/trust directories and synthetic secrets. Test actual CLI processes and database/filesystem boundaries: multi-process claim/revision/singleflight/event replay, forced process death, child survival, file mutation/symlink escape, equal-stat changes, simultaneous pipes, malformed reports, expired artifacts, tampered parts and DST. Tests inspect structured states and durable artifacts, not a successful phrase.

Each integration checkpoint records its code revision, exact reproduction command and outcome in `docs/evidence/` and updates requirements/progress/handoff. Mark implemented only after integrating working behavior; verified only with the corresponding successful checks. A denied host action, untested platform or unavailable service remains honestly blocked/unverified. A planned help entry must return unavailable rather than false success.

Run one heavy build/full test/benchmark at a time. Before host rules are confirmed, at most one active subagent; subsequent concurrency follows actual tools and applicable project/host instructions. The beSir capacity profile is a product fixture and does not itself determine development-host permissions.

After safety/contract checks, benchmark the §18 M corpus and §27 control corpus, keeping cold/warm/OS values distinct and using at least 30 warm samples. Search uses at least 60 labeled queries. The §19/38/45 cost harness fixes snapshot/task/model/policy and includes followup reads, questions, recovery and coordination. Actual paid evaluations require their final authorization; implement and run free reproducible harnesses first. Missing actual usage never becomes zero or a token/cost success claim.

Prepare supported archives/checksums/license notices/SBOM, install/update/remove/migration/recovery and representative command examples. Current Mac evidence does not verify Linux/Windows. Commit/push for the user-requested development GitHub repository is authorized by the goal; package publication, operating-project deployment/account mutation/global hook or OS schedule installation and paid calls are not authorized by the specification alone.

## Initial project observations and next integration actions

The initial tree contained the specification and local metadata/toolchain preparation, with no existing product implementation observed at the start of this analysis. No AGENTS.md was found at the project root or its ancestors. The integration owner later confirmed an empty user AGENTS.md, a 10-CPU/16GiB host, project-local Rust 1.99 under ignored `.toolchain`, a new main branch and the requested Jung95/pctx remote. Up to three subagents are available after host confirmation; heavy builds remain serialized. Authoritative evolving Git/toolchain/process results belong in progress/handoff.

Historical initial integration order (superseded by the serial official-task rule): main integration completes dependency acquisition, common contracts/scaffolding and PCTX01–04/06. Review first actual CLI evidence before changing trace statuses. Then complete the safe exploration and durable task vertical flows; preserve later concrete work in the active backlog.

## D045 integration

Main integrates the task-file boundary and exact-output provenance. work_control owns the bounded pinned-input primitive/direct fixture authoring; requirements independently reviews authority/deadline/alias contracts without edits or test execution. Main runs native CLI/race/static/full verification and commits/pushes development work with no Actions dispatch. Shared reader identity support stays with main. The original-owner reporting-exception/release/priority contract and broader approval/raw-input memory delivery remain subsequent mandatory work, alongside performance/native startup/platform/recovery gates.

## D046 integration

work_control authors the new policy-controls module/schema only; main owns operations/message/deadline and restore integration, tests, public docs and native verification. requirements independently reviews invariants without edits/tests. Follow-ups retain broader automatic source/memory/approval exception qualification, authenticated external owner input, platform/recovery, finite Work/adapter budgets, ranking and native startup/performance gates. No role/report exception slice can complete the full goal.

D047 reporting source-attribution slice: shared authored source classification, source-label/packet-topic conflict union, explicit original-owner cross-topic references, unknown-source hold, and final role/recipient plus Inbox batch expiry/source proofs. Actual CLI covers mixed restricted sources and managed metadata changes blocking Inbox/ack; three controlled helper tests cover real expiry and last-admission/batch source changes. Independent review identified both sequential admission boundaries and found no remaining blocker in the scoped repair. See report-source-verification.json for exact final evidence. This does not verify source-body attribution truth, atomic final output, all memory/approval/receipt/Pack variants or native Windows/Linux; AC77 and the whole mandatory goal remain implementing. No Actions dispatched.

D047 final native55189 terminal101:363 PASS/3 original query startup FAIL. Related policy7/source11/operations11/session13/schedule17 PASS, controlled helper races3 PASS, format and locked all-target Clippy PASS. report-source-verification.json binds exact source hashes, retained initial compile/static failures and final logs. No new Actions.

D048 search candidate admission implements the proposed pinned-reader optimization for body/freshness search without matching ambiguous public error codes. Initial candidate-local rejection is separated from root/config/deadline and post-admission failures; ordinary reader retry/timeout partial contracts remain distinct. Independent review caught and repaired a non-Unix missing-candidate regression. See search-candidate-verification.json for exact evidence. Native Windows reparse/access behavior and a new immutable M30 remain required; the historical 2691.926416ms body-search p95 is still a target miss, not superseded by unit tests.

D048 final native18183 terminal101:367 PASS/4 original query startup FAIL. Candidate admission tests5 PASS, related search/reader/security/CLI/source/task/session tests PASS, format and locked all-target Clippy PASS. search-candidate-verification.json binds source hashes/final logs and retains initial static permission-literal failure. No Actions or new benchmark result.

PCTX01 remains the only active official task (`implementing`). Help/color integration: native52787 terminal101,377 PASS/3 original query startup FAIL across45 suites; actual frontend8 PASS and explicit leaf-contract coverage1 PASS; format/locked all-target Clippy PASS. Evidence: pctx01-help-color-verification.json. Help now states standard JSON schema1.0, per-leaf read/write/execute and conditional flags, native hook/NDJSON and Pack output exceptions. Parser diagnostics mask reflected secrets and escape controls; no-color applies before nested help parsing, qualified on a real Unix PTY. Independent review findings resolved. Original failures are retained with unchanged one-second budgets; native Linux/Windows, remaining finite routes and full argument/envelope/exit matrix are still required. No next official task selected; no additional Actions dispatched.

## Current PCTX01 handoff input admission

PCTX01 remains the only active official task (`implementing`). Handoff Create/Update/Show now share existing nonempty ASCII name grammar before CLI discovery and producer effects. Existing bounded explicit-file input handles Create/Update under the original request clock and1MiB cap before checkpoint; &Path, cwd-relative files, literal dash and regular-file symlinks remain supported, with no new stdin mode or CLI timeout route. Actual invalid names leave missing root/data/output untouched; sparse oversize/invalidUTF8 and FIFO refusal preserve initialized state/checkpoints. Direct invalid-name/expired-original-clock checks and valid create/update/show/collision-preserved handoff body pass. Initial target-selection101 ran no tests; first related47PASS1FAIL was macOS APFS refusing nonUTF8 fixture creation EILSEQ, retained. Repaired related54PASS: macOS actual opaque argv is IO_ERROR7/no effects; native Linux successful opaque filename remains required. Three controlled losses each actual0PASS1FAIL/exact restore. Initial Clippy101 fixture type/constant style fixed without behavior weakening; final format/locked all-target Clippy PASS. Final native full39587 terminal101:452 PASS/1 retained original startup FAIL. Evidence: pctx01-handoff-input-verification.json. Independent scoped review no blocker. Filesystem calls remain cooperative; Windows nonregular-open behavior and broader PCTX11 handoff schema/rollback/recovery are unqualified. Full PCTX01 frontend/platform/startup gates remain open; no next official task or Actions.

Preceding refusal runtime95c09f2a52ce5debb79ec15d210ca7b045ae5e79/full11657:447PASS1FAIL is bound in pctx01-refusal-rendering-verification.json; prior pure argument and inventory proofs remain separately retained.


## Preceding PCTX01 native startup isolation

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
[evidence manifest](evidence/pctx01-hook-verification.json) for initial failure,
targeted/static/full results and source hashes. Independent scoped review found
no blocker. Native Unix pipes are not live Claude or native Linux/Windows proof.
The remaining PCTX01 frontend/phase/platform and original startup gates remain
mandatory; no next official task is selected or Actions dispatched.


## PCTX01 manual-run frontend outcomes

Current PCTX01 frontend run outcome slice uses the same producer supervisor with
a native spawn observer; no independent runner feature was added. Next local work:
remaining pure parser/preflight and budget/representation matrix, post-spawn error
observation, phase races and original startup cause; required native Linux/Windows
gates remain. See [manifest](evidence/pctx01-run-exit-verification.json). Do not
select another official task or dispatch Actions.


## PCTX01 Run refusal and capacity truth

Continue only PCTX01 after the current integration boundary. Next evidence:
actual post-spawn presentation fallback and observer-error outcome, then remaining
oversized-error/argument/representation/phase and original native startup gates.
Native Linux/Windows qualification remains mandatory.
[Current boundary](evidence/pctx01-run-budget-verification.json); no next task or Actions.


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
blocker. See [evidence manifest](evidence/pctx01-delivery-ownership-verification.json).

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
scoped blocker. [Evidence](evidence/pctx01-run-final-bytes-verification.json).

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
[Evidence](evidence/pctx01-budget-fallback-verification.json).

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
[Evidence](evidence/pctx01-outline-coverage-verification.json).

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
[Evidence](evidence/pctx01-nested-execution-verification.json) retains initial
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
[Evidence](evidence/pctx01-startup-counterbalanced-verification.json) records exact
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
limits remain. [Evidence](evidence/pctx01-startup-timeline-verification.json).
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
[Evidence](evidence/pctx01-callback-native-verification.json) preserves initial
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
versus INVALID_ARGUMENT. [Evidence](evidence/pctx01-output-admission-verification.json).
[Concrete remaining admission gates](tasks/PCTX01-admission-audit.md): Checkpoint
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
[Evidence](evidence/pctx01-checkpoint-admission-verification.json). Independent
static review no scoped blocker. Next common defect is ContextGet pure admission;
PCTX01 stays implementing, no other official task, Actions or rejected-push retry.

Final format/locked all-target Clippy PASS. Final native full97981 terminal101: 493 PASS/3 FAIL; original relative resolution/concurrent startup/simultaneous-stream failures retained without budget relaxation. All jobs terminal; PCTX01 incomplete.

## PCTX01 ContextGet pure admission

Only PCTX01 remains active and incomplete (PCTX01-G03). Shared existing
mode/full+since/explicit normalized relative path/budget rules now precede CLI
discovery/response paths and producer session DB. The original request deadline
is checked first for library callers, and valid requests retain project-session
and delivery authorization. Explicit dot components remain forbidden; Checkpoint
glob grammar is separate. No independent session feature was added.
Native18-case missing/initialized root plus absent/existing absolute response
snapshots pass; direct producer/original-expiry proof and normal full/delta
absent-session rejection pass. Existing seven session tests cover receipt/ack/
delta/epoch/policy. Final related10 PASS. Initial0PASS2FAIL and the extra fixture's
incorrect error-project_id assertion failure are retained. Two intentional losses
fail, exact source restored; format/locked all-target Clippy PASS. Independent
read-only review found no scoped blocker. Native Linux/Windows and the remaining
whole PCTX01 gates are open. See
[evidence](evidence/pctx01-context-admission-verification.json).

The existing ten completion rows now have stable IDs PCTX01-G01–G10; previous
and current denominator10, no scope added, whole gates closed0/10. This counts
whole mandatory gates, not effort, local passes or elapsed time. Source boundary
review identifies Repo Status workspace/fields as the next existing G03 admission
gap. Reuse its current grammar before discovery; do not activate PCTX28 features.
The latest goal permits an externally blocked task switch only after available
local work is exhausted and exact resume conditions recorded. PCTX01 has local
work remaining, so no switch is justified. No next official task or Actions.

Final native full21292 terminal101: 496 PASS/3 FAIL; original native startup failures retained at unchanged budgets. All jobs terminal; PCTX01 incomplete.

## PCTX01 Repo admission and coverage propagation

Only PCTX01 remains active (G03/G06). Existing workspace current and comma field
rules now share broker repo_fields, reused before main discovery and producer DB.
Duplicates stay accepted; empty/unknown/whitespace entries still reject2. Actual
32-case JSON/compact missing/initialized project and absent/existing response
snapshots pass unchanged. Direct producer and zero-timeout admission preserve
state. Native valid field selection on Git stays0; nonGit existing innerunsupported
now propagates to outerunsupported/error6 instead of outercomplete/0. Broker
claims/refresh/policy rules remain unchanged, no independent PCTX28 feature added.

Initial1PASS2FAIL retained: missingrootIO7 masks2; actual nonGit0 contradicts
unsupported6. Related13PASS includes9 existing broker process/concurrency tests.
Three intentional losses each0PASS1FAIL/exact source restore: frontend7vs2,
producer accepts invalid workspace and publishes a snapshot, coverage0vs6.
Format/locked all-target Clippy PASS; independent read-only review no blocker.
Dedicated new partial-status CLI fixture and native Linux/Windows remain
unverified. Existing ten gates remain0/10 closed (denominator10 unchanged); G06
correction was required by the actual initial unsupported failure, no new row.
See [manifest](evidence/pctx01-repo-admission-verification.json).
Next existing G03 gap: Pack Plan scope/content/budget/split admission before main
discovery, not independent PCTX47 functionality. No next official task or Actions.

Final native full76492 terminal101: 500 PASS/3 FAIL; original relative resolution, concurrent startup and simultaneous-stream failures retained at unchanged budgets. All jobs terminal; PCTX01 incomplete.

## PCTX01 Pack admission integration

Only PCTX01-G03 is active. Exact existing Plan scope/content/budget/split grammar
now shares plan_arguments before main discovery and producer delivery/task/source;
Create plan IDs share validate_plan_id before discovery and plan-directory access.
Original execute clock and error2/8 remain unchanged. Native36 Plan and20 Create
JSON/compact missing/initialized tree/response/artifact snapshots pass. Direct
producer pure errors and original expiry preserve state. Metadata plan duplicate
scopes succeeds with64000 budget/part; separate512 part remains a required8
refusal when an item cannot fit. No independent pack export/publication feature.
Related15 PASS including11 existing pack policy/stale/tamper/security tests.
Initial1PASS2FAIL, wrong flattened task fixture and actual512 item-size failure
are retained in distinct logs; neither product bounds nor deadlines changed.
Create initial0PASS1FAIL retains IO7mask2. Four controlled losses each0PASS1FAIL,
exact source restored. Format/locked all-target Clippy PASS. Independent read-only
review found no scoped blocker; required Linux/Windows and whole gates remain open.
[Evidence](evidence/pctx01-pack-admission-verification.json).

The admission source inventory now has stable IDs A01–A11. Previous6 groups,
current11: added5 are existing Quota, Work/Agent, Schedule, Session Attach and
Role pure-grammar omissions identified with source references under §14. No new
business feature or official gate; global PCTX01 denominator10 unchanged, closed
0/10. Local evidence recorded6/11 inventory groups; whole platform-qualified
inventory groups closed0/11. Next work is that bounded five-family common matrix,
keeping registered IDs/revisions/authorization/current-state rules in producers.
No other official task is active and no Actions are dispatched.

Final native full86052 terminal101: 503 PASS/4 FAIL; original captured-path resolution, relative resolution, concurrent startup and simultaneous-stream failures retained at unchanged budgets. All jobs terminal; PCTX01 incomplete.

## PCTX01 control-family common admission — 2026-10-08

Only PCTX01 is active. Specification §14/§38; fixed G03-A07–A11 now share exact
existing pure Quota, Work/Agent, Schedule, Session Attach and Role grammar before
CLI discovery/output and producer DB effects. Registered IDs, workspace policy,
revision/current state and authority remain producer checks. Report max-age is
validated even with no observations. Original supplied deadlines are checked
first; no new default write clock. Private schedule plan retains the shared
provider/root guard for reviewed Install/Uninstall callers.

Native132 refusal combinations preserve project/data/response bytes; reviewed
suite9 PASS covers direct original expiry, normal state flows, non-owner policy
and malformed reviewed provider. Related prior71 PASS is separately qualified.
Seven controlled losses fail with exact restoration. Initial compile/target/
schema-assumption/lint failures and actual reviewed-provider panic are retained.
Final formatting/locked all-target Clippy PASS. Independent review identified
the private-plan regression, then verified the repair; no remaining scoped
blocker. Evidence: evidence/pctx01-control-admission-verification.json.

Inventory denominator unchanged11: local evidence11/11, required-platform groups
closed0/11. Whole PCTX01 fixed gates closed0/10. This bounded inventory is not an
exhaustive G03 claim. Linux/Windows and native startup/full remaining gates stay
open. Next audit the complete command registry against pure/state-dependent
admission before choosing further necessary PCTX01 work. No other official task,
Actions, OS registration or independent business expansion.

Final native full31923 terminal101: 512 PASS/4 FAIL. Failed tests: relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, query_program_resolution_uses_captured_command_path, simultaneous_streams_are_collected_and_overflow_never_infers_success, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget. All jobs terminal; no heavy job live. Whole PCTX01 remains incomplete0/10; no other official task selected.

## PCTX01 complete visible admission register and runner correction — 2026-10-08

Only PCTX01 is active, specification §14/§38 and fixed G03. The previous turn
changed code/evidence/publication and is classified as progress. Actual help
enumeration at d0dbaef observes172 visible paths/140 leaves without data/project
effects. tasks/PCTX01-command-admission-registry.md assigns C001–C140 and freezes
R01–R12 currently identified residual groups after source inspection. This
replaces serial rediscovery; the earlier11/11 bounded local groups are not whole
G03 coverage. Official gate denominator10 remains unchanged, closed0/10.

Independent read-only audit covers all Operations, Work, Session, Context, Quota,
Schedule and Inventory routes. Remaining existing pure predicates include owner/
inbox bounds, recipient/evidence grammar, stronger downstream Role controls,
Agent Report, Session reasons, Ingest key, inventory paths, execution argv,
filter/adapter grammar and schedule variants. Library-only Clap-protected checks
are distinguished. Raw input/state/authority policy stays with producers.

Current repair is R12 only: exact existing runner JOB/HELP identifiers and
HelperRequest mode/count/nonsecret/nonlocal explicit-scope predicates shared
before CLI discovery and producer host/helper/DB effects. Private callers retain
shared defenses; original supplied expiry checked first, no new default write
clock. Job cancel alias uses the same validator. No provider, containment,
registered-check feature or other official task is added. Initial CLI exit7
masking and direct host directory creation are retained; first direct fixture
incorrectly relied on default global data resolution and is explicitly corrected
to local Project. Target-selection failure ran no tests. Completion requires
actual missing/initialized JSON/compact+response no-effects, direct unchanged
storage/expiry, accepted/rejected bounds, valid non-owner and queued intent/source
policy, independent review, controlled loss, static and final full evidence.
No Actions or native OS registration.

Initial native full46531 terminal101:514PASS5FAIL, retained. Four original startup
failures plus query_deadline_cli positive timeout fixture used malformed H001.
Corrected only ID to valid HELP-001; zero/positive timeout and no-effects assertions
retained, invalid IDs stay in new matrix. Targeted query/control21PASS and final
staticPASS. Final exact-source full38671 is running; do not promote initial full.

Final native full38671 terminal101: 517 PASS/2 FAIL. Failed tests: concurrent_query_startup_preserves_group_streams_and_original_one_second_budget, simultaneous_streams_are_collected_and_overflow_never_infers_success. All jobs terminal; no heavy job live. Whole PCTX01 remains0/10. R12 has local evidence; remaining R01–R11 and required platforms/full gates open. No other official task.

Two previously failing resolution tests pass in the latest full without any
query-process/test or budget correction. This is not repair evidence; all four
historical startup conditions remain unresolved. No retry-green promotion.

## PCTX01 Operations common admission — 2026-10-08

Fixed G03-R01–R03, §14/§38; only PCTX01 active. Exact existing owner/inbox bounds,
message recipient/evidence grammar and downstream Role role/topic/reason/flag
relationship now shared before discovery/DB. Private scheduled enqueue retains
shared recipient checks. Operations raw recipient labels depend on registered
alias resolution (recipient_key); inventory's pure-recipient claim corrected,
not a requirement removal. Actual leading-space/tab aliases retain pause/resume.

Native60 refusal combinations preserve project/data/response; six direct original
expiry/no-schema checks, Queue/Inbox1/1000, Role256/4096 versus257/4097, helper
bounds and four non-owner routes pass. Final related48PASS, three controlled
losses fail/exact restore; final staticPASS. Independent read-only scoped review
no blocker. Manifest evidence/pctx01-operations-admission-verification.json.
Initial owner-queue failure and incorrect target selection remain preserved.
Residual inventory denominator12 unchanged: local evidence4/12 (R01–R03,R12),
whole required-platform residuals0/12; whole PCTX01 gates0/10. Native Linux/Windows
and historical startup failures remain; no other official task or Actions.
Next same-task boundary R04–R06 existing Work Report/Session reasons/Ack provenance/
Quota Ingest key. Registered IDs/leases/current session/observation input remain
producer responsibilities; do not add independent features.

Final native full35198 terminal101: 518 PASS/4 FAIL. Failed tests: query_program_resolution_uses_captured_command_path, relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget, simultaneous_streams_are_collected_and_overflow_never_infers_success. Original budgets unchanged; all four historical startup conditions remain unresolved. All jobs terminal; no heavy job live. PCTX01 remains0/10.

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

Evidence: `evidence/pctx01-report-admission-verification.json` and review/logs.
Fixed residual denominator remains12: local 7/12 (R01–R06,R12), whole required
platform 0/12. Whole PCTX01 gates remain 0/10. Linux/Windows remain unverified and
all four historical native startup conditions remain unresolved regardless of
incidental later passes. Next local boundary is R07 within PCTX01. No next official
task or Actions is selected.

Final native full44671 terminal101: 521 PASS / 4 FAIL. Failed tests: relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, query_program_resolution_uses_captured_command_path, simultaneous_streams_are_collected_and_overflow_never_infers_success, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget. All four historical startup conditions remain unresolved; original budgets unchanged. All jobs terminal; no heavy job live. PCTX01 remains incomplete (0/10 whole gates).

## PCTX01 inventory path admission — 2026-10-08

Only PCTX01 remains active, fixed G03-R07 and G08, specification §14/§38.
Shared Inventory request validation reuses the exact reader lexical predicate for
Profile/Audit/optional Scan profile before CLI discovery. Direct calls check the
original supplied expiry, then Scan limits, then lexical paths before query_scope
root/policy checks or inventory reads. Invalid paths retain PATH_OUTSIDE_ROOT5;
invalid Scan limits retain INVALID_ARGUMENT2. No new path syntax or profile,
registry, policy, authority or inventory business contract was introduced.

Native macOS: 15 invalid route/path forms across 60 JSON/compact and missing/
initialized project combinations preserve project/data and absent/existing absolute
response bytes (response existence follows project initialization). Five paths
through six direct/dispatch entries refuse PATH_OUTSIDE_ROOT5 before a mismatched
root anchor; six expired path entries and eight expired limit entries retain the
same original Instant and TIMEOUT7 with unchanged storage. Scan invalid limits
retain priority over an invalid profile for nonexpired requests. Actual accepted
Profile/Audit/Scan, internal-dot/repeated-separator paths, Scan without a profile,
policy denials and Audit baseline mismatch preserve state and no operation/script
execution. Trailing separators/whitespace names and exact bounds are shared pure
validator proofs only. Final related 40 PASS; five controlled losses each 0 PASS /
1 FAIL with exact restoration; format/locked all-target Clippy PASS. Independent
read-only review found no blocker.

Initial 0 PASS / 4 FAIL mixes actual product failures and fixture mistakes (duplicate
format, expected root error, project ID assumption). Corrected unchanged-product
baseline 1 PASS / 3 FAIL proves CLI IO7 masking path5, root POLICY_DENIED masking
PATH_OUTSIDE_ROOT and Scan invalidity masking original expiry. Both logs retained.
Evidence: `evidence/pctx01-inventory-admission-verification.json`.
Fixed residual denominator12 is unchanged: local 8/12 (R01–R07,R12), whole required
platform 0/12; PCTX01 whole gates 0/10. Native Linux/Windows and four historical
startup conditions remain open. Next same-task boundary R08 existing execution
argv grammar; no next official task or Actions selected.

Final native full99805 terminal101: 527 PASS / 3 FAIL. Failed tests: concurrent_query_startup_preserves_group_streams_and_original_one_second_budget, relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, simultaneous_streams_are_collected_and_overflow_never_infers_success. Captured-command-path test passed without any startup/resolution code repair; that pass does not qualify a fix. All four historical startup conditions remain unresolved and original budgets unchanged. All jobs terminal; no heavy job live. PCTX01 remains incomplete (0/10 whole gates).

## PCTX01 execution input admission — 2026-10-08

Only PCTX01 remains active, fixed G03-R08/G08, specification §14/§38.
Exact existing argv collection grammar (nonempty, at most256 elements, at most65536
UTF-8 bytes including argv0) and Run modes now share validators before CLI project
access and inside binding/run/trust producers. No new restriction on individual
empty elements, whitespace, Unicode or option strings. Supplied producer expiry
is first; direct Run budget below3000 retains BUDGET_TOO_SMALL8 before modes/argv.
Trust Add creates no new default query clock. Exact fingerprints, owner authority,
resolution/classification/script policy, registered bindings and execution clocks
remain producer contracts.

Native macOS: six invalid route/argv forms across24 JSON/compact and missing/
initialized project combinations preserve project/data and absent/existing absolute
response (response existence follows initialization); caller actor is nonowner.
Six Run requests through library and frontend-observation entry plus six Trust
requests refuse2 without storage; the same18 entry calls with supplied expiry
retain original Instant/TIMEOUT7/not_started/no storage. Isolated nonowner library
Trust Add distinguishes invalid2, valid authority5 and expired7. Pure validators
qualify all modes, empty elements,256/257 count and65536/65537 UTF-8 byte boundaries.
Actual Unix Trust Plan accepts256 elements and65536 bytes without executing or
writing trust; valid nonowner Add denies5 and wrong fingerprint Add denies9.
New positive boundary proof is planning, not child execution. Existing related
execution/output/deadline/delivery suites provide separate child-regression proof.

Related34 outer PASS (one nested child PASS is not an extra suite test); final
isolated5PASS after child stdout suppression. Four controlled losses each0PASS/
1FAIL and exact product-source restoration. Initial static assertion-style failure
retained; equivalent assert syntax repaired, final format/locked all-target Clippy
PASS. Independent read-only review found no blocker. Initial unchanged-product
baseline0PASS2FAIL proves CLI IO7 masks argv2 and invalid Run masks original expiry.
Evidence: `evidence/pctx01-execution-admission-verification.json`.
Fixed residual denominator12 unchanged: local9/12 (R01–R08,R12), whole required
platform0/12; PCTX01 whole0/10. Native Linux/Windows and four historical startup
conditions remain open. Next same-task boundary R09 existing filter grammar;
no independent execution feature, next official task or Actions.

Final native full98801 terminal101: 531 PASS / 4 FAIL. Failed tests: query_program_resolution_uses_captured_command_path, relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget, simultaneous_streams_are_collected_and_overflow_never_infers_success. All four historical startup conditions remain unresolved and original budgets unchanged. All jobs terminal; no heavy job live. PCTX01 remains incomplete (0/10 whole gates).
