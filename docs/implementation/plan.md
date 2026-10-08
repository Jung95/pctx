# Implementation plan

Baseline: all 45 sections of `docs/PCTX-implementation-spec-v0.6.md` and the user goal objective. `requirements.md` owns the item/AC mapping. This plan establishes dependencies and integration boundaries; it does not claim completed features. All groups start `not_started`; the main integration owner updates statuses from reviewed code and execution evidence.

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

Next: main integration completes dependency acquisition, common contracts/scaffolding and PCTX01–04/06. Review first actual CLI evidence before changing trace statuses. Then complete the safe exploration and durable task vertical flows; preserve later concrete work in the active backlog.

## D045 integration

Main integrates the task-file boundary and exact-output provenance. work_control owns the bounded pinned-input primitive/direct fixture authoring; requirements independently reviews authority/deadline/alias contracts without edits or test execution. Main runs native CLI/race/static/full verification and commits/pushes development work with no Actions dispatch. Shared reader identity support stays with main. The original-owner reporting-exception/release/priority contract and broader approval/raw-input memory delivery remain subsequent mandatory work, alongside performance/native startup/platform/recovery gates.

## D046 integration

work_control authors the new policy-controls module/schema only; main owns operations/message/deadline and restore integration, tests, public docs and native verification. requirements independently reviews invariants without edits/tests. Follow-ups retain broader automatic source/memory/approval exception qualification, authenticated external owner input, platform/recovery, finite Work/adapter budgets, ranking and native startup/performance gates. No role/report exception slice can complete the full goal.

D047 reporting source-attribution slice: shared authored source classification, source-label/packet-topic conflict union, explicit original-owner cross-topic references, unknown-source hold, and final role/recipient plus Inbox batch expiry/source proofs. Actual CLI covers mixed restricted sources and managed metadata changes blocking Inbox/ack; three controlled helper tests cover real expiry and last-admission/batch source changes. Independent review identified both sequential admission boundaries and found no remaining blocker in the scoped repair. See report-source-verification.json for exact final evidence. This does not verify source-body attribution truth, atomic final output, all memory/approval/receipt/Pack variants or native Windows/Linux; AC77 and the whole mandatory goal remain implementing. No Actions dispatched.

D047 final native55189 terminal101:363 PASS/3 original query startup FAIL. Related policy7/source11/operations11/session13/schedule17 PASS, controlled helper races3 PASS, format and locked all-target Clippy PASS. report-source-verification.json binds exact source hashes, retained initial compile/static failures and final logs. No new Actions.

D048 search candidate admission implements the proposed pinned-reader optimization for body/freshness search without matching ambiguous public error codes. Initial candidate-local rejection is separated from root/config/deadline and post-admission failures; ordinary reader retry/timeout partial contracts remain distinct. Independent review caught and repaired a non-Unix missing-candidate regression. See search-candidate-verification.json for exact evidence. Native Windows reparse/access behavior and a new immutable M30 remain required; the historical 2691.926416ms body-search p95 is still a target miss, not superseded by unit tests.

D048 final native18183 terminal101:367 PASS/4 original query startup FAIL. Candidate admission tests5 PASS, related search/reader/security/CLI/source/task/session tests PASS, format and locked all-target Clippy PASS. search-candidate-verification.json binds source hashes/final logs and retains initial static permission-literal failure. No Actions or new benchmark result.

PCTX01 remains the only active official task (`implementing`). Help/color integration: native52787 terminal101,377 PASS/3 original query startup FAIL across45 suites; actual frontend8 PASS and explicit leaf-contract coverage1 PASS; format/locked all-target Clippy PASS. Evidence: pctx01-help-color-verification.json. Help now states standard JSON schema1.0, per-leaf read/write/execute and conditional flags, native hook/NDJSON and Pack output exceptions. Parser diagnostics mask reflected secrets and escape controls; no-color applies before nested help parsing, qualified on a real Unix PTY. Independent review findings resolved. Original failures are retained with unchanged one-second budgets; native Linux/Windows, remaining finite routes and full argument/envelope/exit matrix are still required. No next official task selected; no additional Actions dispatched.

## Current PCTX01 auxiliary deadline integration

PCTX01 remains the only active official task (`implementing`). Latest native full35833 terminal101:386 PASS/3 original startup FAIL across46 suites; auxiliary targeted1 PASS, format/locked all-target Clippy PASS. Evidence: pctx01-auxiliary-deadline-verification.json. Opened auxiliary workspace source reads preserve original1s deadline: positive before expiry, changed source refused TIMEOUT/exit7 after expiry, no index/control DB creation. Controlled original-loss mutation fails the regression and exact source is restored. Isolated inert metadata proof is distinct from retained real CLI linked-worktree planning. Independent review found no blocker. Mid-discovery/hash and command-level aggregation expiry, other finite routes, full frontend and native Linux/Windows remain unverified. Historical startup variants remain unresolved; no retry-success promotion, next task or new Actions.
