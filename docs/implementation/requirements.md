# Implementation requirements

Baseline: `docs/PCTX-implementation-spec-v0.6.md` (2026-10-08), all 45 sections, and the user goal objective. Public implementation documents are English. Release stages order implementation; they do not remove concrete later contracts from the goal.

Status vocabulary: `not_started`, `implementing`, `implemented`, `verified`, `blocked`. `implemented` requires integrated working code; `verified` additionally requires recorded execution evidence. Rows now track initial implementation and analysis separately; partial module coverage remains `implementing`. `planned:` paths identify intended artifacts and are not claims that those files exist. Main integration owns status changes after inspecting code and evidence.

Logical module names below describe Application/Domain boundaries; current flat source mappings are recorded in `plan.md`. All commands, adapters and tests must invoke the same domain rules.

## Sequential execution control (2026-10-08)

Only **PCTX01** is active. It is the lowest-numbered incomplete official development item and has no prerequisites. All other incomplete rows are backlog status, not concurrent active work. D-series records describe historical implementation slices; they are not substitutes for whole official-task completion. Finish all PCTX01-owned required CLI/help/version/argument/envelope/error/budget/exit/common-timeout contracts and required platform qualification before selecting PCTX02. Shared acceptance IDs retain their full conditions; record which common contract PCTX01 owns and which independent feature belongs to a later task. Do not mark a whole shared AC verified from PCTX01-only proof.

## Current G07-D03 closure — 2026-10-09

Only **PCTX01** is active and remains `implementing`. **G07-D03 is closed for
its bounded local transport contract**: **32/40** details, **0/10** whole gates;
all fixed denominators unchanged. [Evidence](evidence/pctx01-transport-controls-verification.json)
binds source, executables and logs: final172 focused tests across13 suites,
locked all-target Clippy -D warnings, format and independent read-only review PASS.
No production behavior change was necessary; test fixtures complete existing proof.

The [fixed transport crosswalk](evidence/pctx01-transport-controls-crosswalk.json)
covers78 controls. Synthetic9 success/partial/error documents preserve exact values
and atomic storage; native12 JSON/compact/Markdown success/error stdout/file cases
use77 controls (NUL is separately binary/path admission). Injected workspace metadata
is not generated-ID proof. Native4 hook success/refusal paths use untrusted controlled
input; success output is generated, controlled wire-output values are synthetic.
Injected SQLite event/stage fixtures qualify NDJSON and compact watch transport,
not ordinary domain publication. Per-control short Board/activity rows prevent
70-character projection masking. Corrupt SQLite produces one stderr JSON error
and no stdout. Existing parser diagnostics and real no-color PTY checks pass.

Next: **G08-D04 within PCTX01**, then G05-D05, one condition at a time. Required
Windows console proof remains G09-D04. Original four macOS1s startup product failures,
other required-platform gaps and inactive PCTX36/PCTX47 backlog remain open.
No Actions, whole/platform rerun or push retry. All validation jobs terminal.

## Historical G06-D03 closure — 2026-10-09

Only **PCTX01** is active and remains `implementing`. **G06-D03 is closed for
its bounded local common contract**: detail31/40, whole gates0/10, unchanged
scope/denominators. [Evidence](evidence/pctx01-producer-coverage-verification.json)
records final229 focused tests across18 suites, locked all-target Clippy -D warnings,
format and independent read-only review PASS. Source/binary/log hashes bind proof.

The frozen9-family [crosswalk](evidence/pctx01-producer-coverage-crosswalk.json)
separates successful empty0, required capability/resource absent6 and incomplete3.
Actual native JSON/compact controls cover applicable states. No-index Status null
is an optional inspection; real DB/policy failures propagate. Required source absence
is RESOURCE_NOT_FOUND6; already-admitted disappearance remains IO_ERROR7 and
project initialization absence remains NOT_INITIALIZED6. Both original regression
assertions stay strict. Filter requested JSON parsing partial and saved Output
incomplete capture now yield3. Native empty and140000-line bounded captures
preserve child0/no-rerun truth. Pack optional source validation remains inspection0;
requested current validation unknown is partial3, wholeartifact absence6, inner
integrity9/policy5 retained. Empty Pack scope still refuses2; no new empty-pack feature.

Initial syntax/schema/count/TTL fixture errors and actual product regressions are
retained separately. First related run48PASS1FAIL caught admitted-source priority;
second78PASS1FAIL caught missing-project classification. Corrections changed code,
not those expectations. One unchanged duplicate failure followed a failed fixture
edit script and is retained without treating it as improvement. Final229PASS is
focused common evidence, not whole-suite/all-producer/platform qualification.

Historical next (now closed): **G07-D03 within PCTX01**, remaining success/error control-character cases
for the already frozen representation/transport classes. No other official task
starts. Original four macOS1s startup failures and required-platform gaps remain
open; inactive PCTX36/PCTX47 backlog unchanged. No Actions, whole/platform rerun
or push retry. All jobs terminal; local integration only.

## Historical G04-D03 closure — 2026-10-09

Only **PCTX01** is active and remains `implementing`. **G04-D03 is closed for
its bounded local common contract**: detail30/40; whole gates0/10. Scope and
all frozen denominators remain unchanged. [Evidence](evidence/pctx01-final-capacity-verification.json)
records final111 focused tests PASS, locked all-target Clippy -D warnings PASS,
format PASS and independent read-only review with no material blocker.

After declared metadata reductions, remaining overflow is a complete
`BUDGET_TOO_SMALL` error under AC13, with exact requested/delivered bytes and
`limit_exceeded=true`; this is an explicit failure exception, not budget compliance.
Primary error/exit, available execution proof and reread handles remain intact.
20 overflow controls synthesize proof and use actual atomic-file delivery; they
are not native forced-oversized CLI executions. Native Markdown6 leaves x2
response destinations and stream4 routes x2 absent/existing destinations pass.
The 140-row crosswalk classifies existing source routes, not140 executed producers.
Linked prior Read24, identity/Adapter/error, metadata and native Run evidence
qualifies representative shared boundaries; independent producer semantics remain
with their existing owners. Initial product/test-fixture/invocation failures are
retained in logs, including Markdown diagnostic precedence and self-indexed SQLite.

Historical next was G06-D03, now closed by the current section above.
The next condition is G08-D04, within the same official task. Original four macOS1s startup
product failures, required-platform gaps and inactive PCTX36/PCTX47 backlog
remain open. No Actions, whole/platform rerun or push retry. All jobs terminal.


## Current fixed PCTX01 execution register — 2026-10-09

Only **PCTX01** is active, lowest incomplete official ID/no prerequisite. The
fixed [closure plan](tasks/PCTX01-closure-plan.md) and 40-item register now have
**32/40** locally closed, **0/10** whole gates; denominator40, 140 leaves/172 help
paths/12 historical R groups and required platforms are unchanged.

**PCTX01-G05-D02 is closed as bounded local native postspawn qualification**.
The [final fault manifest](evidence/pctx01-postspawn-runner-verification.json)
connects all seven existing phase groups to current controls and historical
callback/capture/receipt evidence. Final-source103 focused tests PASS
(49lib/5cancellation/11Run/38Runner); locked all-target Clippy -D warnings and
format PASS. Independent read-only review found no material blocker.

Shared Runner finalization now cleans up before evidence/Helper persistence,
retains completed execution through DB/receipt errors, holds ownership unknown
after guardian/owner failure and reports partial slot release as null/unconfirmed.
Failed evidence publication returns unverified/not-published without passing gate
evidence. Helper returned state is separate from its receipt publication. Actual
SQL writer lock, journal refusal after slot removal, Helper final receipt refusal,
and native guardian ECHILD are controlled; one invocation and available artifact
reread/native status are preserved. The backend Result::Err control models that
API boundary using a real completed-process fixture; it does not induce a failure
inside the shared output backend. Actual pipe/wait/worker faults are separate controls.

Initial new-fixture failures are retained: unsupported work-check-show path, and
Helper obstruction selector permitting staging files without establishing intended
final refusal. Corrections use existing check-show CLI and published HELP-*.json;
assertions are not weakened. Earlier callback/continuous-read/receipt/cancellation
failures and original four macOS1s startup product failures remain in evidence.
This is focused local closure, not a passing whole suite or platform integration.

Next exact condition is **PCTX01-G04-D03**: complete supported success/partial/error
envelope required fields and exact final serialized bytes across fixed
representation/destination classes. Use the existing140-leaf crosswalk, not new
producer features or another reorganization. Then existing G06-D03/G07-D03,
G08-D04/G05-D05, G10/G09 obligations remain. No next official task is selected
until every mandatory PCTX01 condition/platform is qualified.

Inactive PCTX36 Tick and PCTX47 Helper metering routing remain separate backlog.
Required macOS x86_64/Linux x86_64/Windows x86_64 native integration is unverified;
Linux aarch64 is supporting evidence only. No Actions or whole/platform rerun.
All jobs are terminal. Main skip-ci commit/push authorized; Node24 pins remain.

## Development items

| Work ID | Required behavior | Spec sections | Module owner | Dependencies (PCTX suffixes) | Acceptance IDs (AC suffixes) | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PCTX01 | CLI, envelope and stable errors | §5,8,14,38,45 | cli/application/domain | — | 01,13,18,63 | implementing | [main.rs](../../src/main.rs), [domain.rs](../../src/domain.rs), [render.rs](../../src/render.rs); [cli_contract.rs](../../tests/cli_contract.rs), [render.rs](../../tests/render.rs), [core_security.rs](../../tests/core_security.rs); [macOS regression log](evidence/third-full-macos-tests.log): one-document errors, safe Markdown and exact JSON budgets pass. [PCTX01 help/color evidence](evidence/pctx01-help-color-verification.json): every public help path, explicit schema/effects, real Unix terminal no-color and masked parser diagnostics pass; native full377 PASS/3 retained startup FAIL. [Work/quota deadline evidence](evidence/pctx01-work-quota-verification.json): CLI7/direct3/runner24 pass; full382 PASS/4 retained startup FAIL. [SQL engine/isolation evidence](evidence/pctx01-cpu-sql-verification.json): final384 PASS/4 retained startup FAIL; engine VM interruption and stale callback regression pass. [Auxiliary source budget evidence](evidence/pctx01-auxiliary-deadline-verification.json): propagation/expiry regression and original-loss mutation qualify opened-target source reads; final386 PASS/3 retained startup FAIL. [Argument preflight evidence](evidence/pctx01-argument-preflight-verification.json): common Read/Outline/Extract/Build/Graph validation before effects, preserved capacity precedence and final443PASS1FAIL. [Refusal rendering evidence](evidence/pctx01-refusal-rendering-verification.json): shared JSON/pre-effect transport, exact-min capacity, actual Unix closed-pipe I/O exit and repaired bidi diagnostics; final related38PASS, whole-task incomplete. [Handoff input evidence](evidence/pctx01-handoff-input-verification.json): shared pre-effect names, bounded explicit file admission, state-preserving input failure and compatibility proofs (related54PASS; whole-task incomplete). Remaining: additional phase/race coverage, complete argument/version/representation/error/exit matrix, remaining option-before-effect checks, original native startup root cause and native Linux/Windows qualification. |
| PCTX02 | Root discovery and project/workspace registry | §6,16,27 | registry | 01 | 01,02,06,25 | implementing | [project.rs](../../src/project.rs), [storage.rs](../../src/storage.rs); [cli_contract.rs](../../tests/cli_contract.rs), [core_security.rs](../../tests/core_security.rs), [runner.rs](../../tests/runner.rs); [macOS regression log](evidence/third-full-macos-tests.log): non-Git/idempotent IDs, foreign-index refusal and linked-workspace provider fixtures pass. Remaining: nested-root discovery, modified-config repeated init and two dirty worktree symbol-isolation/clone/move matrix. |
| PCTX03 | Configuration precedence and policy | §7,17,25,32 | policy/config | 02 | 09,10,17,21,46,69 | implementing | [project.rs](../../src/project.rs), [reader.rs](../../src/reader.rs), [operations.rs](../../src/operations.rs); [cli_contract.rs](../../tests/cli_contract.rs), [reader_policy_cache.rs](../../tests/reader_policy_cache.rs), [core_security.rs](../../tests/core_security.rs), [operations.rs](../../tests/operations.rs); [macOS regression log](evidence/third-full-macos-tests.log): current exclusion and exact-policy cache boundaries pass. Remaining: full defaults/user/project/env/CLI precedence matrix and security-only tightening/ignore override fixtures; finite query timeout is implemented on documented routes (PCTX68), broader coverage remains unfinished. |
| PCTX04 | Safe inventory and verified reader | §6,10,17,24 | reader | 03 | 05,09,10,12 | implementing | [reader.rs](../../src/reader.rs); [reader_policy_cache.rs](../../tests/reader_policy_cache.rs), [core_security.rs](../../tests/core_security.rs), [search.rs](../../tests/search.rs); [macOS regression log](evidence/third-full-macos-tests.log): pinned Unix reads, warmed root/ancestor replacement, private-key masking and oversize candidates pass. Remaining: deterministic mutation during read/parse/pre-output, invalid-encoding/binary/size inventory matrix and Windows equivalent proof. |
| PCTX05 | Literal and regex text search | §11,14,18 | search | 04 | 09,12,18 | implementing | [search.rs](../../src/search.rs), [reader.rs](../../src/reader.rs); [search.rs](../../tests/search.rs), [cli_contract.rs](../../tests/cli_contract.rs). Ninth native search22 cases and full274-test gate pass; [immutable M30](evidence/evaluation-m30-ninth.json) on sourced3d05b4/binary59cf89b… shows body30/30 complete, p952916.91025ms still misses2000ms, metadata28.608583ms/matched29.011625ms. Labeled exact/alias/partial/unsupported-language text retrieval has measured file Recall@20/MRR1, no wrong hashes; no unsupported structure claim. Tenth deferred physical body-snapshot optimization passes27 native search cases and full290-test Mac gate; performance unmeasured. Remaining: full finite-route/native deadline and snippet/range/limit matrix, measured2s body target, full task/model/hardware/cold-cache qualification. |
| PCTX06 | Index migrations and atomic generations | §9,10 | storage/index | 02 | 07,08,19 | implementing | [storage.rs](../../src/storage.rs), [project.rs](../../src/project.rs); [concurrency.rs](../../tests/concurrency.rs), [core_security.rs](../../tests/core_security.rs); [macOS regression log](evidence/third-full-macos-tests.log): 10-reader/2-writer ready-generation fixture, staged rebuild, corrupt quarantine and newer/foreign schema refusal pass. Remaining: forced kill/disk-full publication recovery, injected migration rollback and pinned-reader GC safety proof. |
| PCTX07 | Hashing and incremental index | §9,10 | index | 04,06 | 03,04,06,08,17,40 | implementing | [storage.rs](../../src/storage.rs), [reader.rs](../../src/reader.rs); [cli_contract.rs](../../tests/cli_contract.rs), [core_security.rs](../../tests/core_security.rs); [macOS regression log](evidence/third-full-macos-tests.log): changed source invalidation, scoped rename hints and checkpoint-preserving rebuild pass. [M/30 baseline](evidence/evaluation-m30-baseline.json): historical incremental p95 2822.10 ms and initial 16633.34 ms. [third M/30 report](evidence/evaluation-m30-third.json): incremental p95 2010.404875 ms meets 3000 ms; initial 15653.581125 ms meets 90000 ms (one initial sample). Remaining: full create/delete/rename/branch-switch and equal-size/equal-mtime fixtures. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. |
| PCTX08 | Python/JS/JSX/TS/TSX outline | §11,24 | adapters/languages | 07 | 11,12,18,22 | implementing | [search.rs](../../src/search.rs); [search.rs](../../tests/search.rs), [core_security.rs](../../tests/core_security.rs); [macOS regression log](evidence/third-full-macos-tests.log): real Python/JS/JSX/TS/TSX fixtures, parent symbols, CRLF bounds and parser partial/unsupported pass. Remaining: grammar-specific golden matrix for overloads, exports, anonymous declarations, directory/depth boundaries and actual Linux/Windows results. |
| PCTX09 | Metadata search, aliases and ranking | §11 | search | 08 | 11,15,18 | implementing | [search.rs](../../src/search.rs); [search.rs](../../tests/search.rs); [macOS regression log](evidence/third-full-macos-tests.log): aliases, ranking signals, candidate-only validation and stale candidate refusal pass. [M/30 baseline](evidence/evaluation-m30-baseline.json): historical metadata p95 1255.83 ms, errors 0/30; matched errors 30/30 and labels unmeasured. [third M/30 report](evidence/evaluation-m30-third.json): metadata p95 347.373833 ms misses 250 ms; matched p95 363.727958 ms meets 700 ms; warm errors all 0. All 60 labels measured: exact identifier/Korean alias/partial name/unsupported-language file Recall@20 and MRR=1; no-answer empty_correct 10/10, Recall/MRR null; wrong hashes 0. Remaining: metadata latency target, non-synthetic relevance and deterministic tie/alias-config matrix. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. |
| PCTX10 | Freshness modes and symbol reads | §8,10,11 | reader/index | 07,08 | 04,05,11 | implementing | [search.rs](../../src/search.rs), [reader.rs](../../src/reader.rs), [storage.rs](../../src/storage.rs); [search.rs](../../tests/search.rs), [core_security.rs](../../tests/core_security.rs); [macOS regression log](evidence/third-full-macos-tests.log): stale symbol IDs, duplicate names, explicit off/matched validation and strict refresh pass. [M/30 baseline](evidence/evaluation-m30-baseline.json): selected symbol range and stale ID refusal observed. [third M/30 report](evidence/evaluation-m30-third.json): matched completion errors 0/30 and p95 363.727958 ms; baseline completion failure superseded for this binary. Remaining: equal-size/mtime strict/verify-content and mid-read/pre-output race fixtures. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. |
| PCTX11 | Scoped rules and decision loading | §7,12,25 | context/documents | 03,04 | 14,17,77 | implementing | [documents.rs](../../src/documents.rs), [context.rs](../../src/context.rs), [session.rs](../../src/session.rs); [project_documents.rs](../../tests/project_documents.rs), [session.rs](../../tests/session.rs), [cli_contract.rs](../../tests/cli_contract.rs); [macOS regression log](evidence/third-full-macos-tests.log): scoped required documents, source hashes, unsafe YAML rejection, conflicts/supersession and oversized mandatory-rule failure pass. Remaining: explicit external documents, native-loader omission proof and complete decision/memory/role/topic invalidation; current decision/instruction and supersession cases are covered by D039 fixtures. |
| PCTX12 | Context selection and serialization | §12,14,43 | context | 09,10,11 | 13,14,15,76 | implementing | [context.rs](../../src/context.rs), [render.rs](../../src/render.rs), [main.rs](../../src/main.rs); [cli_contract.rs](../../tests/cli_contract.rs), [render.rs](../../tests/render.rs), [session.rs](../../tests/session.rs); [macOS regression log](evidence/third-full-macos-tests.log): exact final JSON budget, uncut required rules, explicit omissions/representations and packet budgets pass. [M/30 baseline](evidence/evaluation-m30-baseline.json): historical 30 deterministic in-budget builds. [third M/30 report](evidence/evaluation-m30-third.json): 30/30 deterministic builds within 12000 bytes, strict-build p95 2621.5675 ms meets 15000 ms. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. Remaining: complete role/seed/task/handoff ranking matrix, final Markdown downgrade/budget boundary proof, invalid-argument handling below the minimum failure envelope (current build guard uses BUDGET_TOO_SMALL), and selection-only cost measurement. |
| PCTX13 | Tokenizer interface; concrete engines optional | §12,21 | context/tokenizer | 12 | 13 | implemented | [context.rs](../../src/context.rs), [main.rs](../../src/main.rs): CLI tokenizer/budget-token inputs return CAPABILITY_UNAVAILABLE when no tokenizer is registered; byte budgets remain usable. Concrete engines are optional. Remaining proof: dedicated actual CLI unsupported-tokenizer regression and registered-engine extension/counting interface validation; no token estimates are presented as exact counts. |
| PCTX14 | Checkpoints and manifest changes | §13 | checkpoint | 07,10 | 04,16 | implementing | [storage.rs](../../src/storage.rs); [core_security.rs](../../tests/core_security.rs), [cli_contract.rs](../../tests/cli_contract.rs); [macOS regression log](evidence/third-full-macos-tests.log): scoped current hash manifest, unique names/checksums, rename hints, policy-filtered history and rebuild preservation pass. Remaining: equal-size/mtime manifest change, pinned checkpoint deletion/GC and concurrent snapshot/change matrix. |
| PCTX15 | Handoff create/update/validation | §13,14 | handoff | 11,14 | 16,20 | implementing | [storage.rs](../../src/storage.rs), [project.rs](../../src/project.rs), [context.rs](../../src/context.rs), [render.rs](../../src/render.rs); [core_security.rs](../../tests/core_security.rs), [render.rs](../../tests/render.rs); [macOS regression log](evidence/third-full-macos-tests.log): pinned handoff write rejects symlink redirection; rendered author provenance and validation remain distinct. Code creates/updates masked user-provided records with checkpoint references. Remaining: actual CLI normal create/update/show roundtrip, altered-source validation and checkpoint-used fresh full build proof. |
| PCTX16 | Doctor, GC, locks and interrupted recovery | §9,10,17 | maintenance | 06,07,14 | 07,08,19,34 | implementing | [main.rs](../../src/main.rs), [storage.rs](../../src/storage.rs), [project.rs](../../src/project.rs); [core_security.rs](../../tests/core_security.rs), [concurrency.rs](../../tests/concurrency.rs); [macOS regression log](evidence/third-full-macos-tests.log): read-only integrity code, serialized writers, quarantine/rebuild and newer schema protection; [M/30 baseline](evidence/evaluation-m30-baseline.json): historical actual retention GC completed; [third M/30 report](evidence/evaluation-m30-third.json): retention cleanup again returns 0, without independently proving pinned-generation/active-reader retention. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. Remaining: doctor permission/lock/location diagnoses, dry-run no-mutation, active-reader GC, interrupted migration/kill/disk-full recovery and pinned-generation retention proof. |
| PCTX17 | Performance and quality harness | §18,19 | evaluation | 05,09,12 | 15,22,82 | implementing | [generate-corpus.py](../../scripts/generate-corpus.py), [benchmark.py](../../scripts/benchmark.py), [evaluation fixtures](../../tests/evaluation); [sixth M/30 report](evidence/evaluation-m30-sixth.json): seeded M 10000-file/200MiB corpus, 30 warm repetitions, binary SHA256 `43d9317f…`, Darwin arm64; metadata p95 584.49ms and body p95 2589.32ms miss targets. Selection-only timing, controlled cold caches/reference hardware and task-level 30-task/3-repeat provider cost/noninferiority experiments remain unknown. An active later run is not evidence until recorded. |
| PCTX18 | Archives, installation docs and licenses | §21,22 | release/docs | 16,17,27,39,48 | 22,61,62 | implementing | [package.sh](../../scripts/package.sh), [notices.py](../../scripts/notices.py), [install.md](../cli/install.md); [archive validation](evidence/seventh-archive-validation.json): isolated macOS arm64 archive checksum/install/init/index/find/inventory/doctor/removal, project preservation and private-spec exclusion recorded. Earlier [install log](evidence/archive-install-validation.log) covers notices/SBOM. Remaining: three-OS archive installation, minimum OS/libc declarations proved by execution, future-version update/migration and channel-name availability before publication; no permission to publish inferred. |
| PCTX19 | Coordination detection and control migrations | §6,16,27 | storage/control | 02,06 | 19,25,34 | implementing | [project.rs](../../src/project.rs), [work.rs](../../src/work.rs): canonical Git common-dir coordination mapping, workspace-separated indexes and control schema/version checks; [macOS execution log](evidence/seventh-full-macos-tests.log) records work-control and foreign-index fixtures. Remaining: real two-dirty-worktree shared-board/code/run isolation plus independent clone/moved-root matrix, explicit coordination attach and injected control-migration rollback. Matching project IDs alone do not establish shared-board proof. |
| PCTX20 | Tasks, criteria, DAG and revisions | §27 | work/task | 19 | 23,33,35,37 | implementing | [work.rs](../../src/work.rs): task/definition revisions, weighted criteria, dependency reachability, transitions and expected-revision conflicts; [work_control.rs](../../tests/work_control.rs), [concurrency.rs](../../tests/concurrency.rs), [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: task depend add/remove CLI, list state/tag/workspace filters, parent/subtask leaf aggregation, exact DAG-cycle and scope-edit invalidation fixtures. Stored dependency support does not prove the full task contract. |
| PCTX21 | Agents, assignments, runs and leases | §27 | work/agent | 20 | 24,26,27,49 | implementing | [work.rs](../../src/work.rs): private capabilities, assigned/current-workspace claims, one-active-run transaction, epochs and heartbeat expiry; [concurrency.rs](../../tests/concurrency.rs): two_process_claims_have_one_winner; [work_control.rs](../../tests/work_control.rs): reassignment rejects old heartbeat; [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: stale-run report/check/submit matrix, host reboot/clock reconfirmation, role adapter automatic 30s heartbeat and agent report reference/next-action fields; Windows runtime is separate. |
| PCTX22 | Events and idempotent commands | §27,38 | work/events | 21 | 32,35,38,44 | implementing | [work.rs](../../src/work.rs), [watch.rs](../../src/watch.rs): append-only events/receipt triggers, create/report idempotency and monotonic report_seq; [work_control.rs](../../tests/work_control.rs), [concurrency.rs](../../tests/concurrency.rs), [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: receipts for all mutating commands, report-key payload conflicts and lost-success replay after lease revocation matrix; full files/check/evidence report references. Existing report receipts are not universal command retry coverage. |
| PCTX23 | Check reports and target/environment fingerprints | §27,33 | work/check | 10,22 | 28,29,30,55,64 | implementing | [work.rs](../../src/work.rs), [runner.rs](../../src/runner.rs), [output.rs](../../src/output.rs): scoped source/definition/policy fingerprints, native typed-report checks and runner-observed provenance; [work_control.rs](../../tests/work_control.rs), [concurrency.rs](../../tests/concurrency.rs), [runner.rs](../../tests/runner.rs), [macOS execution log](evidence/seventh-full-macos-tests.log). Native local profile/entry-tool/script/environment/PATH bindings are now rechecked at admission, recording, acceptance and completion; [eighth native log](evidence/eighth-full-macos-tests.log) covers changed executable/environment/profile and missing legacy authority. Remaining: complete transitive dependency/platform fingerprints, executable image ABA pinning, bounded explicit report attachment policy/TTL validation and declared external-input completeness; supported JUnit variant import absent. |
| PCTX24 | Review, acceptance and completion gates | §27,32,34 | work/gates | 20,23 | 28,31,35,37,55 | implementing | [work.rs](../../src/work.rs): immutable submission evidence hash, owner review and transaction-gated completion, historical done/current outdated distinction; [work_control.rs](../../tests/work_control.rs), [concurrency.rs](../../tests/concurrency.rs), [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: explicit old-submission review and true concurrent completion interleavings, configured independent-reviewer policy/self-approval matrix, prepare/hash outside transaction with revision recheck, full completion history retrieval and parent/subtask gates. |
| PCTX25 | Board, activity, watch and task context | §12,27 | work/board | 12,22,24 | 23,37,38 | implementing | [work.rs](../../src/work.rs), [watch.rs](../../src/watch.rs), [context.rs](../../src/context.rs), [main.rs](../../src/main.rs); [watch.rs](../../tests/watch.rs), [cli_contract.rs](../../tests/cli_contract.rs), [macOS execution log](evidence/seventh-full-macos-tests.log): finite/follow cursor resume, ephemeral heartbeat, board progress and task-bound context integration. Remaining: all task/agent board filters, leaf aggregation and §27 10000-task/100000-event/50-run load with board/report p95 and redraw ≤3s measurements; current small fixtures are not load proof. |
| PCTX26 | Consistent control backup and safe restore | §27,35 | backup | 19,22 | 34,36,61 | implementing | [work.rs](../../src/work.rs), [session.rs](../../src/session.rs), [quota.rs](../../src/quota.rs): SQLite online backup to new coordination, integrity/schema/checksum checks, interrupted runs/new epochs, revoked receipts/quota/schedule authority; [work_control.rs](../../tests/work_control.rs), [quota.rs](../../tests/quota.rs), [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: inspect/explicit workspace attach to restored coordination, complete secret/absolute-path exclusion across every portable table, adversarial archive/schema fixtures and actual new-host resource/bridge reconciliation; old live authority is not reusable. |
| PCTX27 | Coordination regression and load verification | §20,27 | evaluation | 24,25,26 | 23–38 | implementing | `tests/concurrency.rs`, `tests/work_control.rs`, `tests/watch.rs`; `docs/implementation/evidence/watch-tests.log`, `restore-quota-tests.log`; actual CLI/DB boundary regressions; normative coordination load not measured |
| PCTX28 | Query snapshots, singleflight and negative states | §29,38 | broker | 07,19 | 39,40,41 | implementing | `src/broker.rs`, `tests/broker.rs`; `docs/implementation/evidence/integrated-macos-tests.log`; local safe Git status/cache TTL and refresh claims; authenticated remote pagination/permission scopes unimplemented |
| PCTX29 | Session epochs, receipts, full/delta and brief | §13,30,38 | session/context | 12,21,28 | 16,42,43,58,76 | implementing | planned: `tests/pctx29.rs`, `docs/evidence/pctx29.md` |
| PCTX30 | Durable mailbox, transport, ack and quiet delivery | §30,38 | session/mailbox | 22,29 | 44,54 | implementing | Partial implementation evidence: tests/operations.rs; integrated-macos-tests.log |
| PCTX31 | Usage sources, pools, reservation and quota | §31,38 | budget | 19,21 | 47,48,81 | implementing | `src/quota.rs`, `src/adapter.rs`; `tests/quota.rs` (7 pass), `tests/adapter.rs` (10 pass in `docs/implementation/evidence/full-macos-validation.log`), `docs/implementation/evidence/restore-quota-tests.log`; imported source dedup/thresholds/soft reserves; live provider collectors and billing unknown |
| PCTX32 | Deterministic capsules and resume planning | §31,35,38 | handoff/recovery | 14,26,29,31 | 48,49,61 | implementing | `src/work.rs`, `src/session.rs`, `src/quota.rs`; `tests/work_control.rs`, `tests/quota.rs`, `docs/implementation/evidence/restore-quota-tests.log`; restored history/epoch revocation/quota observation; full deterministic capsule boundary/debounce/resume planning not complete |
| PCTX33 | Role policy, decision dedup and owner provenance | §32,37,38 | operations/policy | 20,30 | 45,46,54 | implementing | Partial implementation evidence: tests/operations.rs; integrated-macos-tests.log |
| PCTX34 | Host resources, legacy bridge and trusted runner | §33,38,40 | operations/runner | 23,33 | 50,51,52,63,69 | implementing | `src/runner.rs`, `src/output.rs`; `tests/runner.rs`; `docs/implementation/evidence/full-macos-validation.log` 19 passing scenarios after durable guardian acknowledgement correction; registered Unix guardian, host heavy/aux slots and legacy mutex; native pre-spawn/cloud/live platform coverage unknown |
| PCTX35 | Git/PR snapshots, WIP and release evidence | §27,34,38 | adapters/git/workflow | 24,28,33 | 41,55,56,57 | not_started | planned: `tests/pctx35.rs`, `docs/evidence/pctx35.md` |
| PCTX36 | Durable schedules, DST, tick and reconciliation | §35,38 | operations/schedule | 26,30,33,34 | 53,54,61 | implementing | `src/schedule.rs`, `tests/schedule.rs` (8 pass in `full-macos-validation.log`); durable namespace/revision/logical occurrence, DST, coalescing and policy barriers; schema-v2 managed bridge/tick execution/install and keep-awake extension authored, 17 local schedule tests pass in `second-full-macos-tests.log`; native OS registration not exercised |
| PCTX37 | Claude manual adapter, hooks and managed configuration | §36,38,41 | adapters/claude | 29,31,32,33 | 58,59,62,70 | implementing | `src/adapter.rs`, `tests/adapter.rs` (10 pass in `docs/implementation/evidence/full-macos-validation.log`); hash-bound local config plan/install/owned uninstall, fixture events/epochs/statusline; installed Claude capability/live connection and warm hook latency unknown |
| PCTX38 | Monorepo inventory, checks and document drift | §37,38 | adapters/inventory | 08,11,35 | 60,62,75 | implementing | `src/documents.rs`, `src/graph.rs`; `tests/project_documents.rs`, `tests/graph.rs`, `docs/implementation/evidence/full-macos-validation.log`; scoped document/source relations only; `src/inventory.rs`, `tests/inventory.rs`, `docs/cli/inventory.md`: static monorepo/check candidates and hash-bound DOC proposals authored, 8 local inventory tests pass in `second-full-macos-tests.log`; task dedup/creation, authority lookup and dynamic resolver coverage remain incomplete |
| PCTX39 | Multi-session savings harness and shadow adoption | §19,28,38 | evaluation | 17,28–38 | 39,42,48,50,54,62,82 | implementing | `scripts/benchmark.py`, `docs/evaluation.md`, `tests/evaluation/live-mission.json`; `docs/implementation/evidence/evaluation-smoke-debug.json`; real CLI bytes/local broker workloads scaffolded; session smoke unknown, model usage/quality/idle wakes unmeasured |
| PCTX40 | Concurrent capture, output records and child exits | §40,45 | output/capture | 23,34 | 63,66,67,68,69 | implementing | Partial implementation evidence: tests/output.rs; output-extraction-tests.log |
| PCTX41 | Builtin parsers, renderers and safe budgets | §40,45 | output/render | 12,40 | 64,65,70,72 | implementing | Partial implementation evidence: tests/output.rs; tests/filters.rs; integrated-macos-tests.log |
| PCTX42 | TOML filter binding and fixture commands | §41,45 | filters | 03,41 | 71,72 | implementing | Partial implementation evidence: tests/filters.rs; integrated-macos-tests.log |
| PCTX43 | Diagnostic extraction and Boolean/limited structural query | §42,45 | search/extract | 08,10,12,41 | 73,74 | implementing | Partial implementation evidence: tests/search.rs; tests/output.rs; integrated-macos-tests.log; output-extraction-tests.log |
| PCTX44 | Adaptive representations and source invalidation | §43,45 | context | 12,29,43 | 76,77 | implementing | Shared adaptive Build/ContextGet selection and baseline-aware actual packet accounting: src/context.rs, src/session.rs; tests/context_adaptive.rs, tests/context_delivery_selection.rs and tests/session_render_budget.rs. Scope/mandatory preservation, signature-to-body delivery, omitted-source addition, tombstones, idempotent receipts and retained serializer history are covered. Full §43 ranking/error spans, invalidation variants and platform evidence remain unfinished. |
| PCTX45 | Bounded static imports and graph traversal | §15,43,45 | graph | 07,38,44 | 75 | implementing | Partial implementation evidence: tests/graph.rs; integrated-macos-tests.log |
| PCTX46 | Explicit source pack plan/create/inspect/verify | §44,45 | pack | 14,26,44 | 78,79,80 | implementing | Partial implementation evidence: tests/pack.rs; integrated-macos-tests.log |
| PCTX47 | Savings observations and single-compressor integration | §31,36,40,41,45 | evaluation/savings | 31,37,40–42 | 70,81 | implementing | `src/output.rs`, `src/quota.rs`, `src/adapter.rs`, `scripts/benchmark.py`; `tests/output.rs`, `tests/quota.rs`, `tests/adapter.rs`, `docs/implementation/evidence/full-macos-validation.log`; repeated retrieval debug observations retain negative bytes; all delivery/compressor/provider attribution incomplete; current isolated Helper cancellation retains its provider-workspace artifact but caller-workspace metering reports OUTPUT_EXPIRED/unknown (pctx01-exit-matrix-verification.json). Repair metering routing in this inactive producer task; shared delivered exit/error truth remains qualified in PCTX01 |
| PCTX48 | Compression quality/cost ablation and adoption gates | §18,19,38,45 | evaluation | 17,39,41–44,47;45,46 for v0.2 | 82 | implementing | `scripts/benchmark.py`, `tests/evaluation/live-mission.json`, `docs/evaluation.md`; sealed offline mission and emitted-byte smoke only; no paid/model trials, paired quality interval or normative adoption gate evidence |

PCTX13 requires an interface and truthful unsupported behavior; concrete tokenizer engines remain optional (§21). PCTX43 includes required v0.2 limited structural predicates. PCTX45 and PCTX46 are required v0.2 work. PCTX18 includes release preparation, not permission to publish packages or deploy operating projects.

## Acceptance criteria

Every AC is an implementation target. AC01–38 originate in §20; AC39–62 in §38; AC63–82 in §45. The original spec is authoritative for full scenario details. Each test must exercise the real boundary described, not only successful text. Platform and live adapter evidence are separate from fixture evidence.

| Acceptance ID | Contract | Work IDs (PCTX suffixes) | Module/evidence owner | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| AC01 | Non-Git initialization supports index/search/build | 02,07,12 | registry | verified | [cli_contract.rs](../../tests/cli_contract.rs): non_git_exploration_changes_and_idempotent_init plus context_budget_is_exact_json_and_required_rules_are_not_cut; [macOS regression log](evidence/third-full-macos-tests.log), [M/30 baseline](evidence/evaluation-m30-baseline.json) and [third M/30 report](evidence/evaluation-m30-third.json): non-Git strict builds, third warm errors all 0. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. Verified local macOS CLI initialization/index/search/context fixture; other platforms and broader performance remain separate requirements. |
| AC02 | Repeated initialization preserves ID and user config | 02 | registry | implemented | [project.rs](../../src/project.rs); [cli_contract.rs](../../tests/cli_contract.rs); [macOS regression log](evidence/third-full-macos-tests.log): repeated init preserves project/workspace IDs. Remaining exact acceptance proof: edit user configuration, repeat init, and assert unchanged contents/settings (current fixture checks IDs only). |
| AC03 | Create/edit/delete/rename exactly update manifest | 07 | index | implementing | [storage.rs](../../src/storage.rs); [cli_contract.rs](../../tests/cli_contract.rs), [core_security.rs](../../tests/core_security.rs); [macOS regression log](evidence/third-full-macos-tests.log): edited source drops obsolete name, scoped rename/change evidence exists. Remaining: one end-to-end create/edit/delete/rename fixture asserting the exact published current file set. |
| AC04 | Equal size/mtime edits found by strict and verify-content | 07,10,14 | index | implementing | [storage.rs](../../src/storage.rs), [reader.rs](../../src/reader.rs), [search.rs](../../src/search.rs) always reads/hashes current source; [search.rs](../../tests/search.rs), [cli_contract.rs](../../tests/cli_contract.rs); [macOS regression log](evidence/third-full-macos-tests.log): stale modified content rejected. Remaining: preserve both size and mtime deliberately and prove strict plus --verify-content reject old snippets; ordinary edits are insufficient proof. |
| AC05 | Changes during parse/output cause retry or explicit failure | 04,10 | reader | implementing | [reader.rs](../../src/reader.rs), [search.rs](../../src/search.rs), [context.rs](../../src/context.rs); [reader_policy_cache.rs](../../tests/reader_policy_cache.rs), [search.rs](../../tests/search.rs); [macOS regression log](evidence/third-full-macos-tests.log): replaced paths and stale IDs fail. tests/context_delivery_selection.rs now controls source replacement during actual delivery preparation and requires CONCURRENT_MODIFICATION before selection output. Mutation during parsing, complete command coverage and other platforms remain unverified. |
| AC06 | Dirty symbols stay in their own worktree | 02,07 | registry | implementing | [project.rs](../../src/project.rs), [storage.rs](../../src/storage.rs); [core_security.rs](../../tests/core_security.rs), [runner.rs](../../tests/runner.rs); [macOS regression log](evidence/third-full-macos-tests.log): workspace identity refusal and actual linked-provider workspace fixtures pass. Remaining: two real worktrees containing different uncommitted same-path symbols; assert each CLI index/query returns only its own symbols. |
| AC07 | Ten readers/two writers see ready generations only | 06,16 | storage/index | implementing | [concurrency.rs](../../tests/concurrency.rs): ten_readers_and_two_writers_see_published_generations; [macOS regression log](evidence/third-full-macos-tests.log): 10 actual readers/2 writers return valid current data and active generation is ready. Remaining: explicit commit-overlap/writer exclusion observation and structured retry-error assertion; successful serialized updates alone do not cover every clause. |
| AC08 | Crash/disk-full preserve active generation and allow recovery | 06,07,16 | storage/index | implementing | [storage.rs](../../src/storage.rs); [core_security.rs](../../tests/core_security.rs); [macOS regression log](evidence/third-full-macos-tests.log): corrupt index quarantine and atomic rebuilt publication preserve checkpoints. Remaining mandatory proof: force process termination during update and inject disk-full/I/O failure, then verify previous active generation and successful rerun. |
| AC09 | Every query and direct read enforces exclusions | 03,04,05 | policy/config | implementing | [cli_contract.rs](../../tests/cli_contract.rs), [core_security.rs](../../tests/core_security.rs), [search.rs](../../tests/search.rs), [reader_policy_cache.rs](../../tests/reader_policy_cache.rs); [macOS regression log](evidence/third-full-macos-tests.log): excluded direct reads, historical search/checkpoint metadata and exact cached policies are blocked. Remaining: exhaustive find/outline/read/build/document/scope/ignore override matrix, including policy changes at long-lived boundaries. |
| AC10 | Traversal and symlink swaps cannot return outside content | 03,04 | policy/config | implementing | [core_security.rs](../../tests/core_security.rs), [reader_policy_cache.rs](../../tests/reader_policy_cache.rs), [cli_contract.rs](../../tests/cli_contract.rs); [macOS regression log](evidence/third-full-macos-tests.log): traversal, symlink writes/reads and warmed root/ancestor replacement refuse redirection. Remaining: actively raced link swap during source capture/publication and Windows equivalent descriptor/path behavior. |
| AC11 | Ambiguous names and stale symbol IDs never auto-select | 08,09,10 | adapters/languages | verified | [search.rs](../../tests/search.rs): stale_symbol_and_ambiguous_names_never_select_code; [macOS regression log](evidence/third-full-macos-tests.log); [M/30 baseline](evidence/evaluation-m30-baseline.json): baseline stale_symbol_rejected=true; [third M/30 report](evidence/evaluation-m30-third.json): stale symbol rejection again true, matched search errors 0/30. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. Verified local fixture rejects duplicate names and old IDs before returning body; broader grammar/platform coverage remains PCTX08/AC22. |
| AC12 | UTF-8/CRLF/emoji and unusual names retain byte/line positions | 04,05,08 | reader | implementing | [search.rs](../../tests/search.rs), [render.rs](../../tests/render.rs), [broker.rs](../../tests/broker.rs); [macOS regression log](evidence/third-full-macos-tests.log): Korean/CRLF/emoji symbol offsets, space/newline parser paths and Unicode rendering pass. Remaining: actual index/find/read CLI roundtrip for unusual filenames and unsupported filename encoding, not only analyzer inputs; Windows name restrictions separate. |
| AC13 | Final budget preserves complete JSON or fails explicitly | 01,12,13 | cli/application/domain | implemented | [cli_contract.rs](../../tests/cli_contract.rs), [render.rs](../../tests/render.rs); [macOS regression log](evidence/third-full-macos-tests.log): final 2000-byte JSON build and explicit budget failure, one-document/Unicode serializer proof; [M/30 baseline](evidence/evaluation-m30-baseline.json): historical 30 builds within 12000 bytes; [third M/30 report](evidence/evaluation-m30-third.json): 30/30 within the same limit, no warm build errors. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. Remaining: exact-byte tiny/escape-heavy/Markdown final-budget matrix and minimum-failure-envelope invalid-argument contract (current build guard returns BUDGET_TOO_SMALL). No truncation of serialized JSON is permitted. |
| AC14 | Oversize required rules fail without silent truncation | 11,12 | context/documents | verified | [cli_contract.rs](../../tests/cli_contract.rs): context_budget_is_exact_json_and_required_rules_are_not_cut; [project_documents.rs](../../tests/project_documents.rs); [macOS regression log](evidence/third-full-macos-tests.log): oversized required rule returns exit 8/BUDGET_TOO_SMALL without partial rule success. Verified local CLI fixture; expanded document/format matrix remains PCTX11/12. |
| AC15 | Repeated builds preserve body and ordering | 09,12,17 | search | verified | [cli_contract.rs](../../tests/cli_contract.rs): repeated build data equality; [macOS regression log](evidence/third-full-macos-tests.log); [M/30 baseline](evidence/evaluation-m30-baseline.json): historical context_checks.successful_samples=30, deterministic_body=true, all_outputs_within_budget=true; [third M/30 report](evidence/evaluation-m30-third.json) independently records the same values across 30 unchanged-corpus builds. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. Verified for recorded unchanged M corpus/input and local fixture; no model quality, universal ranking or selection-only latency claim. |
| AC16 | New session/checkpoint never implies prior receipt | 14,15,29 | checkpoint | implementing | [context.rs](../../src/context.rs), [session.rs](../../src/session.rs), [storage.rs](../../src/storage.rs); [session.rs](../../tests/session.rs), [cli_contract.rs](../../tests/cli_contract.rs); [macOS regression log](evidence/third-full-macos-tests.log): independent/new session full requirements and epoch/old baseline rejection pass. Remaining exact scenario: new session explicitly reuses an existing checkpoint and still receives current full body without inherited receipt. |
| AC17 | New deny policy immediately blocks historical metadata | 03,07,11 | policy/config | implementing | [cli_contract.rs](../../tests/cli_contract.rs), [core_security.rs](../../tests/core_security.rs), [search.rs](../../tests/search.rs), [reader_policy_cache.rs](../../tests/reader_policy_cache.rs); [macOS regression log](evidence/third-full-macos-tests.log): new policy hides historical search/checkpoint fields, including off mode and cached policies. Remaining: comprehensive historical docs/context/aliases/artifacts matrix and live config change during a long-lived command. |
| AC18 | Unsupported analysis differs from valid zero results | 01,05,08,09 | cli/application/domain | verified | [search.rs](../../tests/search.rs): unsupported_is_distinct_from_empty_supported; [core_security.rs](../../tests/core_security.rs): public parser partial envelope; [macOS regression log](evidence/third-full-macos-tests.log); [M/30 baseline](evidence/evaluation-m30-baseline.json): historical unsupported_not_empty=true; [third M/30 report](evidence/evaluation-m30-third.json): again true, unsupported-language file-query Recall@20/MRR=1 across 10 labels and no-answer empty_correct 10/10 (Recall/MRR null). This does not imply unsupported grammar analysis. Measured binary `af1181c` (`a8648c43…`), before subsequent unverified policy/strict changes; same M corpus manifest, macOS arm64 only, reference hardware/cold cache unknown. Verified distinction in recorded analyzer/CLI fixture boundaries; unsupported languages are not claimed structurally supported. |
| AC19 | Future schema or failed migration leaves original data intact | 06,16,19 | storage/index | implementing | [core_security.rs](../../tests/core_security.rs): rebuild_refuses_newer_schema_without_replacing_original plus foreign identity/corrupt quarantine; [macOS regression log](evidence/third-full-macos-tests.log). Remaining: actual injected failed migration with before/after byte preservation and recovery; future-version refusal is not proof of rollback failure handling. |
| AC20 | Handoff author claims differ from file validation facts | 15 | handoff | implemented | [storage.rs](../../src/storage.rs), [render.rs](../../src/render.rs); [render.rs](../../tests/render.rs), [core_security.rs](../../tests/core_security.rs); [macOS regression log](evidence/third-full-macos-tests.log): user_provided origin rendered separately from checkpoint validation; linked write denied. Remaining: actual handoff create/update/show --validate with altered source and explicit assertions distinguishing author claims from observed file changes. |
| AC21 | Local commands work offline; connector unavailable/cache age explicit | 03,28 | policy/config | implementing | [work.rs](../../src/work.rs), [broker.rs](../../src/broker.rs), [concurrency.rs](../../tests/concurrency.rs), [broker.rs](../../tests/broker.rs), [macOS execution log](evidence/seventh-full-macos-tests.log): local temporary-project flows and explicit unsupported Git/cache observations run without provider access. Remaining: network-disabled mandatory command matrix and connector cached-age/unavailable contract across all sources. No remote connector success or complete offline platform claim. |
| AC22 | Actual macOS/Linux/Windows fixture results agree | 08,17,18 | adapters/languages | implementing | [macOS execution log](evidence/seventh-full-macos-tests.log), [Linux CI](evidence/sixth-linux-ci.log), [Windows CI](evidence/sixth-windows-ci.log) are actual distinct platform evidence. Windows native process admission passes separately; Windows core CLI step records 10 failures/4 passes, so matching three-OS structure/context is not established. Remaining: fix Windows paths/native reader/backend/static failures, run identical supported fixture corpus and compare normalized structure/context, then validate archives on each target. |
| AC23 | Complete local task lifecycle works without GitHub | 20–25 | work/task | implementing | [work_control.rs](../../tests/work_control.rs): evidence_gate_and_historical_completion executes create/assign/start/check/criterion/submit/review/complete on local files/SQLite; [concurrency.rs](../../tests/concurrency.rs) exercises real CLI claims/reports; [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: one complete network-disabled public CLI lifecycle with actual registered verification runner and required policy variants on supported platforms; library flow and external synthetic reports are narrower evidence. |
| AC24 | Concurrent claim permits exactly one active run | 21 | work/agent | implementing | [work.rs](../../src/work.rs): immediate transaction and active-run uniqueness; [concurrency.rs](../../tests/concurrency.rs): two_process_claims_have_one_winner launches two real CLI processes, one succeeds, loser TASK_ALREADY_CLAIMED and board retains winning run; [macOS execution log](evidence/seventh-full-macos-tests.log). This exact macOS fixture is verified. Remaining: same cross-process fixture and active-run preservation on Linux/Windows; no platform-wide verification inferred. |
| AC25 | Local worktrees share board; code/run/clone scope remains isolated | 02,19 | registry | implementing | [project.rs](../../src/project.rs): local canonical common-dir map chooses shared coordination; [work.rs](../../src/work.rs): claims bind current workspace. Existing linked-workspace runner/index fixtures in [macOS execution log](evidence/seventh-full-macos-tests.log) cover subsets only. Remaining exact acceptance: two actual dirty worktrees share authoritative board while symbols/run targets remain isolated, plus separate same-project-ID clone defaults to isolated coordination. |
| AC26 | Heartbeat loss marks stale/expired; never auto-completes or reassigns | 21 | work/agent | implementing | [work.rs](../../src/work.rs), [watch.rs](../../src/watch.rs); [watch.rs](../../tests/watch.rs): heartbeat_and_expiry_appear_as_ephemeral_observations, [macOS execution log](evidence/seventh-full-macos-tests.log): expiry visibility does not mutate done/checklist. Remaining: expired agent cannot report until explicit new epoch, adapter heartbeat loss and reboot/large-clock-change reconfirmation; no automatic completion/reassignment is permitted. |
| AC27 | Old lease epoch writes rejected after reassignment | 21 | work/agent | implementing | [work_control.rs](../../tests/work_control.rs): claims_reports_reassignment_and_cursor verifies old epoch heartbeat LEASE_REVOKED; [work.rs](../../src/work.rs) shares lease validation across mutations; [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: explicitly exercise late report, check begin/record, submit and completion after reassignment, preserving diagnostic history without changing current task/run. Shared helper inspection is not that full regression matrix. |
| AC28 | Changed code/definition invalidates checks and blocks completion | 23,24 | work/check | implementing | [concurrency.rs](../../tests/concurrency.rs): changed_source_and_definition_do_not_reuse_check_evidence and check_changed_during_external_execution_stays_stale; [work_control.rs](../../tests/work_control.rs): current_source_change_blocks_completion; [macOS execution log](evidence/seventh-full-macos-tests.log). Source/definition/policy invalidation is verified locally. Current local entry-tool/environment/profile identity invalidation also passed [eighth native fixtures](evidence/eighth-full-macos-tests.log). Remaining: transitive dependencies/platform identity, changed criteria after review and dependency/subtask result mutation matrix. |
| AC29 | Bare agent success remains unverified | 23 | work/check | implementing | [work.rs](../../src/work.rs): manual_claim and disallowed provenance yield unverified; [runner.rs](../../tests/runner.rs): unknown_and_zero_tests_do_not_pass, [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining exact fixture: valid-looking positive-count passed report from manual_claim cannot satisfy a required check or criterion/complete gate. Schema validation and parser summaries never prove actual execution. |
| AC30 | Zero-test/failure/cancel/incomplete report cannot pass gates | 23 | work/check | implementing | [work_control.rs](../../tests/work_control.rs): zero_tests_and_revision_conflict and zero_minimum_cannot_make_zero_tests_pass; [runner.rs](../../tests/runner.rs), [parsers.rs](../../tests/parsers.rs) test observed typed counts/capture boundaries; [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: full completion-gate fixtures for failures/errors/nonzero exit/all-skipped/cancel/timeout/incomplete capture/latest flaky retry; parser-only rejection is not completion transaction proof. |
| AC31 | Wrong revision review cannot approve current submission | 24 | work/gates | implementing | [work.rs](../../src/work.rs): review records current immutable submission ID and checks target/definition/evidence before approval; [work_control.rs](../../tests/work_control.rs) verifies owner review is required, [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining exact fixture: approval of an old submission followed by new code/criteria/submission cannot authorize current completion, including rejection/rework and reviewer independence. |
| AC32 | Replay is idempotent; old reports never overwrite latest stage | 22 | work/events | implementing | [work_control.rs](../../tests/work_control.rs): matching report replay returns identical receipt and old report_seq rejects OUT_OF_ORDER_REPORT; database_rejects_event_modification_and_receipt_conflict; [concurrency.rs](../../tests/concurrency.rs): public CLI replay; [macOS execution log](evidence/seventh-full-macos-tests.log). Task-create same-key/different-payload conflict is also tested. Remaining: report-key payload conflicts and retry after successful response loss/reassignment, reordered distinct reports and idempotency coverage for every state-changing command. |
| AC33 | Dependency cycles rejected; scope edits invalidate evidence | 20 | work/task | implementing | [work.rs](../../src/work.rs): dependency reachability rejects cycles, scope/definition edits advance revision and clear evidence; [concurrency.rs](../../tests/concurrency.rs) covers changed source/definition, [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining exact regression: multi-task cycle insertion and scope-only edit invalidating accepted criteria/review/check evidence; task depend add/remove CLI and parent/subtask aggregation remain absent. |
| AC34 | Index GC/rebuild preserves all control history | 16,19,26 | maintenance | implementing | [storage.rs](../../src/storage.rs), [project.rs](../../src/project.rs), [work.rs](../../src/work.rs) separate index and control DB; [core_security.rs](../../tests/core_security.rs): checkpoint-preserving rebuild and corrupt-index quarantine, [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining exact acceptance: populate tasks/runs/checks/reviews/events/receipts, perform index gc --apply/rebuild, compare authoritative histories byte/logically unchanged. Checkpoint preservation alone is insufficient. |
| AC35 | Concurrent completion changes fail CAS transaction | 20,22,24 | work/task | implementing | [work.rs](../../src/work.rs): immediate transaction plus expected task revision, submission/evidence/current fingerprints; [work_control.rs](../../tests/work_control.rs): zero_tests_and_revision_conflict; [concurrency.rs](../../tests/concurrency.rs): report_replay_and_revision_conflict_are_visible_at_cli; [macOS execution log](evidence/seventh-full-macos-tests.log). Remaining: synchronized second process changes task/evidence immediately before completion and proves CAS conflict without done event. Sequential stale-revision tests do not prove that race. |
| AC36 | Restore preserves history; interrupts runs and revokes capabilities | 26 | backup | implementing | [work_control.rs](../../tests/work_control.rs): backup_restore_revokes_live_leases_and_preserves_pause, restore_preserves_session_history_without_ack_or_epoch_reuse, schedule_v2_backup_strips_local_binding_and_restore_revokes_running_authority; [quota.rs](../../tests/quota.rs), [macOS execution log](evidence/seventh-full-macos-tests.log). Local restoration/history/authority subsets verified. Remaining: full token/approval reuse denial matrix, archive-secret/path audit, explicit workspace attach and native new-host bridge/resource reconciliation. |
| AC37 | Pause/block/checklist percentage are distinct from done | 20,24,25 | work/task | implementing | [work_control.rs](../../tests/work_control.rs): 100% agent estimate stays in_progress and restore preserves pause; [watch.rs](../../tests/watch.rs) separates checklist/time observations; [macOS execution log](evidence/seventh-full-macos-tests.log). [work.rs](../../src/work.rs) keeps task done and historical current validity separate. Remaining exact scenarios: blocked/paused tasks with 100% accepted checklist remain unfinished, no-criteria progress unknown and parent aggregation excludes cancelled leaves. |
| AC38 | Follow reconnect resumes cursor; heartbeat does not change done ratio | 22,25 | work/events | implementing | [watch.rs](../../tests/watch.rs): finite_ndjson_drains_all_pages_and_resumes_after_cursor and heartbeat_and_expiry_appear_as_ephemeral_observations; [cli_contract.rs](../../tests/cli_contract.rs): ndjson_snapshot_follow_and_resume_have_no_event_gap; [macOS execution log](evidence/seventh-full-macos-tests.log). Cursor/heartbeat subset verified. Remaining: forced disconnect while concurrent writers emit events, recovery without duplication/gaps under §27 load and measured redraw ≤3s; ephemeral notices never become durable completion events. |
| AC39 | Five identical session queries have one upstream refresh | 28,39 | broker | implementing | `src/broker.rs`, `tests/broker.rs`, `scripts/benchmark.py`, `evaluation-smoke-debug.json`: local Git shared refresh observation; five-session authenticated remote source contract absent |
| AC40 | Shared parsing never bypasses worktree manifest visibility | 07,28 | index | not_started | planned: `tests/acceptance/ac40.rs`, `docs/evidence/ac40.md` |
| AC41 | Auth/errors/partial pagination differ from empty; cleanup blocked | 28,35 | broker | not_started | planned: `tests/acceptance/ac41.rs`, `docs/evidence/ac41.md` |
| AC42 | Acknowledged unchanged delta returns baseline without repeated body | 29,39 | session/context | not_started | planned: `tests/acceptance/ac42.rs`, `docs/evidence/ac42.md` |
| AC43 | New session/epoch/policy scope rejects old baseline with full required | 29 | session/context | not_started | planned: `tests/acceptance/ac43.rs`, `docs/evidence/ac43.md` |
| AC44 | Message replay has no duplicate effect; failed delivery stays pending | 22,30 | work/events | not_started | planned: `tests/acceptance/ac44.rs`, `docs/evidence/ac44.md` |
| AC45 | Identical approval action merged; environment/cost/artifact separated | 33 | operations/policy | not_started | planned: `tests/acceptance/ac45.rs`, `docs/evidence/ac45.md` |
| AC46 | Agent approval claim cannot mint owner grant or bypass host permission | 03,33 | policy/config | not_started | planned: `tests/acceptance/ac46.rs`, `docs/evidence/ac46.md` |
| AC47 | Account pool totals deduplicate cumulative usage; missing is unknown | 31 | budget | implementing | `src/quota.rs`, `tests/quota.rs`, `restore-quota-tests.log`: cumulative deltas, source/pool separation and missing-as-unknown; live collector/billing unknown |
| AC48 | Quota/crash recovery uses capsule/events without new model summary | 31,32,39 | budget | implementing | `src/quota.rs`, `src/work.rs`, `tests/quota.rs`, `restore-quota-tests.log`: restore needs fresh observation without summary model; full capsules/replay/recovery workload missing |
| AC49 | Restore cannot reuse run/lease/approval already superseded | 21,32 | work/agent | implementing | `tests/work_control.rs`, `restore-quota-tests.log`: restored run/session epoch nonreuse; full run/lease/grant supersession matrix partial |
| AC50 | Across projects and legacy path at most one heavy child runs | 34,39 | operations/runner | implementing | `src/runner.rs`, `tests/runner.rs`, `docs/implementation/evidence/full-macos-validation.log`: cross-project heavy slot and canonical legacy mutex pass locally; native platforms unknown |
| AC51 | Parent death/live child prevents lock theft by TTL | 34 | operations/runner | implementing | `src/runner.rs`, `tests/runner.rs`, `docs/implementation/evidence/full-macos-validation.log`: parent-death/live child cases pass, guardian-negative observation passes after durable attachment acknowledgement; no TTL takeover |
| AC52 | Subagent/cloud share one auxiliary slot and denied capabilities | 34 | operations/runner | implementing | `src/runner.rs`, `tests/runner.rs`, `docs/implementation/evidence/full-macos-validation.log`: local registered helper capacity and cloud intent tested; actual cloud/native provider pre-spawn enforcement unknown |
| AC53 | DST/restarts preserve once-per-local-date and coalesce missed ticks | 36 | operations/schedule | implementing | `src/schedule.rs`, `tests/schedule.rs`: Berlin overlap/gap, interval/restart/coalescing scenarios present; 8 local tests pass in `full-macos-validation.log`; bridge actual execution unknown |
| AC54 | Pause/silence survive quota reset, restore and schedules | 30,33,36 | session/mailbox | implementing | `src/operations.rs`, `src/quota.rs`, `src/schedule.rs`; `tests/quota.rs`, `restore-quota-tests.log`: reset/restore pause/silence persistence tested; schedule fixtures pass in `full-macos-validation.log`; external bridge not implemented |
| AC55 | Wrong-SHA check/CODEOWNERS/staging evidence fails merge/release | 23,24,35 | work/check | not_started | planned: `tests/acceptance/ac55.rs`, `docs/evidence/ac55.md` |
| AC56 | Server merged but not deployed leaves app release blocked | 35 | adapters/git/workflow | not_started | planned: `tests/acceptance/ac56.rs`, `docs/evidence/ac56.md` |
| AC57 | Concurrent PR intents and remote open count respect WIP cap | 35 | adapters/git/workflow | not_started | planned: `tests/acceptance/ac57.rs`, `docs/evidence/ac57.md` |
| AC58 | Only verified native instruction duplicates omit body | 29,37 | session/context | implementing | `src/adapter.rs`, `tests/adapter.rs`, `docs/implementation/evidence/full-macos-validation.log`: events do not imply ack; verified native duplicate instruction omission not implemented |
| AC59 | Claude schema failures degrade capability; protected gates fail closed | 37 | adapters/claude | implementing | `src/adapter.rs`, `tests/adapter.rs`, `docs/implementation/evidence/full-macos-validation.log`: malformed/schema variations and permission observation fixtures pass; installed-version/live protected hook compatibility unknown |
| AC60 | Repeated drift/batch creates no duplicate DOC proposal or semantic edit | 38 | adapters/inventory | not_started | planned: `tests/acceptance/ac60.rs`, `docs/evidence/ac60.md` |
| AC61 | New-host restore omits credentials, revokes leases and reconciles safely | 18,26,32,36 | release/docs | implementing | `src/work.rs`, `src/quota.rs`, `src/schedule.rs`; `tests/work_control.rs`, `restore-quota-tests.log`: local restore revokes epochs and quota observations; actual new-host schedule/credential exclusion matrix unknown |
| AC62 | Other language/VCS/runtime projects work without beSir constants | 18,37,38,39 | release/docs | not_started | planned: `tests/acceptance/ac62.rs`, `docs/evidence/ac62.md` |
| AC63 | Child exit 0/1/2/signal and pre-spawn refusal stay distinguishable | 01,34,40 | cli/application/domain | implementing | `src/output.rs`, `src/runner.rs`, `tests/output.rs`, `tests/runner.rs`, `docs/implementation/evidence/full-macos-validation.log`: child status/refusal/timeout local tests; runner 19 local cases pass; full signal/native matrix partial |
| AC64 | Failure/skip/flaky/crash evidence survives compression | 23,41 | work/check | not_started | planned: `tests/acceptance/ac64.rs`, `docs/evidence/ac64.md` |
| AC65 | Unsupported/timed-out/malformed parser returns partial excerpt, no rerun | 41 | output/render | not_started | planned: `tests/acceptance/ac65.rs`, `docs/evidence/ac65.md` |
| AC66 | Concurrent large pipes drain within memory/disk/record limits | 40 | output/capture | not_started | planned: `tests/acceptance/ac66.rs`, `docs/evidence/ac66.md` |
| AC67 | Split secrets/ANSI/CR/binary never write unmasked bytes | 40 | output/capture | not_started | planned: `tests/acceptance/ac67.rs`, `docs/evidence/ac67.md` |
| AC68 | Full output lookup uses same records; expiry/policy cannot recreate loss | 40 | output/capture | not_started | planned: `tests/acceptance/ac68.rs`, `docs/evidence/ac68.md` |
| AC69 | Literal argv and dangerous inner commands preserve permission boundary | 03,34,40 | policy/config | implementing | `src/runner.rs`, `src/output.rs`, `tests/runner.rs`, `tests/output.rs`, `docs/implementation/evidence/full-macos-validation.log`: exact trust/script changes/agent forgery and manual unsafe command refusal; native hooks/live host enforcement unknown |
| AC70 | Separate run intents execute separately; at most one compressor | 37,41,47 | adapters/claude | implementing | `src/output.rs`, `src/adapter.rs`, `tests/output.rs`, `tests/adapter.rs`, `docs/implementation/evidence/full-macos-validation.log`: no command rewrite/execution from PreTool event; live one-compressor frontend delivery coverage partial |
| AC71 | Filter hash/match/op changes fail trust/schema/ambiguity checks | 42 | filters | not_started | planned: `tests/acceptance/ac71.rs`, `docs/evidence/ac71.md` |
| AC72 | Protect beats drop; failure counts/refs/determinism preserve no false success | 41,42 | output/render | not_started | planned: `tests/acceptance/ac72.rs`, `docs/evidence/ac72.md` |
| AC73 | Diagnostic overlaps deduplicate; stale lines never map to changed source | 43 | search/extract | not_started | planned: `tests/acceptance/ac73.rs`, `docs/evidence/ac73.md` |
| AC74 | Boolean precedence/negative-only/complexity rules preserve literal find | 43 | search/extract | not_started | planned: `tests/acceptance/ac74.rs`, `docs/evidence/ac74.md` |
| AC75 | Import cycles/aliases/dynamic imports respect limits and isolation | 38,45 | adapters/inventory | not_started | planned: `tests/acceptance/ac75.rs`, `docs/evidence/ac75.md` |
| AC76 | Adaptive shrink preserves required rules and unacknowledged full data | 12,29,44 | context | implementing | `tests/context_adaptive.rs`, `tests/session_render_budget.rs`: native targeted fixtures preserve mandatory task/rules/decisions through optional downgrade and require explicit receipt acknowledgement. Build/ContextGet now share adaptive selection and actual full/delta measurement. Native CLI fixtures verify unacknowledged full data is delivered on signature upgrade and omitted sources are later added. Complete new-session representation/ranking and platform coverage remain required. |
| AC77 | Decision/memory changes invalidate context even when Git is unchanged | 11,44 | context/documents | implementing | `src/documents.rs`, `src/context.rs`, `tests/project_documents.rs`, `tests/session.rs`: source hashes/conflicts/supersession integrate with packets; decision/legacy-instruction modification, retirement, deletion and original-hash-preserving supersession now have actual CLI proof with fixed Git HEAD/porcelain; docs snapshot revalidation catches omitted dependency races. Remaining: per-item topic attribution and owner exception conflicts, handoff/task/approval variants, other-platform lineage proof and complete recovery/policy-race variants. |
| AC78 | Pack source/policy/output changes reject stale plan and overwrite/leak | 46 | pack | implementing | `tests/pack.rs`, `tests/session_render_budget.rs`, `src/pack.rs::validation_tests`, `pack-consumer-verification.json`, `pack-validation-race-verification.json`: native stale/source/output boundaries and committed consumer pause during source validation; Windows publication and native check-to-publication interval remain unqualified. |
| AC79 | Pack scope/representation/secrets/licenses/item limits preserved | 46 | pack | implementing | `tests/pack.rs`, `tests/session_render_budget.rs`, `pack-consumer-verification.json`: explicit scopes, four representations, configured masking/license and actual serialized byte budgets; complete content-classification, platform and adversarial matrix remains required. |
| AC80 | Missing/tampered parts and absent sources distinguish integrity/freshness | 46 | pack | implementing | `tests/pack.rs`, `pack-consumer-verification.json`: native checksums/parts/freshness and retained public schema1 read compatibility; Windows reader/publication and complete partial-receipt/recovery matrix remain required. |
| AC81 | Rerender/retrieval/compressor metrics avoid double count and quota confusion | 31,47 | budget | implementing | `src/quota.rs`, `src/output.rs`, `src/adapter.rs`, `scripts/benchmark.py`; `tests/quota.rs`, `tests/adapter.rs`, `evaluation-smoke-debug.json`: actual/estimate/context separation and retrieval negatives; complete delivery accounting/provider receipt unknown |
| AC82 | Compression preserves diagnostic/task success including total followup costs | 17,39,48 | evaluation | implementing | `scripts/benchmark.py`, `tests/evaluation/*.json`, `docs/evaluation.md`, `evaluation-smoke-debug.json`: debug output has diagnostic miss, unknown session and negative savings; actual task success/cost/noninferiority unmeasured |

## Additional mandatory prose contracts

These followup IDs close requirements not adequately represented by the 48-item/82-AC lists. They must be implemented or explicitly resolved before the whole goal is complete. Checks AX below are additional acceptance criteria. Current partial implementations and remaining contracts are recorded individually; planned paths are not execution evidence.

| Work / check | Binding contract and minimum acceptance | Spec | Ownership / dependency | Status / evidence |
| --- | --- | --- | --- | --- |
| PCTX49 / AX01 | Multiple roots: explicit root IDs in file keys; reject ambiguity/collisions and cross-root traversal; discover and query each permitted root without visibility mixing. | §3,6,16 | registry/reader;02,03,07 | not_started; planned: `tests/pctx49.rs`, `docs/evidence/pctx49.md` |
| PCTX50 / AX02 | Metadata export/import: explicit selection plan; no source/DB/log/absolute paths/credentials by default; reject traversal, links, duplicate or colliding names, oversized archives; staging/checksum and no implicit overwrite. | §16,17 | backup/pack;14,26 | not_started; planned: `tests/pctx50.rs`, `docs/evidence/pctx50.md` |
| PCTX51 / AX03 | Read-only Tool API or MCP/serve: versioned envelopes and bounded local read methods invoke Application services; require explicit activation, no independent policy or gate implementation. | §3,5,14 | application/adapters;01,12,25 | not_started; planned: `tests/pctx51.rs`, `docs/evidence/pctx51.md` |
| PCTX52 / AX04 | GitHub write sync/PR action wrapper: immutable object identity, field conflicts, managed scope, approved intent/outbox receipt; lost response becomes unknown until reconciliation, never duplicate posting; external closed does not bypass local gate. | §3,27,34 | workflow/adapters;24,28,33,35 | not_started; planned: `tests/pctx52.rs`, `docs/evidence/pctx52.md` |
| PCTX53 / AX05 | Relations/refs/trace/impact: static import and explicit document relations; direction/depth/node limits and unresolved coverage; potential impact never exact runtime references or proof tests are unnecessary. | §3,15,43 | graph;38,45 | not_started; planned: `tests/pctx53.rs`, `docs/evidence/pctx53.md` |
| PCTX54 / AX06 | Release/update/remove/migration/recovery docs and installable archives: checksums, licenses, SBOM and bundled SQLite version inspection; per-platform actual runtime evidence; preserve user configs on update/remove. | §21,22;user goal | release/docs;18 | not_started; planned: `tests/pctx54.rs`, `docs/evidence/pctx54.md` |
| PCTX55 / AX07 | Fail safely on filesystem/DB/process boundaries: injected disk-full, corruption/migration failure, process crash/live child, signal/timeout and real concurrent readers/writers; complete structured error and no false current/pass. | §9,10,17,27,33,40;user goal | evaluation;06,16,27,34,40 | implementing; `src/runner.rs`, `src/output.rs`; `tests/runner.rs`, `tests/output.rs`, `docs/implementation/evidence/full-macos-validation.log`; real parent death, timeout and slot tests; guardian race correction passes; injected disk-full/DB corruption/native matrix incomplete |
| PCTX56 / AX08 | Reproducible performance corpus: S/M/L sizes and M mix, platform/environment/parser/config recorded; >=30 warm samples p50/p95/max, controlled cold separate; retain unmet targets as unmet. | §18,27,36,45 | evaluation;17,27,39,48 | implementing; `scripts/generate-corpus.py`, `scripts/benchmark.py`, `tests/evaluation/default.json`, `docs/evaluation.md`; `evaluation-smoke-debug.json` is 100 files/one repetition, not M/30 evidence; controlled cold/reference environment unknown |
| PCTX57 / AX09 | Real savings quality harness: >=60 labeled queries; >=30 tasks, >=3 repeats, task-level paired intervals and -5pp noninferiority; actual provider usage distinct from bytes/tokenizer/API/quota; include coordination/retrieval/recovery and fair warm/cold baselines. | §19,38,45 | evaluation;17,39,47,48 | implementing; `scripts/benchmark.py`, `tests/evaluation/live-mission.json`, `docs/evaluation.md`; 60 seeded labels and sealed 30-task/3-repeat protocol; debug retrieval negatives recorded; actual provider/quality paired trials unmeasured |
| PCTX58 / AX10 | Artifact lifetime inventory: index metadata only; recent two ready generations, checkpoint 30d/100 with pins, diagnostics 7d/10MiB; control decisions/capsules/history survive GC; temporary masked outputs 24h and pack opt-in remain distinct. | §9,17,35,40,44,45 | storage/maintenance;16,26,32,40,46 | not_started; planned: `tests/pctx58.rs`, `docs/evidence/pctx58.md` |
| PCTX59 / AX11 | Versioned contracts: config/JSON/DB/parser versions distinct; unknown optional JSON fields tolerated, mandatory meaning changes major; unknown config/filter required keys fail; adapters check installed capability and supported schema. | §8,22,36,41 | domain/application;01,03,06,37,42 | implementing; `src/documents.rs`, `src/adapter.rs`, `src/quota.rs`; strict versioned schemas and compatibility fixtures in `tests/project_documents.rs`, `tests/adapter.rs`, `docs/implementation/evidence/full-macos-validation.log`; installed capability negotiation remains unknown |
| PCTX60 / AX12 | Output surfaces: compact default, one full JSON per nonstream call, NDJSON only explicit stream, stderr diagnostics; original child status exceptional; atomic --output no overwrite; unsupported options fail. | §14,27,38,40,45 | cli/application;01,12,25,40 | implementing; `src/main.rs`, `src/watch.rs`, `src/output.rs`; `tests/cli_contract.rs`, `tests/watch.rs`, `watch-tests.log`; complete JSON/explicit NDJSON/controlled closed pipe; entire command output/atomic-output matrix partial |
| PCTX61 / AX13 | Verified profile loading and drift: Core has no beSir roles/paths/account/brand/model constants; static inventory never executes configuration; source authority/trust/control states separated; role/check/design/asset references retain source and license. | §28,32,34,37 | config/inventory;03,33,38 | implementing; `src/documents.rs`, `src/graph.rs`; `tests/project_documents.rs`, `tests/graph.rs`, `docs/implementation/evidence/full-macos-validation.log`; inert scoped source provenance only; profile inventory/drift/role/check/design/asset license completeness absent |
| PCTX62 / AX14 | Instruction/document handling: bounded inert YAML front matter, explicit conflicts and supersession, external explicit task/handoff documents allowed only under separate size/secret validation; prompt text cannot become action approval. | §12,13,17,25,36 | documents/policy;04,11,15,37 | implementing; `src/documents.rs`, `tests/project_documents.rs`, `docs/implementation/evidence/full-macos-validation.log`; bounded inert YAML/tags/alias/duplicate ID rejection, conflicts/supersession and masking; explicit external task/handoff document reader contract incomplete |
| PCTX63 / AX15 | Schedule/managed install safety: read-only inspect/plan precede hash-bound explicit apply; preserve existing hooks/scheduler entries, only owned uninstall; no LLM on idle tick or implicit global installation; keep-awake owner/TTL cleanup. | §35,36,37 | schedule/adapters;36,37 | implementing; `src/adapter.rs`, `src/schedule.rs`; `tests/adapter.rs` (10 pass in `docs/implementation/evidence/full-macos-validation.log`), `tests/schedule.rs` (8 pass in `full-macos-validation.log`); hash-bound local owned hook install/uninstall; OS scheduler bridge and keep-awake not implemented |
| PCTX64 / AX16 | Required builtin output parsers: Git status/log/diff, TypeScript diagnostics, ESLint reports and registered Jest/Vitest reporters; pnpm script fingerprints; supported version fixtures and incomplete/fallback paths, never presentation-as-check evidence. | §37,40,41 | output/adapters;38,40,41 | not_started; planned: `tests/pctx64.rs`, `docs/evidence/pctx64.md` |
| PCTX65 / AX17 | Resource/approval authority: deterministic multi-resource acquire/all-or-release, host process identity and boot evidence; owner grants bind artifact/argv/environment/cost/expiry, trusted provenance; incidents cannot turn into host permission. | §32,33,34 | operations;33,34,35 | implementing; `src/runner.rs`, `src/operations.rs`; `tests/runner.rs`, `tests/operations.rs`, `docs/implementation/evidence/full-macos-validation.log`; host identity/registered guardian/grants locally covered; runner local regressions pass; native/cloud host permission and full multi-resource matrix unknown |
| PCTX66 / AX18 | Command contract completeness: every command in §§14,27,38,40–44 implemented or explicit optional/exclusion record; help read/write/execute capability metadata and cross-frontend conformance test; planned commands never return success. | §14,27,38,40–45 | application/cli;all required work | implementing; `src/main.rs`, `tests/cli_contract.rs`, `watch-tests.log`; integrated module routes exist; canonical check-plan/run/resource/job aliases, full help capability and cross-frontend conformance incomplete |

## Binding shared invariants and numeric defaults

* Domain keeps evidence (`observed/inferred/unknown`), freshness (`current/stale/unchecked/missing`) and coverage (`complete/partial/unsupported`) independent. Locations use SHA-256, exact UTF-8 bytes `[start,end)` and inclusive one-based line requests; no filesystem-atomic claim (§8).
* Root priority: explicit root, nearest config, nearest Git worktree, current directory. Nested repositories/submodules excluded by default. UUID project lineage is not authentication; canonical workspace and coordination identities isolate clones and dirty worktrees (§6,16,27).
* CLI > environment > project > user > defaults for ordinary settings. Denies/security exclusions combine by union; configured network intersects user destination/action trust. Ignore relaxation cannot remove security exclusions. No repository-supplied automatic executable configuration (§7,17).
* Safe verified bytes are the single reader input to parsing; at most two changing-file retries. Default 1MiB input, UTF-8-only text/name support, no link following or case/Unicode rewriting. Current output policy reapplies to historical records (§6,10,17).
* Index uses local WAL, foreign keys, one writer and 5-second wait bound. SQLite >=3.51.3 or independently verified WAL fix is mandatory. Parsing outside transaction; ready publication/active pointer inside one transaction; pinned reader generation; failures preserve active database (§9,10).
* Search: smart case, literal default, regex explicit, <=16 alias expansions, 20 default results/1000 maximum, 200 evaluated candidates. Score maxima: seed 100, exact name 80, path segment 60, name token 50, heading 40, literal 25, aliases x0.8; change/role bonuses <=10 each; raw path/start-byte ties (§11).
* Build/checkpoint/full default strict; query default matched; off is unchecked. Mandatory scoped rules never truncate. Budget includes serialized envelope/keys/separators and stabilization within three rounds; unsupported tokenizer cannot enforce exact tokens. Default build 12000 bytes, brief 6000 bytes, read 80 lines/64KiB; query 10s versus strict/index/checkpoint 120s (§10–14,30).
* Task workflow and run activity are independent. Single active assignment/run enforced atomically. Trusted run capability is never stdout/backup/context. DB-received timestamps control 90s stale/180s expiry; report sequence/order/idempotency cannot regress state. Progress reports <=4096 bytes and next action <=2048 bytes (§27).
* Complete rechecks submission revision/target, predecessor/child state, acceptance, last valid check attempts, matching review/evidence set, blockers and leases in CAS transaction. Zero unit tests/all skipped/missing exit code never pass; default unit minimum 1; human or independent review cannot be self-approved (§27).
* Snapshot key includes project/workspace/permission scope, content, config/parser/policy and source revision. One process refreshes; no network call while holding transaction; late response cannot replace newer snapshot. Suggested display TTLs Git 2s, issue/PR 30s, remote lists 120s; gates refresh. Missing/partial/error/unauthorized are not empty (§29).
* Context full/delta uses one explicit per-session/epoch receipt ledger. Emit/deliver/ack/understand differ; compact/new sessions cannot inherit ack. Removed files and cancelled decisions/expired grants produce tombstones. Mailbox defaults <=5 lines/2048 bytes; at-least-once delivery, precise effect dedup, no heartbeat wake/broadcast (§30).
* Actual usage/cumulative counters, provider cache tokens, tokenizer counts, dollar estimates, subscription quota and context window are separate. Unknown is not zero/sufficient; local reserve is soft. Quota 80% conserve/90% drain/provider limit blocked; reset requires reobservation. Capsule boundary persistence/debounce 30s and event replay require no new summary model (§31).
* Effective action policy intersects explicit deny/pause, scoped current grant, standing delegation and host permission. Silence applies before ranking/delivery and survives restore/reset/reconcile. Owner-origin requires trusted operator/transport evidence. No role document grants permissions (§32).
* Task lease does not own child resource lifetime. Heavy and auxiliary defaults each capacity 1 at host scope; legacy path uses the same validated provider. Unknown child/process identity blocks admission; cancellation confirms child exit before release. Unknown native pre-spawn capability is advisory (§33).
* PR identity/head SHA, current CI/CODEOWNERS/exception/manifest and remote-open plus pending reservation govern gates/WIP. Issue closed, auto-merge requested, merged and deployed are distinct; Terraform grants bind exact plan/environment; destructive cleanup produces reviewable plan (§34).
* IANA timezone, revision+logical-occurrence uniqueness, once-per-local-date and next-valid DST policy; interval versus wall-clock schedule explicit; missed runs coalesce without false success; pause/silence outrank install/reconcile (§35).
* Claude capability is installed-version evidence, fixture compatibility and live connection separately. Allowlist hook metadata, opt-in managed hash plans, fail-open telemetry and fail-closed protected gates; normal hook warm p95 <=100ms. No private conversation DB or credential UI scraping (§36).
* Runner spawns exact argv once and never uses query dedup, implicit shell or compressor flag changes. Child exit and wrapper error are separate; noninteractive capture/stdin closed by default; execution timeout independent of query timeout; no automatic rerun on pipe/parser/storage failure (§40).
* Output policy: mask before any disk write, independently drain both streams, preserve CR errors and provenance, unsupported binary record omission explicit. Defaults: 16MiB/execution, 256MiB/project, 1GiB/host, TTL 24h, record 256KiB, parser RAM 32MiB, filter CPU 1s, compact 8KiB. Retain none disables raw retrieval; expiry cannot erase permanent check evidence (§40).
* Filters inert TOML; exact executable/argv-prefix trust; hash changes require revalidation; equal-priority match fails. Protect/typed errors outrank drop; no arbitrary code/network/filesystem/plugin operations. One compressor; already-compacted input cannot invent original size (§41).
* Boolean only with --query: quoted phrase/token/parens/uppercase NOT > AND > OR, <=64 AST nodes/depth 8, no negative-only global query, positive aliases only. Extract shares verified reader/AST, overlap merge and stale diagnostic refusal (§42).
* Adaptive source fingerprint includes task/rules/decisions/memory/grants as well as Git. Required rules precede code; full span → signature → outline → reference downgrade explicit. Import depth default 0/max 2, max 200 nodes; graph trace default depth 2/max 1000 nodes; unresolved dynamics never fake exact edges (§15,43).
* Pack default metadata/64KiB, explicit source mode and scope, immutable hash plan revalidated at create, input/output disjoint, staging publish, true serialized total including part headers/manifest, item-boundary split without total-budget increase. Integrity and current-source freshness independent; partial part receipt is not full ack (§44).
* Savings denominator is same execution masked uncompressed output; compact plus all retrieval/delivery costs counted, redaction savings excluded, negative savings retained. Rerender not delivered is compute cost only. Target verbose fixture bytes >=50% lower, false success/required-diagnostic loss zero; actual task quality/usage criteria remain separate (§45).

## Optional extensions and explicit exclusions

| Classification | Item | Basis / rationale | Status and evidence |
| --- | --- | --- | --- |
| optional | Concrete tokenizer engines; additional grammar types/language adapters and resolvers; provider expansion/CI extensions | §3,12,15,21,43: capability-dependent and no bounded required provider list beyond initial support | not_started; no implementation evidence |
| optional | Embeddings, model reranking, LSP, exact runtime calls/references, advanced test optimization | §3,11,15,23,42: conditional review/experiments, beyond bounded static relations | not_started; no implementation evidence |
| optional | Shared immutable Base/Overlay, daemon, remote real-time state/auth transport and multiple-host operation | §3,16,23,33: introduced only after evidence or subsequent concrete contract | not_started; no implementation evidence |
| optional | Third-party compressor adapters, PTY, automatic shell wrappers, package-manager channels, file watcher | §3,22,39–41: baseline local/manual paths are required; extensions gated by fixture/authority proof | not_started; no implementation evidence |
| optional | External message/schedule bridges beyond initial supported managed bridge and manual transport | §3,30,35,36: capability dependent; local durable registry/queue required | not_started; no implementation evidence |
| excluded | Autonomous code editing/commit/deployment/model launch; transcript or credential replication; automatic source backup | §1,3,17,31: explicit exclusion; user goal commit/push authorizes development repository work only | not_started; exclusion record here |
| excluded | Arbitrary language type/runtime analysis; network-filesystem shared SQLite; security claims over non-PCTX file access | §3,5,9,17: explicitly unsupported security/storage model | not_started; exclusion record here |
| excluded | YAML CLI output; initial pack XML/archive/remote clone/clipboard/upload; project auto-install URLs/executable templates | §7,14,44: explicit formats/actions outside required baseline | not_started; exclusion record here |

## Section coverage audit

All sections read in full, including tables and command examples. §1–4 → goals/scope/flows; §5–8 → architecture/identity/policy/envelope; §9–14 → index/search/context/handoff/CLI; §15–17 → graph/portability/security; §18–22 → evaluation/AC/work/release; §23–26 → decisions/interfaces/documents/source limits; §27–28 → coordination/product intent; §29–38 → broker/session/budget/recovery/operation/adapter/profile/extensions; §39–45 → output/filter/extract/adaptive/pack/metrics. Reference links in §26 are design sources, not proof of installed capabilities or achieved results.

Structural coverage: 48 baseline work rows, 82 AC rows, 18 extra prose work/check rows, plus PCTX67/AX19. This checks traceability cardinality, not full contract completion. Partial evidence never upgrades an entire AC to `verified`.

## Additional public-documentation requirement

| Work | Requirement | Source | Module | Acceptance | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| PCTX67 | Faithful English public edition of the supplied Korean specification; preserve the local original | Owner goal §2; D018 | docs/specification | AX19: all 45 sections and contracts translated without changing scope | implemented | docs/PCTX-implementation-spec-v0.6-en.md; docs/translation-coverage.md; independent linguistic review not performed |

## First macOS integration evidence

`docs/implementation/evidence/integrated-macos-tests.log` records the integrated unit and CLI/DB/process suites for broker, CLI, concurrency, core security, filters, graph, operations, output, pack, project documents, quota, search, sessions and work control. It predates later runner/scheduler integration and diagnostic extraction additions. `output-extraction-tests.log` verifies eight output scenarios after those additions. This is partial evidence for the mapped requirements; complete AC rows remain unverified where mandatory subcontracts or platform/provider evidence are still absent. `first-integration-tests.log` and earlier logs retain failure history and must not be cited as an all-pass release gate.

## Current reconciliation checkpoint (2026-10-08)

Evidence names abbreviated in rows resolve under `docs/implementation/evidence/`. `full-macos-validation.log` records format checking, all-target Clippy with warnings denied, and 148 passing tests across 21 test result groups (including zero-test binary/doc groups). Adapter 10, documents 5, quota 7, pack 10, runner 19 and schedule 8 local scenarios pass. Runner guardian-negative observation now waits for durable guardian attachment acknowledgement. `/tmp/pctx-full-validation.log` and earlier runner logs retain the prior synchronization failure; they are superseded by this gate, not erased. The gate establishes the tested macOS revision and does not establish full contract or cross-platform acceptance.

`watch-tests.log` records CLI contract 5 and watch 4 passing tests. `restore-quota-tests.log` records quota 7 and work-control 8 passing tests. These establish their local scenarios only. No actual Linux/Windows runtime, installed Claude connection, authenticated GitHub pagination/write receipt, cloud execution or operating scheduler registration is established by these fixtures. Unknown capability is preserved rather than converted into empty/success.

`evaluation-smoke-debug.json` observes 100 generated files/51,200 bytes and one repetition, with unknown cold caches/reference equivalence. It is nonnormative. Session setup is unknown; verbose child exits are missing; one mandatory diagnostic is missing; binary trust and nonzero artifact observation are incomplete. The verbose aggregate records -78,166 net saved bytes (-34.08%) after retrieval. Parser-bundle eligibility and task-quality/provider outcomes remain unknown. These debug failures require correction before a new smoke run, then M/30 measurements; neither harness existence nor script completion verifies AC82.

Mandatory prose audit leaves three substantial independent next slices:

1. Complete the scheduled-operation lifecycle (§35–36, PCTX36/37/63): reviewed hash-bound owned bridge registration/removal, actual tick dispatch and durable job outcome, keep-awake TTL/cleanup, pause/silence/session-boundary reconciliation; distinguish install, planned occurrence, spawned operation and success. Schedule schema-v2 tick/bridge/keep-awake code is passes the local schema-v2 fixtures in `second-full-macos-tests.log`; actual native OS registration remains unverified.
2. Complete remote workflow and static profile drift (§29/34/37, PCTX35/38/52/61): authenticated read snapshots with pagination/error/permission scope, immutable PR head/WIP and deployment gates, approval/outbox reconciliation, inert monorepo check inventory and deduplicated DOC proposals. Local Git metadata and scoped rules do not cover these contracts.
3. Finish measured adoption evidence (§18–19/38/45, PCTX17/39/48/56/57): repair recorded smoke misses, include all packet/output retrieval and delivery bytes, run normative M/30 on disclosed environments and real platform CI, then separately authorized task-level provider/quality trials. Keep tokens, prices, quota, emitted bytes and accepted provider receipt separate.

Immediate integration debt is also explicit: runner guardian synchronization is locally resolved; canonical command aliases/capability help need conformance, and nested runner budgets plus delivery metering remain incomplete. None of these gaps is an exclusion from the active full goal.

### Third development integration evidence (partial contracts)

PCTX04/PCTX10: Unix no-follow project-root and ancestor validation, current-policy glob compilation without permission/source caching, and metadata candidate-first search are implemented. `tests/reader_policy_cache.rs` (4), `tests/search.rs` (12), and `evidence/third-full-macos-tests.log` establish these fixture contracts on macOS only. General changing-file races, Windows reparse-point protection, multi-root support and normative search performance remain incomplete/unverified; these tests do not promote the entire requirements.

PCTX01/AC63: `src/render.rs`, `tests/render.rs` (8) and actual CLI fixtures verify minified Compact/JSON, safe reversible Markdown and complete budget-error presentation. NDJSON remains governed by its separate streaming implementation. Tiny error-envelope minimum size is disclosed in `docs/cli/formats.md`. Broader final-budget boundaries remain subject to the full acceptance audit.

PCTX27/PCTX34/PCTX40: Windows native identity/Job Object foundation has code and seven platform fixtures, but it is unverified until actual Windows CI executes them. It is not the full launch/guardian/lease backend. macOS full gate passed 209 tests and all-target static checks; second CI proved published second macOS source only, while Linux/Windows failed. Third release M/30 remains pending; baseline target misses and unknown accuracy are retained in evaluation-m30-baseline.json/.md. No token, paid usage or release acceptance claim follows from these local results.

### Sixth integration evidence (bounded contracts)

PCTX01/04/10/12, AC09/10/18: Mandatory Project.root_anchor pins root and all ancestor instances; Unix opened directory and source/config identities are compared before publication. reader_policy_cache fixtures cover ordinary root/ancestor replacement, deleted-root recreation, source edits, symlink policy and nonblocking FIFO rejection. Five CLI capacity fixtures cover exact error-envelope minima, all exposed stdout-budget routes, no output files/children, unchanged index/control/lease/check/context ledgers and valid-budget mandatory-rule failure. sixth-full-macos-tests.log contains221 actual macOS passes; sixth-targeted-tests.log contains42; sixth-macos-static.log denies all warnings. Whole requirement statuses are not promoted from these scenarios: general timeouts, Windows pathname races/native application wiring, complete acceptance matrix and performance qualification remain unfinished.

PCTX27/34/40 native Windows: fifth-windows-ci.log proves13 suspended-admission/identity/Job Object fixtures on published4650e5b; it then records24 shared-module static failures. Keeper/lease integration is separate. fifth-linux-ci-failure.log preserves the actual later broker concurrency failure; earlier Linux green runs cannot prove this revision.

### Seventh integration contracts (verification in progress)

PCTX04/10/12: indexed metadata-only lookup is connected to the CLI with pinned generation and candidate authorization/hash-before-LIMIT. Five new real-index regressions cover current exclusion/deletion, complements, pre-LIMIT validation failures, staleness/body-only discovery, invalid policy/foreign workspace and replaced empty root. All18 search tests and51 total targeted macOS cases passed in seventh-targeted-tests.log; full/static gates pending. Generic snapshots remain physically filtered. Normative performance improvement is unmeasured.

PCTX27/34/40: windows_guardian implements process-owned canonical locks, nonce/native identity-bound durable admission ACK and sealed root-exit/empty-job release. The private CLI entry is registered; bounded parent pipe exchange uses owned cancellation workers with residual-worker capacity limits. Native Windows fixtures, lease/capture/cancel integration, guardian-death takeover blocking and reboot identity remain incomplete/unverified. macOS tests compile these modules out.

PCTX01/12/AC18: atomic publication has create-only8-writer conflict and open-reader replacement tests; two macOS passes do not prove Windows publication or crash durability. Windows same-directory MoveFileExW branch replaces Unix directory fsync; next actual Windows CI is required. Sixth report has body30/30 partial errors and unsupported labels0/10 measured, so full search accuracy/performance qualification remains incomplete.

Seventh full macOS gate now passed230 tests; static warnings denied and formatting passed. Seventh local archive installation/removal proof is in seventh-archive-validation.json; it does not establish forward-version update/migration, Windows packaging or full release acceptance.

Current seventh observation: indexed metadata p9527.891ms and matched symbol28.187ms,30 successful repetitions on unchanged M/200MiB corpus. Body30 partial samples and unsupported-language labels0/10 measured still prevent complete performance/accuracy acceptance. Source9c8b694 actual CI proves Linux/macOS full jobs and Windows13 foundation plus11 keeper functions; Windows atomic open-reader replacement fails, remaining core/static tests skipped. Component proof does not promote PCTX27/34/40 as a whole. Evidence evaluation-m30-seventh.* / seventh-github-ci-status.json / seventh-windows-ci.log.

### Additional admission and deadline contracts

| Work / acceptance | Contract and source | Implementation / evidence | Status and remaining scope |
|---|---|---|---|
| PCTX68 / AX20 | One finite-request deadline starts before project discovery; ordinary10s, indexing/strict build/checkpoints120s, explicit positive common timeout overrides. Nested phases share it; child execution has its separate timeout. Expiry must preserve active generations/checkpoints, avoid false deletions and mandatory-context omissions, distinguish safe partial matches from no usable result. §14,18,40; D030. | deadline/project/reader/storage/search/context/main and bounded query process; query_deadline_cli, reader_deadline, storage_deadline, project_deadline, derived_query_deadline and context_input_deadline fixtures. Initial native failures retained in ninth-intermediate-* evidence. | implementing; corrected native deadline/input/storage/derived/process lifecycle cases pass; ninth Mac274 tests pass; tenth saved-artifact/doctor/finite-stream/retained-accounting changes pass final Mac290 gate (tenth-full-macos-tests.log), including newline bytes and post-delivery expiration. Session/context entry points now retain the outer budget (library default10s), guard commits and selection loops; session and session_render_budget fixtures verify expired-entry and blocked-writer ledger/epoch preservation. Evidence: session-deadline-targeted.log and session-deadline-static.log; full session-deadline-full-macos.log records302 actual macOS tests PASS. One earlier relative-PATH timeout remains unexplained. Complete command coverage and platform/input/native I/O qualification remain required. Removing the old2s scan cutoff does not satisfy the separate2s measured body target. |
| PCTX69 / AX21 | Query admission must not execute repository-configured external conversion commands. Current Git status observes effective clean/process filter configuration under the same deadline and declines unsupported conversion before invocation; malformed configuration cannot become an empty success. §11,14,37 and owner invariants5; D031. | query_process internal exact config preflight and query_filter_guard actual Git fixtures. | implementing; three actual native Git fixture tests pass after correcting the stat-only baseline trigger. Preflight is a current configuration observation, not protection against hostile concurrent Git configuration/attribute edits. Stronger admission isolation or equivalent race protection remains required before a complete side-effect-free query guarantee. |

Eleventh PCTX12/PCTX44/AC13/AC76 progress: new context_adaptive7 fixtures prove distinct multiline parser signatures versus outline, exact original CRLF/excerpt ranges, deterministic intermediate tiers/final accounting, preserved mandatory decisions/rules, explicit-seed priority over imports, actual Markdown/escaped JSON byte counts and truthful unsupported tokenizer. New session_render_budget2 fixtures prove rendered-capacity failure creates no emission/event, exact capacity includes escapes/newline, canonical masked representation receipts, no automatic stdout acknowledgement, acknowledged stable deltas and retained-v1 baseline rejection. render9/session6/cli_contract14 complete the38-case targeted Mac gate. Static fmt/all-target Clippy pass after correcting fixture lint/schema/cardinality failures; whole eleventh Mac gate is running. These proofs do not establish shared adaptive selection across ContextGet, optional registered-engine counting, full §43 ranking/invalidation or Windows execution.

Tenth release qualification is scoped: evaluation-m30-tenth.* binds beeda2ff/binarydcf14408… and30 complete body scans; p952691.926416ms misses2s. tenth-archive-validation.json/tenth-upgrade-validation.json prove isolated Macarm64 install/source replacement/retention/removal, both0.1.0-dev; forward-version/schema migration remains unverified. tenth-github-ci-status.json records fullLinux/Mac PASS but fullworkflowFAIL dueWindows atomicrename87; keeper/latersteps skipped. Eleventh nativeNt correction is unverified until Windows execution. Monthly3000-minute Actions constraint requires targeted diagnostics and no repeatedfullmatrix retries; narrowgreen is not fullqualification.

D035/D036 native twelfth evidence: exact b96eff66/Windowsjob113167519852/run37733427929 records atomic_publication6 PASS and windows_guardian11 PASS in the scoped diagnostic. This qualifies create-only conflicts/Unicode, ordinary/delete-shared-reader POSIX replacement, denied-sharing failure with unchanged complete data/cleanup, and authored keeper fixture contracts. It does not prove full Windows reader/CLI/static/application runner/lease/capture/cancel/reconcile/unknown-job takeover, arbitrary filesystem behavior, crash durability or complete ancestor/staging races. Other Windows stages were deliberately skipped to respect monthly3000 Actions minutes. Historical failedfullruns retained; nooverallfullplatformpass.

D037 shared-selection integration: Build now delegates to context::select_with_measurement, allowing actual consumer envelope/escaping/newline cost to drive the same adaptive tiers. tests/context_delivery_selection.rs covers distinct delivery overhead, expiry/errors and mandatory capacity; shared-selector-targeted.log records20 targeted passes, shared-selector-full-macos.log records305 native macOS passes, and shared-selector-verification.json binds source hashes. D038 now connects ContextGet baseline/scope/receipt measurement to this engine; PCTX12/PCTX44 and AC76 remain implementing for complete ranking/invalidation/platform contracts.

D038 native evidence: adaptive-receipt-targeted.log records34 PASS, adaptive-receipt-full-macos.log records308 PASS, and adaptive-receipt-verification.json binds exact source hashes/basef69b8da and semantic serializer3/schema1. These qualify authored macOS shared-selection/receipt fixtures, not all PCTX44/AC76/AC77 ranking/invalidation, mid-selection epoch races or other platforms.

D039 adds current-versus-historical decision validity and metadata-only retirement references, explicit cancellation/supersession invalidations, non-Git instruction changes and final scoped-document snapshot revalidation. Authored fixed-Git and omitted-dependency race fixtures are in tests/session_render_budget.rs, tests/project_documents.rs and tests/context_delivery_selection.rs. PCTX11/PCTX44/AC77 remain implementing for the outstanding variants and platforms.

D039 final native proof: decision-lifecycle-targeted.log records33 PASS and decision-lifecycle-full-macos.log313 PASS; decision-lifecycle-format.log/static.log pass. decision-lifecycle-verification.json binds source hashes/basef62f3a4 and serializer4/schema1. Intermediate312 passes preceded the omitted-dependency race fix and are retained separately, not final-source proof. Actual scoped decision/instruction/supersession/race cases are qualified on macOS; whole AC77 and PCTX44 remain incomplete.

D040 implements persistent workspace decision retirement through replacement deletion and explicit accepted `reinstates` claims, using the existing control events. New source/CLI tests cover rollback on unknown restoration, workspace isolation, policy-hidden identifiers, retained backup history, corruption rejection and new-session reference-only delivery. Reviewed native macOS full gate317 PASS and targeted41 PASS; persistent-lineage-verification.json binds the final sources. Earlier D039 results do not prove these sources. Retained earlier query timeout remains unexplained; final gate success is not a root-cause repair. PCTX11/PCTX44/AC77 remain implementing.

D040 payload hardening: lineage-payload-verification.json records documents12 PASS and static/format PASS, with SQL byte/type sentinel and streamed per-row validation. Full native316 PASS/2 FAIL retains unresolved original1000ms startup failures; no whole-gate promotion. D041 topic/role consumer binding is a required planned contract with no runtime implementation proof.

D041 scoped delivery proof: delivery-barriers-verification.json binds Build/ContextGet consumer registration, pre-ranking and final original-deadline barrier checks, metadata-only baseline binding, current/alternate registered-role pause/resume and exact declared-topic/no-topic CLI suppression. Final-source related56 PASS, format/Clippy PASS; whole native325 PASS/3 startup FAIL. Source-topic classification, exception/handoff/task/approval and agent-bound Pack variants remain required; no complete AC77 claim.

D043 scoped PCTX46/AC78–80 evidence: pack-consumer-verification.json binds actual agent lease/consumer ownership, all four representations, pause/resume/epoch stale plans, topic/no-topic path-free suppression with no plan publication, private schema1 replan and public schema1 read compatibility. Actual CLI also verifies artifact --output with stdout response. Final native327 PASS/3 startup FAIL; related Pack11/session9 PASS, format/Clippy PASS. Concurrent final-pass changes, direct library timeout classification and native Windows publication remain unqualified; per-item topic/owner exception/memory variants remain required. This evidence does not complete the full Pack or AC77 contract.

D043 follow-up: pack-validation-race-verification.json proves committed whole-role/declared-topic changes after inventory admission fail final validation, and an original-budget planned-source TIMEOUT remains TIMEOUT7 in direct library calls. Final native329 PASS/3 startup FAIL; controlled2/Pack11/session9 PASS and format/Clippy PASS. Native syscall publication interval, actual agent-epoch race and Windows remain unqualified.

D044 scoped AC77/§32/43/44 evidence source-topics-verification.json: bounded source assignment/declaration union, unknown legacy hold under silence, pre-cap/tiering optional suppression without hidden metadata, mandatory policy conflicts, actual handoff/managed-Pack bypass regressions, task source-scope guarding and controlled policy mutation/deletion revalidation. Final343 PASS/3 startup FAIL; related59 PASS and format/Clippy PASS. Source topic attribution now has runtime proof in this scope; original-policy-owner exceptions/priority and broader memory/platform/recovery contracts remain implementing, not complete AC77.

D045 task-input authority implemented and scoped-verified: shared project source admission before task body, external alias-to-project classification, lexical project alias/traversal refusal, pinned external canonical parent and original alias/file identity/hash revalidation, opaque exact-budget provenance and one-shot stdin. Actual CLI and controlled source mutation/ABA fixtures cover the boundaries. Independent review identified separate pin/read identities; main repaired via actual shared-reader identity and demonstrated differing/identical-byte ABA rejection. Final native97671 terminal101:352 PASS/4 query startup FAIL; scoped33 PASS, format/locked all-target Clippy PASS. Evidence task-input-verification.json binds sources and all retained intermediate/final logs. No Actions dispatched. Whole mandatory goal remains active; original owner release/reporting exception provenance/priority/expiry, broader memory/raw-input/platform/recovery/performance and native startup gates remain required.

D046 scoped original-owner/reporting contract implemented and verified: immutable restriction owner/ID/revision/evidence, explicit legacy attestation and stale/wrong-owner/agent-run denial; separate owner-authored typed reporting rule scope/category/recipient/topic/priority/complete redacted Message hash/finite expiry. Actual Inbox/ack checks current rules; ordinary messages and execution remain denied. Restore preserves restrictions/owners, disables active exceptions, increments revisions and reinstalls identity/no-delete protections. Independent review found trigger/source-exclusion/full-metadata bypasses, repaired and re-reviewed. Final native28036 terminal101:358 PASS/3 original query startup FAIL; related46 PASS and format/locked all-target Clippy PASS. Earlier full357 PASS/4 startup FAIL and initial CLI/list-shape/static failures retained. Evidence owner-reporting-verification.json binds sources/logs. No Actions dispatched. Whole mandatory goal remains active; local operator identity is trusted transport only, automatic memory/approval/source/epoch/platform/recovery and authenticated external owner transport qualification remain open.

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

Common manual-run outcomes now retain attested spawn, typed prelaunch/processing
errors, child codes/signals and partial capture separately. Actual CLI proof and
retained initial failures: [manifest](evidence/pctx01-run-exit-verification.json).
PCTX01 stays implementing; remaining argument/budget/phase/platform qualification
and original startup failures prevent whole-task completion.


## PCTX01 Run refusal and capacity truth

Run pre-effect refusals and minimum capacity now include attested no-child facts.
Valid small budgets preserve original errors/final byte bound without requiring
an artifact ID. [Evidence](evidence/pctx01-run-budget-verification.json) records
both-mode minimum−1/minimum/minimum+1, missing-project6 and no-effects proofs.
Full oversized-error/post-spawn/platform gates remain; PCTX01 is incomplete.


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

## PCTX01 filter argument admission — 2026-10-08

Only PCTX01 is active, fixed G03-R09/G08, specification §14/§38. Activate reuses
existing ASCII ID grammar (nonempty, at most64 bytes, letters/digits/hyphen/
underscore) with FILTER_INVALID2, retained in private storage. Explain reuses its
existing1..256 argv count, without a new byte limit or empty-element prohibition.
Relative explicit Validate/Test/Apply input paths share the existing UTF-8 error2
before discovery or source reads. Absolute paths are deferred: relative() strips
the actual project root before UTF-8 conversion; the root itself may be opaque.
Apply still accepts ID or project filter path. Input '-' remains bounded stdin.
Original supplied expiry precedes producer grammar; existing default clocks and
Test/Activate write behavior are unchanged. Schemas/hashes/fixtures/current policy
and activation authority remain producer-owned.

Native macOS: eight invalid forms across32 JSON/compact and missing/initialized
project combinations preserve project/data/absolute response; response existence
follows initialization. Nine direct commands (including four opaque relative-path
forms) refuse typed2 without storage; the same nine expired calls retain the exact
original Instant/TIMEOUT7. Pure tests qualify ID64/65, Explain256 and accepted empty
elements/large UTF-8 input, identifier/project paths, stdin sentinel and deferred
opaque absolute paths. Actual ID/relative/absolute Validate and Apply preserve
preview child exit17 and storage; Explain256 succeeds without execution. Activation
without complete fixture report and valid nonowner activation deny5; outside-root
path denies5. No new successful activation or opaque-root native proof is claimed.
Final related18PASS; four controlled losses each0PASS1FAIL/exact restoration;
format/locked all-target Clippy PASS. Independent read-only review no blocker.
Initial unchanged-product baseline0PASS2FAIL proves POLICY_DENIED5 masks invalid
Activate ID and main missing-root IO7 masks2. Evidence:
`evidence/pctx01-filter-admission-verification.json`.

Fixed residual denominator12 unchanged: local10/12 (R01–R09,R12), whole required
platform0/12; whole PCTX01 gates0/10. Native Linux/Windows and four historical
startup conditions remain open. Next same-task boundary R10 existing adapter
labels; no independent filter feature, next official task or Actions.

Final native full27236 terminal101: 535 PASS / 4 FAIL. Failed tests: query_program_resolution_uses_captured_command_path, relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget, simultaneous_streams_are_collected_and_overflow_never_infers_success. All four historical startup conditions remain unresolved and original budgets unchanged. All jobs terminal; no heavy job live. Whole PCTX01 remains incomplete0/10.


## PCTX01 adapter identity admission — 2026-10-08

Only PCTX01 is active, fixed G03-R10/G08, specification §14/§38.
`src/main.rs` and `src/adapter.rs` now share existing unconditional Plan/Event
agent, Uninstall plan and Statusline task/pool/session/counter/key admission
before discovery, owner or input/state effects. Exact grammar: nonempty, at most
256 UTF-8 bytes, no controls or detected secret; whitespace remains accepted.
Original supplied expiry precedes direct admission. No new mutation/default
clock, protocol, config installation or independent adapter feature was added.

The registry's unconditional Event-key classification was incorrect: existing
import returns PreToolUse protection before consulting an explicit key. The key
is input-dependent and remains validated after parse/protection and before
receipt lock/DB/replay. Native protection accepts empty/control/257-byte unused
keys without command execution, host permission or state effects; other events
reject empty keys before receipts. This corrects the classification without
changing the fixed residual denominator12 or product grammar.

Native macOS evidence: nine invalid forms across36 missing/initialized and
JSON/compact combinations preserve project/data and absent/existing absolute
response files, with INVALID_ARGUMENT2/null project ID. Nine direct invalid
calls and nine expired calls preserve state and the exact original Instant;
isolated nonowner direct admission detects authority masking. Pure Plan checks
cover ASCII256/257, multibyte256/258, whitespace, controls and synthetic secrets;
hook/file conflict remains admitted before access. Actual owner Plan accepts
256-byte/whitespace identities without installing config; valid nonowner Plan
retains POLICY_DENIED9. PermissionDenied same-key replay preserves data and
exact durable bytes. Accepted Statusline boundaries and valid nonowner
Event/Statusline/Uninstall are not separately qualified by this new suite.

Baseline0PASS2FAIL retained; initial related22PASS, final related26PASS; four
controlled losses each0PASS1FAIL/exact source restoration. Final format/locked
all-target Clippy PASS. Independent read-only review found no scoped blocker.
Evidence: `evidence/pctx01-adapter-admission-verification.json`. Required native
Linux/Windows and four historical startup conditions remain open. Local residual
11/12(R01–R10,R12), whole required-platform0/12, whole PCTX01 gates0/10.
R11 is the next local boundary within PCTX01; no next official task or Actions.
Full native integration is terminal; this is not whole-task completion.

Source/spec reconciliation: adapter's existing error helper maps POLICY_DENIED
to9, whereas specification §14 requires policy exit5. R10's positive test records
the current behavior, not specification conformance. This is a mandatory current
PCTX01-G05 residual; do not mark stable-error completion or move to another task.
Resolve the shared producer error classification with a complete error/transport
matrix after the pending integration, preserving the recorded initial behavior.


Final native full98442 terminal101: 541 PASS / 4 FAIL. All four historical startup conditions fail at unchanged budgets: query_program_resolution_uses_captured_command_path, relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, simultaneous_streams_are_collected_and_overflow_never_infers_success, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget. All jobs terminal; no heavy job live. PCTX01 remains incomplete0/10.


## PCTX01 schedule admission — 2026-10-08

Only PCTX01 remains active, fixed G03-R11/G08, specification §14/§38. Existing
Add key, Update/Remove/Recover namespace/id/reason, List/Reconcile/Tick optional
namespace, RFC3339 --at and RunLoop interval/ticks/TTL/conditional purpose checks
now share admission before discovery/owner/input/storage. The private RunLoop
reuses the same helper. Original supplied expiry remains first, with no new
mutation clock or change to the existing finite read scope. Definition schemas,
revisions, ownership, eligible schedules, native registration and future Tick
prohibition stay producer-owned. No schedule business feature or OS mutation.

Exact labels remain nonempty, at most256 UTF-8 bytes, no controls/detected secret,
with whitespace accepted. Recovery reasons remain trim-nonempty and at most2048
bytes, without a new control/secret restriction. Loop bounds remain interval
1..3600, ticks1..10000, TTL1..86400. Purpose is trim-nonempty, at most256 bytes and
non-sensitive only when keep-awake is true; embedded controls remain accepted
by this predicate and ignored purpose when false is not newly constrained.
RunLoop namespace validation stays conditional on reaching an eligible tick.
An unmatched empty namespace can finish with no tick. Inspect/Uninstall lookup
keys and Recover occurrence/revision/attempt gain no invented grammar.

Native macOS:23 invalid forms ×4 missing/initialized and JSON/compact variants
(92 combinations) refuse INVALID_ARGUMENT2/null project ID before effects,
preserving project/data and absent/existing absolute response bytes. Direct23
invalid+23 expired calls preserve state and original Instant/TIMEOUT7. Isolated
nonowner admission proves grammar before authority; six valid mutations retain
POLICY_DENIED5 without state effects, expired invalid/valid calls retain timeout.
Pure boundaries qualify labels256/multibyte256/whitespace, reason2048 with
controls, all numeric maxima/minima, RFC3339 offsets/fractions/future syntax,
conditional purposes, and unrestricted lookup exceptions. Actual List256 and
whitespace return empty without writes; future Tick refuses before execution;
unmatched empty-namespace loop performs no tick/model/native wake assertion;
actual Add256 same-key replay preserves data and exact durable bytes.

Baseline1PASS1FAIL (missing-root7 masks2) retained; related25PASS then28PASS.
Five controlled losses each0PASS1FAIL, exact source restored. Initial Clippy
collapsible-match failure retained; equivalent List Some-pattern repair passed
final format/locked all-target Clippy. Loss and related evidence precede that
style-only change; final native full covers final source. Independent read-only
review no scoped blocker. Numeric maxima/purpose acceptance are grammar proofs,
not a long runtime/native wake assertion; successful Recover/Update/Remove
transitions remain outside this admission fixture. Evidence:
`evidence/pctx01-schedule-admission-verification.json`.

Fixed residual denominator12 unchanged: local12/12(R01–R12), whole required
platform0/12; whole PCTX01 gates0/10. This closes the identified local residual
register only, not all CLI matrices/gates or required native platforms.
Linux/Windows and historical startup conditions remain open. Next same-task
boundary: PCTX01-G05 adapter source/spec stable-error discrepancy. No next
official task or Actions. Full native integration is terminal (547 PASS / 3 FAIL).


Final native full98360 terminal101: 547 PASS / 3 FAIL. Failed tests: relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, simultaneous_streams_are_collected_and_overflow_never_infers_success, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget. Captured-command-path passed incidentally without a startup/resolution repair; all four historical conditions remain unresolved and original budgets unchanged. All jobs terminal; no heavy job live. PCTX01 remains incomplete0/10.


## PCTX01 adapter error classification — 2026-10-08

Only PCTX01-G05/G06 is active, specification §14 plus extension §27/§38.
The general exit table does not remove extension state/revision/lease conflict9.
Adapter now distinguishes input/settings safety cap2, original plan/config bytes
changed4, policy/path5, unavailable protocol6, storage/schema7, and actual
receipt/binding/ownership conflict9. No global conflict or budget remapping.
CONFIG_CONFLICT retains context: hooks object/event-array input2, malformed
stored ownership object7, missing ownership/modified owned hooks9. PLAN_MISMATCH
likewise distinguishes supplied hash syntax/equality2, changed stored bytes4,
unsupported plan schema2, and project/workspace/additions binding9. Existing
codes remain; unsupported plan schema has a more precise message. Clocks, owner,
replay, publication and business predicates remain unchanged. Historical policy9
evidence is retained at its exact prior source; current policy5 supersedes it.

Persisted receipts.result and installs.owned JSON parse failures are now generic
DB_CORRUPT7, distinct from user JSON2. Receipt object/event/hook_output/incomplete
shape and ownership group arrays are checked before returning a result or
publishing config; a malformed group cannot panic. Incomplete intent retains
reconciliation9 before normal-result shape checks. This is common error/success
truthfulness, not a new adapter feature.

Evidence: baseline0PASS4FAIL retained; first related35PASS, added hook/IO36PASS.
First full61665 terminal101:554PASS4FAIL before stored JSON repair, all original
startup failures retained. Stored JSON baseline66073 terminal101:0PASS1FAIL
proves INVALID_ARGUMENT2 instead of DB_CORRUPT7. Final related36322 terminal0:
37PASS; final target41532 terminal0:9PASS includes expanded event/output/flag
shape fixtures. Native owner policy5 is qualified for JSON/compact; unsupported
protocol6 includes real stdin hook with complete JSON then EOF and standard error
envelope. Native input cap/config shape2, symlink5, absent input IO7 preserve
project/data; symlink target/link bytes/identity remain unchanged. Direct cases
qualify input/source/schema/binding distinctions, original TIMEOUT7, future/empty
DB schema7, stored ownership/JSON7 and same-key replay/conflict/reconciliation9.
Snapshot is taken after initialization/corruption, not treated as rollback.

At runtime064ab55, Install's invalid-hash path created the adapter lock before
refusal. This historical G03-C004 defect is repaired in the later C004 integration
below; its original observation remains source-bound. Unbound PreCompact stores
an incomplete intent before binding9; retry9 requires reconciliation, not rollback.
Five initial classification losses and two stored-JSON/shape losses each actual
0PASS1FAIL/exact restoration. Both stages are hash-bound; new shape subcases were
added after loss execution and pass final target. Final format/locked all-target
Clippy PASS; independent read-only review no scoped blocker. Every message and
retryable field is not separately asserted by this fixture. Full final source
integration is terminal (555 PASS / 4 FAIL). Evidence: `evidence/pctx01-adapter-errors-verification.json`.

Whole PCTX01 gates remain0/10, native Linux/Windows unverified. Historical fixed
label/admission residual groups remain locally12/12 and required-platform0/12;
that bounded list does not close all G03 leaf contracts. C004 is an existing
visible leaf in the fixed140-leaf register, not a new official task or group.
Next same-task boundary: move its existing Install hash admission before
discovery/lock, preserving original expiry and valid owner/state behavior.
Four historical startup conditions remain unresolved; no Actions/next task.


Final native full4105 terminal101: 555 PASS / 4 FAIL. Failed tests: query_program_resolution_uses_captured_command_path, relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry, simultaneous_streams_are_collected_and_overflow_never_infers_success, concurrent_query_startup_preserves_group_streams_and_original_one_second_budget. All four historical startup conditions remain unresolved, original budgets unchanged. All jobs terminal; no heavy job live. PCTX01 remains incomplete0/10.


## PCTX01 C004 Install hash admission — 2026-10-08

PCTX01 remains the only active official task. C004 now shares its exact existing
64-byte ASCII-hex and byte-equality predicate before CLI project discovery and
direct authority/lock effects. Original supplied expiry remains first. No case
normalization or separate expected-hash grammar; uppercase/mixed hex remain
admitted. Stored bytes/schema/project/workspace/additions/config comparisons stay
in the producer, with owner authority and publication unchanged.

Evidence: evidence/pctx01-install-admission-verification.json. Baseline1PASS4FAIL
retains created lock, policy-priority and missing-root failures. Related38PASS
covers64 native invalid combinations, eight direct invalid plus eight original
expired requests, isolated nonowner priority, exact accepted grammar and actual
isolated Plan→Install preserving permissions/statusLine. Existing stored-byte4,
schema2, binding9 and stale-config4 tests remain. The contention fixture now uses
a valid64 hash to reach its held lock; original50ms request and no-effects
criteria unchanged. Initial invalid target selection and pre-repair contention
failure are retained. Three actual guard-loss failures restore source exactly;
format/locked all-target Clippy PASS. Independent read-only review no blocker.
Upper/mixed-case successful stored-plan installation is not claimed.

Whole gates0/10, visible leaves140 and historical local groups12/12/platform0/12
unchanged. This repairs a predicate of the existing leaf, not a new residual
group or official task. Native Linux/Windows and all four historical startup
conditions remain open. Final full results are recorded in the manifest and
current handoff. No next official task or Actions.

Final native full32591 terminal101: 560 PASS / 4 FAIL. All four historical
startup conditions fail at unchanged budgets; failure names/source hashes in
the manifest. All jobs terminal, no heavy job live. PCTX01 remains incomplete0/10.


## PCTX01 refusal envelope consistency — 2026-10-09

PCTX01-G02/G04/G05 only, §8/§14. Actual parser/admission/render errors inherited
coverage=complete despite refused execution. Shared domain::error_envelope now
uses error status, partial coverage/code reason while preserving supplied data,
identity, error/retryable and timestamp. Parser/admission/render consume it;
minimum-budget/outerexecute are equivalent refactors. Exits/clock/authority/
success/stream/delivery semantics unchanged. Partial coverage does not imply3.

Manifest evidence/pctx01-refusal-envelope-verification.json: baseline1PASS3FAIL,
related66PASS, renderer1PASS (no clock input2/original expired TIMEOUT7), final
target4PASS. Native618 attempts: frozen140 leaves × format/destination560 (280
JSON envelopes,280 compact plain stderr), C004 required/duplicate options16,
semantic/representation6 and admitted state/destination36. Producer classes
5/4/2/9/4/7 retained;24 error envelopes delivered,12 existing-response attempts
refuse publication9 and preserve original bytes. All root/data bytes unchanged
after prepared fixture state; pre-created lock is not first-use rollback.

Coverage guard loss yields three actual failures/exact restoration; data loss
fails real minimum-budget metadata test. An initial Run data-loss probe passed
because later facade reconstructs prelaunch evidence; retained as inconclusive,
not mutation sensitivity. Initial generation/target-selection and Clippy
collapsible-if failure retained; equivalent fixture style fix/final static PASS.
Independent read-only review no blocker. Full final results in manifest/handoff.

Whole gates0/10, visible leaves140 and historical local groups12/12/platform0/12
unchanged. Complete remaining argument/representation/budget/phase/platform
conditions remain mandatory. Native Linux/Windows and four historical startup
conditions remain open. No Actions/next official task.

Final native full51862 terminal101:566PASS3FAIL. Captured-command-path passed
without startup repair; all four historical startup conditions remain unresolved
and original budgets unchanged. Full failures/source hashes in manifest. All
jobs terminal, no heavy job live. PCTX01 remains incomplete0/10.


## PCTX01 C001 Activity cursor admission — 2026-10-09

Only PCTX01 active; this integration covers G03/G08. Reuse exact existing nonnegative i64 cursor/error2/
message across CLI, Work and Watch. Normal admission precedes discovery; stream
admission follows output/route guards before discovery, preserving JSON-follow
refusal and stderr error records. Work original expiry-first unchanged; Watch
now checks supplied expiry before cursor/stdout with no new/default clock.

Manifest evidence/pctx01-activity-admission-verification.json: baseline1PASS2FAIL
retains missing-root IO7 masking cursor2 and expired-negative Watch input2.
Related48PASS, final target4PASS. Native48 negative combinations plus32 typed
parser refusals preserve root/data/existing response bytes and exact error
transport/priority. Direct negative/expired Work and Board/follow Watch variants
retain same original instant/state. Finite0/i64-max across three formats preserve
empty events/next cursor; snapshots follow initial storage setup, not first-use
rollback. Exact default cursor field and new live follow/active-deadline positive
behavior are not separately asserted. Existing bounded watch suite is included.

Four actual guard-loss failures/exact restoration cover frontend/stream/Watch
expiry/cursor grammar; final parser fixture follows losses. Format/locked
all-target Clippy PASS; independent read-only review no blocker and restored
source confirmed. Final full/publication status in manifest/current handoff.
Whole0/10,140 leaves and historical local12/12/platform0/12 unchanged. This is
existing C001 admission, not a new task/group/stream feature. Native platforms
and all four historical startup conditions remain open. No Actions/next task.

Final native full13204 terminal101:570PASS3FAIL. Captured-command-path passed
without startup repair; all four historical startup conditions remain unresolved
and original budgets unchanged. All jobs terminal, no heavy job live. PCTX01
incomplete0/10. Offline read-only Linux arm64 container executes, but cargo/rustc
absent; capability log is environment proof only, not native PCTX verification.


## PCTX01 native Linux verification — 2026-10-09

G05/G08/G09, §14/§38: opened Linux proc-stat descriptors can return ESRCH when
the observed process is reaped. src/query_process.rs now classifies only that
vanished-process condition as absent; required pinned-root observation, other
read errors/size/parser refusal and conservative zombie members remain. Actual
FD-after-reap regression, missing-root refusal, EACCES failure and unchanged live
bytes are verified in the fresh native Linux44-test library suite; original
project_deadline14-test group/stream/budget suite also passes.

Evidence: evidence/pctx01-linux-verification.json. Linux aarch64 Ubuntu24.04.4
Rust1.99 VM fresh full578PASS1FAIL; concurrent schedule INDEX_BUSY remains open.
Earlier555PASS22FAIL (noexec/no init),575PASS2FAIL and mutation/contaminated-cache
failures retained. Source hash alone did not bind shared-target artifacts; the
launcher now isolates build targets by source hash, with Git archive and binary
identity independently recorded. Independent read-only reviews found no scoped
production blocker. This is actual platform execution, not whole-G09 closure,
Windows/x86_64 support or macOS startup repair. Fixed whole0/10,140 leaves and12
historical groups unchanged. Next same-task audit classifies the persistent Linux
DB admission contention; independent PCTX36 functionality remains inactive.
