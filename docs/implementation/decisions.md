# Implementation decisions

Baseline: `docs/PCTX-implementation-spec-v0.6.md`, all 45 sections, and the user goal objective. Decisions below interpret implementation boundaries; they do not weaken acceptance criteria or claim working code. Integration owns code/schema decisions and can supersede an entry with rationale and evidence.

| ID | Decision and rationale | Source / acceptance impact |
| --- | --- | --- |
| D01 | Use Rust Core with Domain/Application services, local workspace index SQLite and durable coordination control SQLite. Physical crate/file subdivision is an integration choice; recommended crate count is not a mandatory architecture. | §5,9,21,24,27; preserves AC06–08,19,25,34 |
| D02 | Implement all 48 baseline items and concrete v0.2 contracts. Track otherwise unassigned multi-root, metadata export/import, read-only API/serve, GitHub writes and bounded relations as PCTX49–53 rather than silently excluding them by release phase. | User full-goal scope; §3,15,16,27,34,42–45 |
| D03 | Optional/review-only technologies remain separately classified. A required tokenizer interface supports explicit unsupported results; concrete engines, semantic search, LSP, exact runtime call graph, Base/Overlay and external compressor executables are optional. | §3,12,15,16,21,23,39,42; prevents false supported claims |
| D04 | Reference beSir profiles/roles/paths/models/brands/account selectors are inert data/fixtures, never Core constants or development-host policy. PCTX uses generic minimal profiles and validates unsupported connections. | §28,32–37; AC52,54,62 |
| D05 | `persist_source=true` remains rejected for the permanent index. Explicit masked temporary command output and explicit source packs have their own opt-in stores and lifetimes; they do not change this setting's meaning. | §7,9,17,23,40,44–45; AC67–68,79 |
| D06 | JSON envelope and exit mappings are shared by all frontends. `run` uses the documented child-exit exception; child outcome, wrapper/capture/parser failures and task gate result remain independent structured fields. | §8,14,27,38,40,45; AC13,63–65 |
| D07 | UUID and SHA-256 identities follow the spec; no branch-name workspace identity. Worktrees share only validated local coordination and permitted content cache, never dirty manifests. Clone identity alone never joins boards. | §6,16,27,29; AC06,25,40 |
| D08 | Error-free parser output or a compressed summary cannot constitute passed test evidence. The most recent valid attempt for the exact target/definition/environment governs gates; zero or all-skipped unit reports fail minimum count. | §27,40–41; AC28–31,64,72 |
| D09 | Singleflight is exclusively query refresh. It is never applied to child executions; replay of a command receipt is distinct from conflating two execution intents. | §27,29,40; AC32,39,70 |
| D10 | Reader freshness and policy are applied at every current access. Stored generation metadata is not proof of present source; matching byte hash is required for symbol/diagnostic ranges. Strict mode reenumerates but never claims atomic filesystem snapshot. | §8–12,17,42; AC04–05,09–11,17,73 |
| D11 | Emission, delivery, acknowledgement and comprehension remain distinct. The single context ledger is scoped to session/epoch/policy/scope; new epoch cannot inherit a baseline, and invalidated/deleted decisions/grants/files produce tombstones. | §13,30,38,43; AC16,42–44,76–77 |
| D12 | Task/run leases and host resource ownership are separate. No TTL or parent death releases a resource while child survival is unknown; explicit cancellation waits for process exit. A legacy bridge must share the canonical lock, else heavy capability is disabled. | §33,40; AC50–52 |
| D13 | Keep capability claims narrow: fixture support, installed-schema inspection and live connection are separate states. Core local/manual flows proceed even if external capability or credentials are absent; protected gates fail closed. | §21,36–38; AC21,58–59,62 |
| D14 | Trusted owner decisions bind precise action fingerprint/environment/argv/artifact/cost/expiry/actor. Agent messages and a shared GitHub identity do not prove owner approval. Product grants never bypass host tool permissions. | §32,34; AC45–46,55 |
| D15 | Pause/silence is durable policy, checked before action admission or ranking/delivery and restored unchanged. Quota reset, schedule reconciliation and new sessions cannot resume it. | §31–35; AC49,54,61,77 |
| D16 | User's development repository commit/push instruction is authorized. This does not authorize publishing a package/site, mutating user operating issues/accounts, paid model calls, global Claude hooks or OS schedules merely because those features are in the spec. Implement using isolated fixtures and prepare concrete live validation steps. | User goal §§2,6,7,9; product §3,35–36 |
| D17 | Verification status tracks observed code and executions, never analysis coverage. Requirements rows start not_started; planned test/evidence paths are not existence/pass assertions. Main integration alone promotes statuses after checking evidence. | User goal traceability and final completion requirements |

## Gaps with conservative minimum contracts

| Gap | Minimum implementation contract / next decision | Check and limits |
| --- | --- | --- |
| Multi-root and Tool API lack detailed schemas | PCTX49 assigns stable root IDs and rejects ambiguous/cross-root paths. PCTX51 offers explicitly activated bounded read methods mapping to shared Application commands and versioned JSON; no remote authenticated mutation unless separately specified. Freeze fixtures before implementation. | AX01/03; later remote services remain optional |
| v0.2 external provider writes need live account state | PCTX52 implements safe intent/outbox/conflict/reconcile code with offline protocol fixtures. Live writes require concrete authorized target/action and actual credentials. Never treat fixture receipt as remote success. | AX04; AC41,55,57 |
| Complete `refs` and later call relationships are not fully specified | PCTX53 implements references from supported static import/explicit document edges, with direction, bounds and unresolved coverage. It must not relabel text hits as symbol references or claim a full call graph. | AX05; AC75 |
| Minimum OS versions/libc not defined | Determine supported versions from actual CI/runtime evidence and declare before release. Cross-compilation is build evidence only. Until then platform claims remain unverified. | §22; AC22; AX06 |
| First labeled corpus/public snapshot missing | Build reproducible secret-free synthetic corpus and record exact generators, sizes/mix/symbol distributions. Select redistributable real snapshots with license and immutable revision before measured quality claims. | §18–19; AX08/09 |
| Tokenizer engine/version list open | Freeze interface/error behavior and byte budgets first; select optional exact engines only with official version/license and regression fixtures. Unknown tokenizer refuses token guarantee. | §12,21,23; PCTX13 |
| Package/distribution name availability unknown | Check configured publication channels before publishing; build archives/checksums/license notices independently. No implied package publication authorization. | §22; PCTX18/54 |
| Message archive/usage retention lacks numeric defaults | Keep original decision/pause/approval evidence and capsules durable; no automatic destructive archive purge until a trusted explicit retention policy exists. Aggregation never deletes required provenance. | §38; AX10 |
| Normalization/masking patterns are bounded protection, not universal secrecy | Use synthetic secrets and chunk-boundary regression; disclose redacted/incomplete records, enforce path exclusion and before-disk masking; unrecognized secret formats remain documented detection limits. | §17,40; AC67 |
| Signal/Windows child semantics vary by platform | Preserve native observed status in JSON and use documented shell convention where possible. Test on real platform; avoid interpreting signal as regular exit/pass. | §40; AC22,63 |
| Dynamic package/alias/config forms unsupported | Parse only explicit static JSON/package configuration, report unresolved/unknown forms and do not execute configuration to discover it. More resolvers require independent fixture acceptance. | §37,43; AC62,75 |
| Schedule occurrence defaults need executable fixtures | Use IANA timezone, wall-clock once-per-local-date/next-valid versus elapsed interval, uniqueness by definition revision+occurrence; coalesce missed recurring reads while preserving actual success timestamps. | §35; AC53–54,61 |
| Permission incident exact command may contain secrets | Store masked structured descriptor plus safe host-request reference and redaction locations. Preserve execution-not-started evidence without logging credential values. | §32,40; AC46,67,69 |
| Performance/savings require expensive or unavailable runtime evidence | Implement reproducible free harness first; record p50/p95/max and unmet/unknown actual usage. Paid evaluations and unsupported real-platform/live checks remain unverified; do not lower goals or substitute byte/4. | §18–19,38,45; PCTX17,39,48,56–57 |

## Initial observations

This analysis read the 2418-line specification in full, including every section, all PCTX01–48 rows, all AC01–82 rows and command tables/examples. No AGENTS.md was found at `/`, `/Users`, `/Users/dev` or `/Users/dev/PCTX` during inspection. The source specification remains unchanged. Toolchain, Git registration, credentials and running jobs are confirmed and maintained by the integration owner; no transient environment condition is assumed to permanently block local implementation.

Documentation-only coverage check: an automated ID audit found exactly one row each for PCTX01–48 and AC01–82, plus 18 followup work/check rows; each baseline row starts `not_started`. All three owned documents were checked for English-only prose. This does not verify product behavior.

## D018 — Preserve the original input and publish English artifacts

The owner supplied the normative specification in Korean and requested English public documentation. The source file remains unchanged locally and is excluded from publication while a faithful English edition is prepared. The English requirement tables preserve all 45-section coverage and work/acceptance references, but are not represented as a full translation. Public specification translation is additional required documentation work (PCTX67 / AX19), and cannot be treated as completed by this development snapshot.

## D019 — Atomic derived-index rebuild and recovery evidence

Rebuild creates an independent index, verifies source hashes and database integrity, preserves readable checkpoints, consolidates WAL under the shared publication lock, and atomically replaces the active file. The previous file remains as a quarantine hard link. A corrupt database cannot truthfully recreate historical checkpoints from current source; its original bytes and sidecars are retained and a recovery warning is returned. A newer schema fails closed. Reader snapshots and checkpoint operations participate in the same lock. An independent review rejected a preliminary two-rename publication window; the corrected implementation uses one replacement. Real process-kill and Windows replacement coverage remain unverified.

## D020 — Static inventory claims and portable schedule bindings

Explicit profile schema 1 records current file hashes and observed static selectors. A project file claiming owner confirmation proves only the observed claim/hash; it never grants local owner or operation authority. Document source_refs use exact section, rendered template, source/document hash, and deterministic coordination-scoped proposal IDs; proposals require later authorized task/decision integration. Dynamic configuration is not executed for discovery.

Schedule schema 2 adds local execution bindings, attempt history and reviewed installation metadata. Portable backups strip bindings and private installation paths/environment. Restores invalidate bindings, retain finished attempt/results and durable pauses, mark live attempts interrupted_unknown and installation state unknown_restored. Restore never registers an OS bridge or revives execution permission. Migration and restore regressions must pass before verification status is promoted.

## D021 — Policy programs and current root identity

Only compiled glob programs are shared, keyed by the exact current exclusion vector plus fixed security exclusions. An eight-entry eviction bound controls entry count, not bytes or RSS. Permission, source contents and root identity are never cached. Unix root opening walks every absolute component with no-follow directory descriptors; only the fixed macOS /tmp and /var aliases are accepted when their actual link target exactly matches the corresponding /private path. Metadata searches filter current policy before candidate evaluation and validate physical candidate paths before output. Unmatched indexed metadata is not a source-discovery guarantee.

## D022 — Presentation budgets and execution truth

Compact and JSON retain one minified complete envelope; Markdown has readable evidence plus a reversible full JSON appendix. A final budget overflow produces a JSON error instead of truncated source/JSON, retaining available output handle and child outcome. Wrapper budget failure exits 8 even under child exit policy. Complete error documents have a minimum representable size, so arbitrarily tiny requests cannot be represented as successful bounded context. Renderer tests and CLI fixtures are separate evidence; neither establishes model-token savings.

## D023 — Windows suspended admission is a component contract

The native module atomically assigns the child to a private Job Object with STARTUPINFOEX JOB_LIST and creates it suspended with restricted inherited handles. Its explicit resume API requires the caller to establish durable identity-bound guardian ACK; module fixtures cannot supply production lease proof. Empty accounting snapshots and object-handle closure never independently authorize resource release. Capture pipe readers must be drained concurrently by the shared bounded collector. Host formatting/static gates do not establish native Windows compilation; actual Windows CI is the next evidence boundary. Unicode environment-name support and guardian/lease integration remain pending.

## D024 — Loaded root identity and safe file-type admission

Project authority includes immutable handles to the root and all ancestors, not only a pathname or persistent project ID. Unix comparison caches captured dev/inode pairs while retaining the actual original handles against inode reuse; every later traversal compares newly opened descriptors. Configuration is loaded through the same anchored no-follow chain, bounded to a regular1MiB file and compared with a reopened descriptor before publication. Source reads similarly reopen and compare actual file identities. Ordinary source edits remain valid; a root or ancestor replacement requires a new explicit project load, never automatic authority refresh. Windows checks reparse attributes/identities but still lacks descriptor-relative pathname-race proof and complete ReFS128-bit identity qualification.

Unix file admission uses O_NONBLOCK for the final opened component so a FIFO cannot stall before the regular-file check; this follows the [POSIX open contract](https://pubs.opengroup.org/onlinepubs/007904875/functions/open.html). This is not a hard deadline for regular/network-file I/O. Unix reader optimization removes only redundant pathname authorization: current policy checks plus anchored O_NOFOLLOW traversal still occur before both source opens. Non-Unix retains pathname authorization pending a stronger native traversal backend.

## D025 — Invalid capacity precedes application side effects

The CLI derives a command-specific conservative minimum from a complete newline JSON INVALID_ARGUMENT error envelope, including a maximum fractional timestamp and its own measured minimum field. Smaller stdout capacities fail argument validation (exit2) before loading a Project, writing an output file, publishing an index, recording a receipt, or admitting a child/check. Budgets above that minimum can still fail required-context/evidence admission with BUDGET_TOO_SMALL(exit8). Pack capacity is a separate artifact budget. Detailed executable boundaries are in docs/cli/formats.md and tests/cli_contract.rs; underlying API metadata minima remain separate from this CLI guard.

## D026 — Native publication and shared refresh deadlines

Windows atomic_write flushes and closes a complete same-directory temporary file, then uses [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw) with WRITE_THROUGH and optional REPLACE_EXISTING. No cross-volume copy fallback or volume/admin flush is used. The Unix directory-fsync path remains unchanged. This corrects a path whose Unix directory-open/fsync behavior cannot be assumed on Windows; native power-loss qualification remains unverified. Both platforms classify a lost create-only publication race as REVISION_CONFLICT, preserve the winner and clean temporary files.

The broker separates freshness TTL from waiting: default query waiting is10s (§11), with one monotonic budget across waiting and Git probes; bounded callers can obtain truthful refresh_pending without taking a live claim. Existing DB initialization and identity subprocess limits are separate and remain part of the whole-command deadline audit.

## D027 — Candidate-first DB metadata search

Only path/symbol/document lookup uses a private current-policy metadata snapshot from one pinned workspace generation. Every actual expression match receives physical authorization and required hash validation before ranking/LIMIT. Generic snapshots and text/all retain physical filtering. Unopened historical noncandidate cardinalities are suppressed with scanned_files=null and physical_non_candidates_checked=false; no claim of a complete current filesystem snapshot follows. Strict refresh coverage is preserved by the CLI. Performance improvement requires a new equivalent measurement.

## D028 — Locally observed check bindings

Native check evidence retains the trusted current profile, entry executable/script hashes, configured environment, captured PATH bytes, working directory, workspace and policy binding from the supervisor artifact. Child-reported environment is a separately labelled claim. Recording, criterion acceptance and completion gates re-evaluate the current binding; missing legacy bindings or changed/removed profiles cannot authorize a native pass. Explicitly allowed external reports retain their claim semantics and no local environment authority.

Manifest hashing can take time, so the supervisor repeats binding/trust validation immediately before spawn and durably records not_started on rejection. PATH supplied to the child and its fingerprint use the same captured bytes. This narrows the admission window but does not pin the executed image against an ABA race. Transitive tools resolved by scripts, dynamic libraries, OS/platform versions and undeclared dependencies remain unverified; this binding is not a complete toolchain fingerprint.

## D029 — Replace an open Windows file by native handle

Actual Windows CI rejected D026's MoveFileExW replacement while a non-delete-shared reader held the target. The pending correction uses a flushed source handle opened with DELETE access and a pinned non-reparse directory handle, then FileRenameInfoEx with REPLACE_IF_EXISTS and POSIX_SEMANTICS for replacement. Create-only publication omits those flags. The bounded relative UTF-16 name and aligned variable-size structure follow the [FILE_RENAME_INFO contract](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info) and [native rename flags](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information). No delete-first or copy fallback is permitted. Open readers must retain their old complete revision; new opens must see the new complete revision. Unsupported filesystems fail honestly. Native CI, crash durability and full ancestor/staging pathname-race qualification remain separate evidence boundaries.

## D030 — One budget for the complete query

The §14/18 implementation establishes one monotonic deadline on the documented finite routes after argument validation and before root discovery; remaining route/platform qualification is tracked in PCTX68/AX20. Ordinary queries default to10s; indexing, strict build and checkpoints default to120s. Strict find/query/outline are queries: refresh and lookup share their ordinary outer budget. An explicit positive checked --timeout-ms replaces the entire budget, never adds a new per-phase wait. Child execution retains its separate §40 profile/execution-timeout contract. The2s body-search target is measured performance, not an execution cutoff.

Pass the same deadline through discovery, locks/SQLite, enumeration, read/hash/parse, refresh, selection, source revalidation and serialization. Safe collected matches may return partial exit3 with reason timeout and unknown omitted count; no usable result returns TIMEOUT/exit7. Independent file caps remain distinct. Preparation timeout cannot publish an incomplete replacement generation or checkpoint, infer deletions, or bypass mandatory build validation. Broker refresh_pending remains truthful. Cooperative phase checks do not guarantee cancellation of blocking filesystem/output calls; no abandoned mutating worker may publish after timeout. The source implements these phase contracts on documented routes; incomplete route/platform and cooperative I/O qualification prevent a complete deadline guarantee.

Session and Context CLI operations now share the ordinary outer10s budget, including explicit overrides before discovery. Public library entry points establish10s only when no outer budget exists. Session mutation, receipt emission and acknowledgement check expiry immediately before commit; selection/delta/source revalidation checks consume the same budget. Already expired admission and blocked-writer regressions assert unchanged ledger cardinalities and epoch. A cooperative native commit cannot be interrupted reliably; expiry after a completed commit is not a rollback guarantee. Full finite Work/control/adapter coverage remains unfinished.

## D031 — Finite Git queries do not authorize conversion helpers

Disabling hooks, fsmonitor and pagers alone does not prevent Git status from invoking configured clean/process conversion helpers during tracked-file comparison ([index comparison](https://github.com/git/git/blob/master/read-cache.c), [conversion implementation](https://github.com/git/git/blob/master/convert.c)). The finite query utility therefore observes the exact effective filter configuration with a bounded read-only config query under the same deadline, then declines status with CAPABILITY_UNAVAILABLE when any filter driver is defined. Missing config matches are distinct from invalid config or failed collection. All status entry paths use this guard. Disabling conversions and claiming accurate dirty state would change Git's comparison semantics.

This preflight narrows execution admission; it is not an atomic configuration snapshot against concurrent edits. Native conversion-trigger/no-marker fixtures and stronger configuration isolation/race protection remain separate qualification requirements (PCTX69/AX21). The finite-query Windows resume path owns a contained job through capture and cancellation without a shared-resource lease; it does not assert guardian ACK or replace the registered-check admission contract.

## D032 — Defer body-query source admission until its requested scope

Text/all indexed search may load policy-eligible rows under the pinned ready generation and shared deadline without physically authorizing every historical row first. Search still validates every eligible in-scope/language source, reads and hashes current bytes before evaluation, then ranks/truncates. No metadata expression may eliminate a body candidate. Generic snapshots retain physical validation. This removes one filesystem authorization pass, not duplicate body reads, and requires fresh measurement rather than a claimed speedup. Historical row counts are suppressed; verified_body_files counts successful physical reads only. Missing new files without refresh, current-body versus stale indexed-name distinctions and cooperative I/O remain explicit.

## D033 — Delivered output and failed measurement are distinct

Saved output views, handoff show and existing savings routes are finite10s queries; loading, hashing, selection, filtering, rendering and delivery preparation retain the same deadline. Operation errors observed after expiry become TIMEOUT. Delivery accounting reuses the opened Project and its original budget rather than reopening a potentially different workspace. An actual successful write cannot be undone by later accounting failure: preserve delivered child/retrieval outcome and emit bounded, path-free OUTPUT_MEASUREMENT_UNRECORDED with measurement_recorded unknown. Failed writes never acquire delivery claims; missing measurement is not zero usage or invented savings. Actual macOS CLI regressions prove exact newline-inclusive accounting and complete pipe delivery followed by expired accounting with unchanged ledger. Hard blocking-I/O and other-platform qualification remain separate proof obligations.

## D034 — Measure the emitted context representation

Required adaptive selection (§12/43) prepares verified full excerpts with original ranges, parser-declared multiline signatures, deterministic outline metadata and references. Unsupported signature boundaries fall back explicitly; source text cannot become instructions. Mandatory rules/task constraints are preserved. Applicable decisions are conservatively retained in full until a structured blocking-decision classifier exists. Optional items advance through finite tiers or omission in deterministic priority order. Every three-pass accounting attempt must converge on actual rendered UTF8 bytes including newline; an unstable or oversized attempt reduces optional selection and retries. Fingerprints exclude their own hash and used-byte counter while binding selected representations, source/policy/parser/recipe and format. Envelope timestamps have fixed millisecond RFC3339 width so measurement and CLI reconstruction have identical size. JSON terminal escaping and Markdown fences/metadata are measured by the shared renderer. Concrete tokenizer engines remain optional; unsupported exact token budgets fail honestly.

The eleventh receipt baseline checked escaped final JSON before inserting an emission/event. Historical serializer minimal-context-v2 binds delivered representation and masked-body descriptors; existing v1 records are retained but cannot authorize a v2 delta. This changes semantic baseline compatibility without rewriting the schema or automatically acknowledging stdout. Fresh integrated tests/native gates remain required. D037 and D038 subsequently integrate the shared Build/ContextGet selector. Complete span prioritization and registered optional-engine interfaces remain separate follow-up work.

## D035 — Native Windows handle-relative publication

Tenth actual Windows CI rejects four initial create-only publications at the Win32 rename boundary with error87, after staging/write/sync succeed. This locates the failing call; it does not identify the specific rejected parameter, and current Win32 documentation claims relative RootDirectory is allowed. The correction uses [NtSetInformationFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntsetinformationfile) with the [native rename information contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information), class65 and the same pinned directory, DELETE-capable synchronous staging handle, aligned initialized buffer and bounded simple UTF16 leaf. Create-only flags0 preserve conflicts; replacement flags1|2 preserve open readers. No pathname fallback, delete-first or retry is introduced. Every nonzero native status fails with numeric NTSTATUS/mapped Windows code. Native Windows atomic publication and keeper integration must pass before this correction is qualified; crash and general pathname races remain separate limits.


## D036 — Windows readers cannot override OS sharing constraints

Exact eleventh native CI proves create-only publication/conflicts, Unicode parent names, ordinary open-reader replacement and failed-publication cleanup. The remaining test incorrectly demanded replacement while an external handle explicitly omitted FILE_SHARE_DELETE; native rename returns STATUS_SHARING_VIOLATION (0xc0000043)/Win32=32. Microsoft's [CreateFile sharing contract](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew) keeps that handle's sharing mode in effect until closure, with deletion access governing rename as well. Rust's [primary implementation discussion](https://github.com/rust-lang/rust/issues/123985) explicitly describes this opt-out from POSIX rename. Together with actual native evidence, this corrects our previously overbroad test assumption; it does not authorize bypassing external OS controls (§17 and goal invariant5).

Keep the positive repeated-Unicode/open-reader test with explicit delete sharing. Add a separate denied-sharing regression that requires the exact numeric failure, unchanged complete old data through both the original handle and named opens, no staging leftovers, and successful new explicit publication only after the incompatible handle closes. No production fallback, hidden retry, target deletion or permission override is introduced. Both native branches and keeper still need the next scoped execution; the eleventh failing observation remains retained. Cache restore/save is split so a failed test still preserves reusable build dependencies without caching test results or claiming a pass.

## D037 — Adaptive selection measures the delivery consumer

PCTX12/PCTX44 share a selection engine rather than sizing a Build document as a proxy for another command. context::select_with_measurement owns the existing deterministic priority/tier algorithm, mandatory preservation, policy checks, source revalidation and original deadline. Its application consumer supplies the complete rendered-byte measurement, including envelope, escaping and newline. Measurement is repeated while representations change and must be free of receipt publication, acknowledgement and external execution; a measurement failure propagates, and expiry after measurement overrides a late success/error. The Build wrapper now supplies its actual frozen-width JSON/Markdown envelope through this interface. This is an integrated extraction of the required shared engine, not completed Session/ContextGet integration.

Before connecting ContextGet, read and validate its baseline without holding a writer transaction across index refresh. Measure the actual full/delta packet for each proposed selection, then recheck session epoch and baseline authorization in the receipt transaction. Scope restrictions must constrain optional candidate expansion and preserve applicable mandatory rules. Persist the selected semantic representations only after final capacity/source/task checks; omitted or unacknowledged bodies cannot be silently treated as received. Changed selector semantics need a serializer version boundary that preserves historical records and refuses incompatible delta baselines. No automatic ack or duplicate execution may be introduced.

## D038 — Baseline-aware adaptive session delivery

ContextGet uses D037's scope-restricted adaptive engine, measuring its actual full/delta packet for every tier. Directory scopes resolve to allowed file seeds; lexical expansion cannot escape the requested/task scope. Mandatory task metadata and applicable rules/decisions survive optional downgrade/omission. Selection/index refresh execute outside the receipt writer transaction; immediately before publication the transaction revalidates epoch/status, exact baseline/ack compatibility, source hashes and task revisions, then final rendered capacity and deadline. Cooperative syscall/commit limits from D030 still apply.

Serializer adaptive-context-v3 is a semantic boundary, not a database schema migration. Historical v1/v2 records survive and require a new acknowledged full packet before v3 deltas. The ledger contains selected masked-body hashes/representation, original source/ranges, parser metadata and an omission plan, never source/task plaintext. Unselected files are absent from received evidence. Delta delivery upgrades an acknowledged signature/reference by sending its full body; merely returning an earlier full packet cannot authorize skipping that body. Present-but-unselected prior evidence receives an invalidation tombstone, not a deletion claim. Repeated identical emission/ack cannot grow event ledgers; reuse preserves original ack provenance.

Native CLI fixtures distinguish a tight signature packet, unacknowledged refusal, acknowledged unchanged delta, later full-body upgrade, directory scope, mandatory policy preservation, omitted-source addition, changed-source invalidation, retained v1/v2 rejection and metadata-only storage. Controlled source replacement during delivery measurement verifies stale spans are rejected before a selection result. Whole PCTX44/AC76/AC77 remain implementing until complete ranking/error-span/decision-memory invalidation and platform qualifications are proven.

## D039 — Decision lifecycle and non-Git source invalidation

§30/43 and AC77 require memory/decision changes to invalidate context without a Git change, and retired claims cannot become current instructions. Accepted declarations without an effective replacement retain conservative mandatory delivery. Proposed declarations are references; superseded/deprecated/rejected/cancelled/completed records are history references with source ID/hash, scope, status, declared/effective replacement links and explicit document-only validity. Proposal/rejection links cannot retire an accepted claim; terminal replacement declarations remain lineage evidence while their link records exist, preventing automatic revival when their status becomes cancelled/completed. No document claim grants permission or proves implementation behavior. Persistent lineage across link-record removal/restoration remains unfinished.

The common selector cannot deliver retired/proposed bodies as current guidance even through explicit/lexical seeds. Inapplicable decision records are excluded with document_scope, preserving logical scope. A previous current decision becoming a history reference receives decision_no_longer_current invalidation in addition to the changed reference. Retrospective instruction changes and physical deletion have independent delta/tombstone semantics.

Serializer adaptive-context-v4 and selector adaptive-v3 preserve older records but refuse incompatible receipt baselines. The current document result fingerprint binds source_versions and the receipt plan. Final selection reloads/revalidates the scoped document snapshot because supersession/applicability may depend on a document omitted by budget; selected-path checks alone would miss that race. A controlled omitted-proposal→accepted mutation invalidates an unchanged selected predecessor before output. Cooperative native I/O and the final validation-to-output window remain the existing observation limits.

Isolated actual Git CLI fixtures keep HEAD and porcelain state unchanged while decision/instruction bytes, status and replacement links change. Fixtures verify original-hash-preserving supersession invalidation, cancellation without predecessor revival while links remain, metadata-only storage, history-only seeded delivery, physical deletion and retained v3 rejection. Full topic/role filtering, handoff/task/approval-memory variants, link-removal recovery and all platforms remain required; AC77 is not promoted to complete.

## D040 — Durable workspace decision lineage and explicit restoration

§30/43 and AC77 require retired claims to stay historical across context reconstruction. D039's live source graph alone lost retirement when a replacement file disappeared. Store validated source observations in the existing append-only control event ledger, keyed by project/workspace/subject; do not create a second memory engine or cache source bodies. Source verification precedes observation publication. Observation is distinct from delivery/acknowledgement and can survive a later rendering failure. Existing control backup/restore preserves observations without restoring execution or receipt authority.

The minimal explicit restoration contract is accepted front matter `reinstates: ID` or an ID array. Each ID must be observed for this predecessor in this workspace; syntactic and unknown-history failures roll back this load's additions. An extant accepted replacement blocks restoration. Deleted or non-accepted replacements can be explicitly restored, retaining observed provenance. Policy-hidden history stays conservatively historical, including explicit reinstatement; filtered absence is not deletion evidence. Identifiers explicitly declared in an allowed source remain its claims, while hidden observation identities are withheld. Reader deadline/configuration errors propagate; corruption, unsupported observation versions and bounded-history overflow cannot silently produce current guidance. Serializer5/selector4 bind the added validity metadata; existing receipt versions remain preserved but require a fresh full acknowledged baseline.

This contract qualifies decision declarations only. Topic/paused-role delivery restrictions, handoff/task/approval memory variants, all ranking/error-span contracts and other-platform proof remain required. No observation or restoration grants permission or establishes actual implementation behavior.

Independent read-only review found the initial restoration path could treat a policy-hidden accepted replacement as absent. Integration requires a currently visible, policy-bound observation before restoration and adds direct/CLI regressions for the combined exclusion/reinstatement case. Aggregate serialized history is capped at8MiB in addition to per-subject/row limits. Final verification must bind these reviewed sources; earlier results remain intermediate.

## D041 — Planned context delivery audience barriers

Read-only AC77/§32/43 investigation found durable role/topic controls already exist in operations.rs and are enforced for claims/schedules, but ContextGet drops the registered session role and supplies implementer; Build has no recipient/topic binding. This is a required implementation gap, not qualified behavior.

Resolve session ownership/workspace/registered role before inventory/ranking. Reuse a shared read-only operations helper for recipient normalization, wildcard/role pauses and exact topic silences. ContextGet needs explicit topic binding; Build needs an authenticated consumer binding (session preferred) plus topic. A caller role hint cannot override durable registered barriers. Relevant silence with an unspecified topic must require a binding rather than infer a topic from source text. Suppress the whole packet with a bounded path/body-free error when necessary, preserving mandatory-rule semantics instead of silently dropping constraints. Bind barrier state into receipt identity and recheck before commit under the original deadline; resume requires fresh full context and cannot restore acknowledgements.

This first contract does not finish per-item topic attribution, handoff/task/approval memory variants or owner policy exception priority. Their explicit attribution and policy-conflict contracts remain required. CLI proof must cover ownership/impersonation, alias/wildcard matching, registered-role pause, other recipient/topic isolation, no source/receipt leakage on suppression, mid-delivery barrier changes and resume without ack inheritance. No delivery code or runtime evidence is claimed by this plan.

D040 defensive follow-up implemented: SQL guards TEXT type and UTF8 BLOB byte length before transferring each row, so invalid oversized rows do not reach Rust payload materialization. Streaming retains per-subject/aggregate checks and invalid-request rollback. This is not a hard native SQLite/RSS claim. Final-source documents12 PASS/static PASS; full native316 PASS/2 startup failures remain unresolved (lineage-payload-verification.json).

## D042 — Query process group setup and native startup investigation

Use the standard Unix process_group(0) setup in place of the sole setpgid pre_exec callback, preserving PGID=PID and existing root/group cancellation. [Rust1.99 CommandExt](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.process_group) documents equivalence; its [Unix source](https://doc.rust-lang.org/src/std/sys/process/unix/unix.rs.html) shows callbacks disable native spawn eligibility. PCTX resolves the executable to an absolute path beforehand. No latency or platform guarantee follows from this API change.

The identical8-worker×16-query fixture retains each original1000ms deadline, group-existence probe and separate stdout/stderr. It FAILED before the change, PASSED alone after, then FAILED in the related suite alongside original path/stream tests. Read-only live sampling reproduced four failures and captured three failed children still at _dyld_start+0, before script output, with minimal native footprint. This disproves treating the API change as the startup repair. The underlying native cause remains unknown; no deadline is extended and no retry proves repair. Evidence query-startup-verification.json binds sources and all results.

D041 integration now resolves consumer ownership/workspace/registered role before source loading and adds explicit Build/ContextGet topic binding. Build's role hint cannot override a registered consumer; unbound agent builds require a session. Local owner inspection is still subject to wildcard/selected-role controls. The shared helper reads bounded relevant rows and fingerprints durable relevant-role policy events without returning labels/reasons. Parent review aligned the helper's registered-role set with the existing all-registered-role pause check, preventing alternate-role pause/resume from resurrecting an acknowledged baseline. Initial/final checks use the original deadline; ContextGet rechecks inside its receipt transaction. Serializer6/selector5 bind the new metadata. Declared packet topics are not per-item source classification, and the outstanding D041 contracts listed above remain mandatory.

## D043 — Consumer-bound source packs

Extend D041 shared delivery binding to Pack plan/create in every representation. Private schema2 plans bind actual registered session/epoch and declared-topic barrier. Selected Pack passes that same consumer into Build. Old schema1 plans require replanning; public artifact schema1/pack-v1 remains compatible. New artifacts expose only an opaque barrier, with no consumer-session/topic fields or authority restoration. Complete serialization includes its bytes.

Recheck delivery before source work, after source validation and immediately before Unix pinned publication under one original10s deadline, including library entry with no existing deadline. No writer transaction is held over source reads or filesystem publication. The final check is cooperative and does not eliminate the interval before the native publication syscall. Independent review identified the initially missing post-source check; integration added it. A concurrent final-validation policy-change fixture and Windows publication remain unqualified.

Actual CLI testing exposed the shared global/local --output argument propagation: Pack created the artifact then attempted a second envelope write. Route Pack Create responses to stdout while keeping its documented --output artifact flag. A unique argument-ID attempt failed Clap's duplicate-long-option assertion and is retained as intermediate evidence.

This slice addresses PCTX46/AC78–80 consumer binding; graph is PCTX45. It does not complete per-item topic attribution, owner exception conflicts, handoff/task/approval variants or whole pack acceptance.

D043 source-validation follow-up preserves TIMEOUT/exit7 when inventory or planned-source readers exhaust the original budget. Other access/content failures remain stale. A private synchronous observer is a no-op in production; native unit fixtures use it after inventory to coordinate a real SQLite policy writer before source reconstruction and final delivery validation. They prove committed whole-role and declared-topic changes are denied, then public Create creates no output parent. A separate controlled original-deadline expiry proves library classification after inventory admission. This does not prove an actual agent epoch race or transactional publication. Independent review identified unit fixture crate-path and temporary-root canonicalization issues, both corrected.

## D044 — Planned source topic attribution and owner reporting exceptions

Original §§32/43/44 require restrictions before ranking. Packet --topic is only a request declaration and must not declassify source items. Add bounded authored source-topic metadata bound to source identity/hash/scope, combining source declarations and trusted project-policy assignments conservatively. A source cannot remove a trusted label or grant an exception. Explicit seeds and Pack representations use the same restrictions. Unknown legacy attribution under applicable silences is unresolved delivery policy, not an implicit public label.

Optional suppression must expose no path/title/body/topic/retrieval reference. A mandatory-rule conflict fails the whole packet with a bounded policy error, rather than omitting the rule. Classification/control fingerprints participate in selection, baselines and final revalidation. Backup/restore must preserve durable attribution/exception policy without reviving acknowledgements.

Reporting exceptions require the original policy owner, affected policy/source/recipient scope, reason, expiry and authored priority. Existing ordinary approvals do not override silence. Undefined, forged, expired or wrong-owner conflicts fail closed. An authorized reporting exception does not resume claims, model wake, execution or host permissions. Independent read-only requirements review identified these contracts; no D044 implementation or runtime proof is claimed yet.

Next implementation must settle the bounded authoritative classification format and legacy-source hold behavior, then test mixed public/silenced sources, explicit-seed bypass attempts, mandatory-rule conflicts, post-ack classification changes and narrowly scoped exception priority without authority restoration.

D044 source-classification slice implemented: policy.source_topics is defaultempty with unchanged empty serialization, bounded user/project/overlap union, and explicit public empty labels; Rule/Decision declarations add labels. Unknown legacy sources under relevant silence cannot inherit packet-public classification. Optional candidates are suppressed before cap/tiering with a generic unknown-count omission; mandatory conflicts hold the packet. Seed/scope/graph provenance is opaque. Shared task-scope guarding and pre-body handoff classification prevent metadata/body projection. Independent review identified missing handoff and Pack declaration guards, repaired with actualCLI regressions.

Pinned current project/effective-user policy is compared at source-policy construction and revalidated before emission/publication. Original config presence is remembered; initialized-registry missing config failsclosed. Controlled measurement callbacks change/delete policy after admission and verify no delivery/receipt table. Native final343 PASS/3 original startup FAIL; related59 PASS and format/Clippy PASS, source-topics-verification.json. Serializer7/selector6 fresh acknowledgement required. This does not finish D044 owner exception identity/provenance/expiry/priority, broader memory variants or native platform/transactional publication qualification.

## D045 — Task-file input authority

Extend D044 source admission to Build's explicit task files. Preserve relative-to-CWD CLI semantics. Canonical targets inside the captured project receive shared source-topic/document admission before body access; external aliases into the project cannot bypass it. Reject lexical in-project traversal and symlinks, including aliases to external content. Explicit external input is an independent pinned canonical-parent/file authority, with original alias/ancestor stamps retained privately. Recheck file identity, metadata, content hash and alias resolution before emission. Use the shared anchored reader and original request deadline; no worker or renewed retry budget. Input provenance is path-free kind/hash/replayable metadata, included in exact output accounting. Stdin is one-shot and not replayable.

This does not infer topics for caller-authored raw text/stdin or grant delivery exceptions, receipt acknowledgement or host authority. Native filesystem calls and the final check-to-output interval remain cooperative. Windows alias stamps and task input require native platform qualification. Existing external input is limited to UTF-8 regular files/1 MiB; project input also honors its smaller configured limit.

Independent review caught an ABA file replacement gap: the initial identity pin and shared project reader used different handles. The reader now returns the retained identity of its actual body-read handle; task input compares it to the original pin. A controlled native unit fixture replaces A with B between pinning and reading, restores A afterwards, and rejects both distinct and identical B bytes. No-op production observation points grant no new authority. Re-review found no blocker in this delta; native Windows and atomic final-output guarantees remain unproven.

## D046 — Original-owner restrictions and reporting-only exceptions

§32 requires the original designated owner to release policy and forbids inferred urgency. Add a versioned operations policy schema with immutable restriction IDs/original owner/keys, optimistic revisions, input evidence hashes and durable role-policy events. New trusted local policies capture PCTX_OWNER_PRINCIPAL; legacy records retain unknown ownership until an explicit evidence-bound attestation. The local operator boundary requires owner actor and no agent run credentials, but is not OS authentication or shared-account proof. Existing role pause/resume flags are preserved; wrong-owner overwrites/releases and stale releases fail atomically.

Reporting is a separate typed message projection. The original owner authors affected restriction IDs/revisions, per-conflict binary precedence, actual recipient/role/topic/category/source scopes, complete redacted message hash and finite validity. Every pause/silence conflict needs current explicit precedence; undefined, restriction-first, stale, expired, revoked or mismatched rules hold delivery. Source-reader exclusions and current effective policy remain enforced. Role-pause delivery may be excepted while claims, wake, schedules and execution remain paused. Ordinary approvals/messages and Build/Context/Pack never acquire reporting authority from these rules. Actual Inbox read/ack derives all registered roles/canonical identities and rechecks current rules under the original deadline; report fingerprints are delivery provenance, not receipt comprehension.

Independent review found restore dropped identity protections, source references bypassed exclusions, and initially body-only hashing omitted sender-controlled metadata. Main repaired each: idempotent trigger installation on existing/migrated/restored schemas, active exception invalidation on restore, reader-policy/current-policy checks, and a shared complete normalized/redacted Message fingerprint. Owner report-fingerprint and actual delivery use the same projection. Tests include unknown legacy ownership, wrong-owner/stale/agent-run denial, actual mailbox suppression/revocation, hidden metadata/body-hash mismatch, excluded path hold, real expiry with pause preserved, and restore trigger/owner/history invariants. Source references remain owner-reviewed attribution claims; no source body is fetched or verified by this reporting route. Native platform/automatic memory/transport qualification and the full AC77 contract remain open.

Operations now share a cooperative10s default across initialization, bounded input, transaction and final commit; CLI --timeout-ms is supported. JSON protocol remains64KiB with bounded shared task reader allocation and FIFO-safe admission. Static initial pointer-type failures and CLI fixture/schema-shape failures are retained; tests are not weakened to promote success. No native syscall atomicity or new platform repair claim follows.

## D047 — Reporting source classification and final admission proofs

§32/43/44 require reporting attribution to preserve actual source-topic restrictions. Share a bounded SourceClassifier with ordinary source delivery; combine owner-authored assignments and managed Rule/Decision declarations without granting delivery authority. Reporting conflicts use the union of source labels and packet topic. Explicit current original-owner references may cover a different source topic; a payload hash/source scope cannot declassify unknown sources. Relevant silence with unknown attribution holds delivery. Metadata-only classification fingerprints remain opaque; source bodies and attribution truth are not verified.

Independent review found sequential role/recipient admission could accept an earlier grant after expiry, then found the same boundary across an Inbox batch. Retain minimum actual exception/message expiry and source bindings. Revalidate after role/recipient evaluation and again across retained Inbox reports before a single final time observation removes expired projections. Controlled native helper tests exercise real expiry, classification changes after the last role admission, and an earlier selected report changing/expiring during later selection. These are helper-boundary proofs, alongside actual CLI mailbox/ack tests; they do not establish atomic filesystem-to-output or native Windows/Linux guarantees. Original request deadlines and underlying database transaction authority remain unchanged. Whole AC77 and the full mandatory goal remain incomplete.

## D048 — Phase-aware search candidate admission

The tenth M30 body-search p95 remains above the §18 target. Body/freshness searches previously performed physical pathname authorization before repeating anchored admission for the actual body read. Reuse the initial regular-file handle through the shared reader. Candidate absence is an internal phase result, not a public error-code match: root changes and candidate denial previously shared `POLICY_DENIED`, while missing candidates and unrelated failures shared `IO_ERROR`.

Only initial descendant missing/access/link/non-directory rejection, authored exclusion or a proven nonregular handle can omit a candidate. Root/config/deadline failure and unexpected I/O/resource errors propagate. Once a regular handle is admitted, size, encoding, read, policy, reopened identity and metadata failures remain errors; a replaced instance cannot become a retry against a different file. Existing ordinary reader retry semantics and explicit timeout partial-search accounting remain separate. Metadata-only searches retain their physical authorization path and final root gates.

Independent review found an initial non-Unix portability regression: candidate absence had become an error. Repair with native ErrorKind/reparse/type admission before domain conversion and anchored-root revalidation before omission. Windows still uses its existing pathname traversal, not NT-relative handles; authored portable fixtures have no new native Windows execution proof. Controlled native tests cover missing/excluded/link/FIFO/access/type omission, fatal root/config, size/encoding errors, post-admission deletion/same-byte replacement, normal body/hash and original deadline. This implementation does not establish a performance target or full search/task quality; a new immutable M30 is required.

## PCTX01 sequential execution and pre-effect frontend contract

The user's latest goal requires exactly one active official task. PCTX01 is lowest incomplete and has no prerequisites. Historical D-series slices do not establish whole-task completion. Its completion audit is tasks/PCTX01.md; all other incomplete official tasks are backlog. Preserve AC ownership and required platform/error/budget gates without expanding independent domain features.

Fix actual frontend defects within PCTX01: retain OS-string argv until Clap validation; recognize only actual JSON format options before the child delimiter; reject unsupported NDJSON routes before project loading; reuse pure Find/Structure argument validators before project discovery, strict refresh or output paths. Relative scope traversal/absolute/drive/backslash requests fail as invalid arguments. Existing library entry points use the same validators. Actual CLI snapshot tests modify source after indexing, making a forbidden refresh observable, and require every durable file byte and rejected output path to stay unchanged. Other frontend gates remain open, including no-color/help metadata/common finite routes/full platform matrix. No task completion or next-task selection is asserted.

Drain the pre-existing eleventh M30 job and preserve its results only. Package target-directory repair and approval-memory integration are inactive backlog; no unrelated new implementation/verification is authorized while PCTX01 is active.

## PCTX01 help, color and parser diagnostics

Apply no-color before parsing through the derived Clap command tree, including nested help; JSON-requested help is plain. Non-JSON parser diagnostics cross the shared secret redactor and escape unsafe controls. Preserve OS-string argv and child delimiter handling. Public leaf effects are explicitly registered and reviewed against dispatch, not inferred from command names. Every declared leaf has a coverage guard and every visible help path is exercised by actual CLI before project access. Standard JSON uses envelope schema1.0; native hook output and NDJSON events retain separate transport shapes. Pack Create --output names its artifact, while streaming output-file options are refused.

Native real-terminal fixtures require a positive colored control and one ten-second test deadline for reads/exit; failure kills/reaps the child. This fixture limit does not alter product or original one-second query budgets. Review transport/output and fixture-lifetime findings are resolved. Native full52787 retains377 PASS/3 original startup FAIL; earlier environment/type/reflected-value fixture errors and the first interrupted full gate remain in pctx01-help-color-verification.json. No retry-success promotion, whole-task completion or platform inference.

## PCTX01 immutable Work/quota read budgets

Explicitly classify ordinary task/agent/check reads and task completion dry-run plus quota Report/Plan/Reconcile. CLI discovery starts one10s default or positive override; library read execute entries establish one default only when none is supplied. Existing Deadline objects are retained. Writes, registered child execution and continuous watch lifetimes keep separate contracts. Expose shared Project SQL phase helpers: recompute remaining busy waits (round sub-ms values upward), interrupt long SQL and normalize actual expiration without creating a new clock. Check transactions, row collection, parsing, aggregation and final read results. No domain gate or quota authority rule is loosened.

Work check plan delegates to runner profile validation. Related auxiliary workspace opening must inherit p.deadline; otherwise descendant policy/source/executable reads lose the budget. Add phase checks without introducing child execution. Native tests prove frontend/opening contention and Work BEGIN IMMEDIATE contention separately. Quota exclusive contention is mainly database opening; this is not large-aggregation proof. Filesystem/parser/commit calls remain cooperative; initialization completed before later expiry is not retrospectively rolled back or falsely reported as successful query output.

The attempted auxiliary Git-delay fixture failed because registered opening performs no native Git query. Preserve that initial failure log; replace its invalid premise with a positive finite linked-workspace plan/no-launch regression. This does not lower or close the auxiliary expiry condition: phase-expiry during auxiliary discovery/fingerprinting remains explicitly unverified, as does CPU/large-row expiry. Review's earlier observation wording was corrected after actual execution. Full24156 terminal101 retains382 PASS/4 original native startup FAIL with unchanged one-second budgets; no retry success or task completion is asserted.

## PCTX01 SQL engine expiry and request isolation

Native recursive SQL verifies actual engine work (at least1000 VM steps) before TIMEOUT/exit7 under one immutable50ms request budget. A second regression found that a connection retained a prior expired progress callback when a distinct caller had no deadline, causing DB_ERROR for normal SQL. configure_sqlite now removes that callback only in the deadline-free caller scope. Its initial deadline check still refuses an expired Some(deadline); no old request is renewed. Initial29470 terminal1014 PASS/1 FAIL is retained; fixed45355 terminal0/direct5 PASS. Full14793 terminal101384 PASS/4 original startup FAIL, unchanged original budgets. Independent scoped review found no blocker. This is engine interruption and callback isolation proof, not every command's CPU/aggregation/auxiliary phase proof.

## PCTX01 auxiliary source budget propagation proof

Registered workspace opening is inert, so native Git-delay injection was invalid (retained earlier failure). Instead exercise the private auxiliary factory in an isolated subprocess with bounded inert linked metadata and a private registry/config. Read before the original1s deadline, then require subsequent changed source to be refused TIMEOUT/exit7 without database creation. Do not mutate process-global environment shared with other tests. Parent waits at most10s and kills/reaps a timed-out probe. Actual CLI linked-Git-worktree positive planning remains a separate integration fixture.

A controlled mutation replaces inherited open_with_deadline with original Project::open. The new regression fails because changed source is returned after expiry; a finally block restores the exact source. This detects the original propagation defect and is not a product failure retry. Targeted84851 passes, mutation88616 expected test exit101 (0 PASS/1 FAIL), restored-source full35833 terminal101386 PASS/3 original startup FAIL. Prior query-program-resolution failures remain unresolved despite this run passing that variant. Independent review found no blocker in source propagation proof. Expiry during discovery/hash, large command aggregation and required platforms remain unqualified.

## PCTX01 adapter finite query scope

Only Doctor/Verify/ProtocolFixture receive a shared finite read scope, default10s only when no deadline exists. CLI starts it before discovery; direct callers retain supplied deadlines. Final root/deadline checks normalize late errors. Configuration/protocol input is bounded regular-file data, checked around open/read/parse; Unix finite leaf opens use O_NONBLOCK as well as O_NOFOLLOW. Filesystem/parsing are cooperative and do not claim syscall preemption or Windows race proof.

Doctor admits exact claude --version through the existing query supervisor, retaining original deadline, bounded128KiB combined capture and shared process-group cleanup. Captured PATH/caller CWD availability preflight distinguishes explicit absence from lifecycle/capture errors; never catch all SOURCE_UNAVAILABLE as installed:false. Nonzero/empty/invalid-UTF8 responses fail SOURCE_UNAVAILABLE6; overflow remains PARTIAL_RESULT3. Explicit minimal environment preserves isolated HOME and runtime PATH; no model calls/account verification are inferred from a synthetic version.

Verify must not acquire a new adapter lock or create configuration. Existing mutation acquire is deadline-aware only for callers already supplying a deadline, using try-lock and remaining-budget polling; deadline-free mutation semantics stay blocking. This does not classify CLI install as a finite query. Native fixtures prove Verify succeeds while existing lock is held and Install returns original-budget TIMEOUT without installation.

Initial positive protocol fixture used macOS /var alias, which pinned no-symlink opening correctly rejected. Canonicalize test base only, retain initial failure. Final full83611:395 PASS/3 original startup FAIL; targeted19 and overflow1 PASS, static/format PASS, independent review no scoped blocker. No whole PCTX01, live adapter or other-platform qualification; no Actions dispatch.

## PCTX01 command-level aggregation expiry proof

SQL opening/engine interruption is insufficient to prove command-level aggregation. Public quota Report uses10000 valid observations tied to real registered Task/Agent/Session. Setup validates every record, preserving public import limits. Positive20000 tokens/10000 records/zero duplicate exclusions proves the fixture. Test-only thread-local RAII observers distinguish admitted rows, entry before existing loop guard and real work after it. After100 worked rows, wait at101 entry for the unchanged supplied2s deadline; require TIMEOUT7 and no later entries/work. Inspect observation/event snapshots through a separate unbudgeted caller without renewing the expired request.

Controlled aggregate-guard removal still yields outer TIMEOUT but performs10000 iterations; regression fails. Billion-row SQL tests retain50ms and receive emergency2s SQLite interruption solely for test safety; its activation explicitly fails. Handler-loss mutation proves this cannot create a false PASS. Driver finally restores exact source bytes for both mutations. These expected failures are sensitivity evidence, not product retries.

Final other_windows assembly adds per-row original-deadline checks, preserving references/order/data. Restore existing report lint annotation after observer insertion. Full14811:395 PASS/4 original startup FAIL; scoped/static gates pass and independent review finds no remaining blocker. No broader quota feature, live usage, performance target, other-platform or whole-task qualification; no Actions dispatch.

## PCTX01 auxiliary loading and fingerprint input budgets

Original source-after-open proof is insufficient for actual loading/fingerprinting. Separate isolated exact-selectable tests use fresh deadline-free fixtures, then original supplied2s budgets. A load observer reaches actual ConfigAdmitted before expiring that same budget, requiring no RegistryRead/ResultReady and unchanged registry/no DB. Fingerprint observer admits a real64KiB chunk then expires before update, requiring read1/hash0 and no later work/artifacts; positive full-byte SHA256 equality remains. This proves a fingerprint input boundary, not preemptive SHA execution. Each parent child is killed/reaped after10s; no process-global environment mutation in concurrent tests.

Use existing shared anchored deadline reader for config, external executable/guardian and inert Git metadata. Scripts retain source policy reader. Streaming executable hashing preserves SHA and128MiB cap including growth, pins identity and final metadata; Git metadata stays regular4096-byte bounded with Unix O_NONBLOCK admission. None execution retains its own lifetime. Remove unreferenced deadline-free anchored wrapper rather than suppress dead-code lint. Actual linked-Git-worktree CLI refuses both FIFO metadata files exit9 before execution/host resources.

Controlled inheritance loss, post-read guard loss and replacement with unbounded FIFO reader each fail their specific tests; finally restores exact sources. Initial combined warning and test-observer nested-if Clippy failure retained; final style-only repair suppresses no lint. Final full42750:399 PASS/3 original startup FAIL, scoped/static gates pass and independent review no blocker. All filesystem calls remain cooperative, additional races/native Linux/Windows and other finite routes remain required. No whole-task completion or Actions dispatch.

## PCTX01 finite runner/trust read scope and honest resource status

Specification §8 sets general queries to10s; §31 describes plan/status as reads. Direct library calls establish one absent default and retain supplied instants, while CLI starts that same scope before discovery. Execution, trust writes, cancellation and guardians retain their separate lifetimes. Read-only host/helper path admission must not create directories. A status metadata error cannot be translated through exists() into absence: validate existing host ancestors as directories and skip slots only on explicit NotFound, preserving all other errors after the deadline check. Native identity timeout is an error, never proof of dead ownership/release.

Pinned regular bounded metadata uses the shared anchored reader; native/syscall checks remain cooperative. TLS test observers separate actual identity observation from local admission so removing the guard fails even when an outer check still reports TIMEOUT. Trust observes each actual hash chunk with one original/default instant. Isolated FIFO probe owns kill/reap cleanup; controlled guard/scope/reader losses demonstrate sensitivity and exact restoration. Initial direct-shell positive fixture was invalid under existing policy; use the permitted private executable without weakening classification. Independent resource-error finding repaired and covered by final source. Full29877:406 PASS/3 original startup FAIL, static/format PASS. Evidence pctx01-runner-read-verification.json; no whole-task/native other-platform qualification or Actions dispatch.

## PCTX01 inert filter request clocks and shared stdin

General filter queries and explicit previews use one absent default10s or supplied request instant. Fixture Test/Activate are writes; observed output/child renderingNone receives no default query clock. Keep the independent1s processing cap, with original request checks before phase work. Reuse input::read_stdin serialized1MiB Unix/Windows byte transport/RAII cleanup rather than another blocking reader/worker; filter alone maps byte-size and UTF8 domain errors. Task document transport behavior remains unchanged.

Read binding/report paths must not create directories or flatten filesystem errors into empty selection. Shared hash domain uses an explicit error constructor at size checks; never match generic POLICY_DENIED because anchored symlink refusal shares that code. Real post-read executable replacement proves denial5 while sparseoversize proves limit3. Original250ms admission hooks distinguish real records/input from work, and controlled local guard removal fails despite outer TIMEOUT. Actual held partial stdin expires before EOF; owned3s parent cleanup catches unbounded transport regression. Full56147:414 PASS/3 retained original startup FAIL, static/format PASS, scoped review no blocker. Initial test-only enum compilation and absent target invocation retained. See pctx01-filter-deadline-verification.json; cooperative calls/native other-platform and whole-task gates remain required; no Actions dispatch.

## PCTX01 finite schedule observation and read-only control state

Specification §8 general queries require one10s default or the supplied request instant; execution ownership remains separate under §40. Schedule List/Plan/Inspect use existing read-only control state rather than creating/migrating schedule tables. Preserve existing runtime256MiB and native128MiB caps through shared bounded hash input, and bound/pin owned manifests. Native observation admits only exact current-user managed-label print/is-active queries and shares the original supervisor clock. Keep capture/timeout/capability failures explicit; only actual definition/config invalidation yields current_bindingfalse. Numeric/hash receipts exclude raw output and durable artifacts. No native install/start/remove action is performed by these fixtures.

PCTX01 is the only active official task (`implementing`). Final native full61099 terminal101:425 PASS/1 retained original startup FAIL; schedule unit2 and related34 PASS; format/locked all-target Clippy PASS. Evidence: pctx01-schedule-deadline-verification.json. Schedule List/Plan/Inspect use one absent default10s or supplied instant from pre-discovery through SQLite rows, parsed state, current project reopening, runtime/native hashes, owned manifest reads and final assembly. Mutation/execution routes gain no default query clock. Existing control state opens read-only without Work/schedule initialization or migration; missing List is empty and absent Plan/Inspect retain explicit unavailable errors. Runtime hash preserves256MiB and native128MiB; bounded pinned regular256KiB manifests reject FIFO without blocking. Exact native observations use shared supervised current-user/managed-label query whitelist, cleared minimal environment and numeric/hash receipts with no raw captured output or durable artifact. Nonzero gives registration_unknown; expiry propagates TIMEOUT7 instead of absence or stale binding. Public1000-row positive/default one instant followed by original250ms admission expiry requires1 admitted/0 worked/unchanged events. Synthetic native query completes before controlled original2s expiry after observation, then refuses local admission with1 observed/0 worked/no DB/output. Two local guard losses each produce actual0PASS1FAIL and exact restoration before final checks. Actual CLI zero budget refuses before discovery; positive List/Plan/fixture Install/Inspect retain state/no query outputs, and original100ms exclusive DB contention returns TIMEOUT7/null data with unchanged installation/events. Opening contention is not row-phase proof. Initial related14PASS3FAIL used macOS /var alias in old direct fixture; canonicalized test base only, retaining policy and initial log. First full38971 yielded424PASS2FAIL: original startup plus backup fixture alias. After canonicalization, work-control66549 yielded10PASS1FAIL because backup setup relied on List schema initialization; explicit Add now prepares the schema and3074 gives11PASS. Both failures retained. Independent scoped review found no blocker. Filesystem/serializer/SQLite opening remain cooperative; SQLite pathname ABA binding and live OS service/native other-platform evidence are unqualified. Historical original startup failures remain unresolved even when a particular latest run passes them; this result does not promote retries or establish their root cause. Inventory, full frontend matrix and original startup root cause remain required. No next official task or Actions.

## PCTX01 static inventory clocks and partial-error boundary

Specification §8 requires one general-query10s default or explicit original request clock. Read-only inventory Scan/Profile/Audit previously missed common routing; direct public APIs now retain the same scope. Reuse existing reader/walker and exact limits, and add cooperative checks to parsing, masking, claims, aggregation and document evidence. An expired request or invalid root/loaded policy is a request failure, never successful partial coverage. Incidental source absence or unsupported parsing remains explicit per-item data. Do not interpret loaded-policy validation as proof of concurrent on-disk policy stability.

PCTX01 remains the only active official task (`implementing`). Final native full98832 terminal101:435 PASS/1 retained original startup FAIL; inventory unit4 and related14 PASS; format/locked all-target Clippy PASS. Evidence: pctx01-inventory-deadline-verification.json. Inventory Scan/Profile/Audit and direct application methods establish one absent default10s or retain the supplied instant; nested scan/profile reads keep the same scope. CLI preflight reuses exact existing Scan file/byte bounds before discovery. Shared reader handles walking/regular source input; static parsing, fallible recursive masking, source observation, metadata, workspace/dependency/role loops, section/document comparison and final assembly check the original budget. Catch-to-problems/unconfirmed/review validates original deadline, anchored root and loaded policy validity before admitting incidental missing/unsupported/denied items. TIMEOUT/config/workspace failures propagate; no silent .ok fallback for audit source reread. This checks the loaded policy, not a qualification of concurrent on-disk policy changes. No DB/schema/output/script effects or new inventory features. Actual128-package positive/default work phases share one10s instant; actual256 confirmed profile claims and256 audit references pass. Each separate original1s request completes16 real work iterations, expires at17th admission, and requires17 admitted/16 worked/TIMEOUT7/same instant/unchanged source manifest/no state or EXECUTED marker. Direct and dispatch expiry, bounds/default empty, missing/unsupported claims and FIFO refusal2 pass; Unix FIFO parent owns3s cleanup. Actual CLI all three zero budgets and invalid Scan bounds refuse before missing root/data effects; positive finite reads preserve conflicting/unconfirmed claims, baseline mismatch9, complete envelopes, every file byte and directory, and never execute scripts. Three processing guard losses and original-scope loss each produce actual0PASS1FAIL; partial-error helper loss is separately detected by direct fallback guard test. The initial partial mutation script mistakenly removed test observers and failed compilation without tests; narrowed repair gives actual0PASS1FAIL. Both initial driver failure and repaired evidence retained. Saved source is restored before final static/full checks. Independent scoped review found no blocker. Calls/parser/sort/filesystem remain cooperative, not preemptive or atomic snapshot proof; additional phase races/native Linux/Windows and full frontend matrix remain. Historical original startup failures remain unresolved even when a particular latest run passes them. No next official task or Actions.


## PCTX01 pure grammar and capacity precedence

Specification §14 and §12 require invalid arguments before effects and byte capacity below the minimum error envelope to return input error2. Reuse producer validators rather than duplicate grammar in the CLI. Capacity admission precedes command semantic validation; direct producer budget errors retain exit8. Source-independent path/line/selector/representation/bounds checks move before refresh/input, while actual source upper lines, permissions and artifact-derived locations remain producer checks. Preserve accepted mixed Extract inputs, handoff-only Build and Read end clamping.

PCTX01 remains the only active official task (`implementing`). Shared pure argument validation now runs for Read/Outline/Extract/Build/Graph before project discovery, stdin input, index refresh and response-file writes, and again at library producer entry. Existing path policy, source-dependent upper line/symbol/artifact/permission checks and valid mixed Extract or handoff-only Build requests remain intact. Read positional/named path aliases conflict explicitly. Output capacity validation still runs first: the initial full65959 exposed five exit2-to-exit8 regressions (438 PASS/6 FAIL including the retained startup failure); moving pure validation after the existing capacity guard repairs that precedence without changing tests or budgets. Initial targeted58 and final related72 PASS; final controlled CLI/producer guard losses each yield actual0PASS1FAIL and sources restore byte-exactly; format and locked all-target Clippy PASS. Final native full5705 terminal101:443 PASS/1 retained original startup FAIL. Evidence: pctx01-argument-preflight-verification.json. Independent scoped review found no blocker; routing audit found no clearly omitted ordinary finite read leaf, but does not prove every downstream phase. Schedule Inspect help now correctly describes finite native observation without a durable capture. Full argument/representation/error/exit and native Linux/Windows gates remain open, including preflight error representation consistency and original native startup root cause. No next official task or Actions.


## PCTX01 refusal transport and failed stdout delivery

Specification §14 guarantees one JSON document for JSON errors and input errors before effects; it does not mandate Markdown for rejected arguments. Preserve existing JSON pre-effect refusal transport and document it, rather than changing consumers to Markdown. Use the shared terminal-safe serializer. Failed stdout delivery is I/O exit7 in parser, semantic, timeout and capacity paths; successful invalid-capacity refusal remains2. A message shortened to fit may not change the underlying error code/exit.

PCTX01 remains the only active official task (`implementing`). Pre-effect argument errors retain one JSON document across compact/json/markdown; parser stderr and NDJSON stderr exceptions remain explicit. Shared final rendering now applies to parser JSON, semantic/watch/deadline and capacity rejections, preserving no project/input/response-file effects and capacity-first exit2. Independent review found the tiny-capacity path ignored stdout failure; all four actual closed-pipe CLI refusal classes now return7 without panic. Initial related36/final37 PASS, controlled ignored-write guard loss actual0PASS1FAIL/test101 and exact restoration, format/locked all-target Clippy PASS. Initial full80733 was intentionally stopped130 after review; remaining runner child33822 was confirmed terminated before final checks. Pre-bidi full2682 terminal101:446 PASS/1 original startup FAIL. Actual no-color U+202E leak confirmed; shared plain-diagnostic/literal-source control escaping repairs it. Bidi-related38 PASS and helper-call loss actual0PASS1FAIL/exact restore; final static PASS. Final native full11657 terminal101:447 PASS/1 retained original startup FAIL; no whole-task completion claim. Evidence: pctx01-refusal-rendering-verification.json. Message shortening preserves classification for over-capacity detail, but no current static request was identified that reaches that branch. Full frontend invalid/duplicate/global-position/representation/error/exit matrix, original native startup root cause and native Linux/Windows remain open. No next official task or Actions.


## PCTX01 handoff argument and explicit input admission

Specification §14 requires invalid options before effects. Reuse producer name grammar for every leaf, consistently rejecting empty names. Handoff source files reuse the bounded explicit-file reader with &Path to retain native filenames and exact existing cap/deadline; do not reinterpret dash as stdin or add Create/Update timeout support. Source/encoding failures precede checkpoint creation. macOS EILSEQ is a platform fixture limitation, not proof of Linux opaque filename success. Broader §13 schema and write rollback are PCTX11 feature gates.

PCTX01 remains the only active official task (`implementing`). Handoff Create/Update/Show now share existing nonempty ASCII name grammar before CLI discovery and producer effects. Existing bounded explicit-file input handles Create/Update under the original request clock and1MiB cap before checkpoint; &Path, cwd-relative files, literal dash and regular-file symlinks remain supported, with no new stdin mode or CLI timeout route. Actual invalid names leave missing root/data/output untouched; sparse oversize/invalidUTF8 and FIFO refusal preserve initialized state/checkpoints. Direct invalid-name/expired-original-clock checks and valid create/update/show/collision-preserved handoff body pass. Initial target-selection101 ran no tests; first related47PASS1FAIL was macOS APFS refusing nonUTF8 fixture creation EILSEQ, retained. Repaired related54PASS: macOS actual opaque argv is IO_ERROR7/no effects; native Linux successful opaque filename remains required. Three controlled losses each actual0PASS1FAIL/exact restore. Initial Clippy101 fixture type/constant style fixed without behavior weakening; final format/locked all-target Clippy PASS. Final native full39587 terminal101:452 PASS/1 retained original startup FAIL. Evidence: pctx01-handoff-input-verification.json. Independent scoped review no blocker. Filesystem calls remain cooperative; Windows nonregular-open behavior and broader PCTX11 handoff schema/rollback/recovery are unqualified. Full PCTX01 frontend/platform/startup gates remain open; no next official task or Actions.


## PCTX01 startup isolation does not change execution policy

A raw Rust Command driver reproduces first-path script expiry without PCTX supervision or process-group setup. This narrows the investigation but does not identify a production defect or justify dispatching a script through an interpreter. Executable path, argv, trust, signal and group semantics must remain unchanged. Concurrent policy scan timing is correlation while OS logs redact executable paths. A copied system executable is a distinct trust/location case, and its signal/cleanup failures cannot be treated as ordinary startup-only timing. Preserve every failure and the original1s contract. The small descendant-free harness's blocking root reap is diagnostic only; never reuse it as production cleanup.

PCTX01 remains the only active official task (`implementing`). Read-only native startup isolation at unchanged base fc1849d18df11e9f21f1dad2bf9d550ac580f03d reproduces original1s expiry outside the PCTX supervisor: raw direct scripts fail with both empty and minimal PATH environments; reversed-order group controls show sub-millisecond spawn return followed by delayed first execution, and no-group-first scripts also produce two expiries. System /bin/echo passes128 attempts; a private copy fails all8 (6 expiry,2 early signal-status failures; signal number not recorded), so it is not an equivalent trust/location control. Correlated privacy-redacted syspolicyd scans are not bound to exact child paths/PIDs and do not prove causation. Passing later attempts are not repairs. Tiny inert drivers stop each worker after failure and root wait after kill can exceed1s; no production cleanup qualification. Evidence: pctx01-startup-isolation-verification.json. No production/test/budget changes, security-policy changes, interpreter substitution, full-gate rerun or Actions. At that evidence boundary the exact-source full was452PASS1FAIL; frontend and native Linux/Windows gates remained open. No next official task selected.


## PCTX01 representation refusal precedes all dispatch

Specification §14 limits Markdown to build/outline/read/handoff and rejects unsupported options before effects. A guard only inside execute could still write a rejected response file and could be bypassed by watch/follow. Reuse the existing support registry in common preflight after minimum capacity; rejected formats use the established JSON refusal transport and grant no output-file authority. Keep the execute guard for defense. A failed plain parser diagnostic is I/O7, while delivered invalid arguments remain2, preserving the existing mask/control boundary.

PCTX01 remains the only active official task (`implementing`). Common Markdown admission reuses the exact supported registry after capacity checks and before semantic/project/stream/response-file dispatch; execute retains defense. Eleven valid unsupported routes with/without output require JSON INVALID_ARGUMENT2 and unchanged missing-root/data/output state, including watch/follow and run argv. Initial relative-output attempt returned7; the absolute fixture proves rejected init wrote its response file. Both are retained. Removing only common preflight yields actual0PASS1FAIL and exact source restoration. Actual plain parser closed stderr initially exits101; checked writes repair it toIO7 while normal delivered diagnostics stay2, with masking unchanged. Related37PASS; format and locked all-target Clippy PASS. First full71673 stops at261PASS3FAIL; final no-fail-fast full38079 is terminal101: 453PASS/2FAIL, retaining original native startup failures. Evidence: pctx01-representation-admission-verification.json. Independent review no scoped blocker. Unix pipe evidence does not qualify Windows; full frontend/phase/platform/startup gates remain open. No next official task or Actions.


## PCTX01 singular global inputs across command depth

Specification §14 defines scalar common options and rejects invalid/unsupported inputs. Preserve existing same-level duplicate refusal consistently across a selected command path rather than silently allowing child values to override ancestors. Pinned Clap4.6.7 uses separate child matchers and later selects a child global value, so post-propagation indices cannot recover the overwritten occurrence. Fresh shared typed value-parser guards count only CommandLine inputs and preserve the original value types/metadata. Repeatable producer options and delimiter-owned child argv are excluded.

Pack/Create's explicit required output was checked before global propagation, rejecting a single pre-group output. Defer only that builder check: the derived nonoptional field checks presence after propagation, with no default/sentinel, and help explicitly states the required one path at any command depth. This repairs common CLI binding, not independent Pack semantics. A guarded command is single-parse state; construct a fresh guard for every request.

PCTX01 remains the only active official task (`implementing`). Singular root/format/output/timeout-ms/no-color options now reject same/cross-depth duplicates through a fresh shared command-line-only typed parser, preserving defaults, native paths, typed validation/possible-values metadata, repeated scopes and literal child argv. Ten actual duplicate cases refuse2 before missing-root/data/output effects; three global positions bind identical root/workspace and preserve accepted10000ms/zero-refusal semantics. Four root version/flag-order cases return exact compiled plain version/no effects. Pack/Create alone defers builder output requiredness until global propagation; nonoptional PathBuf presence still required and help explicit. Actual Pack CLI publishes artifacts at all3 depths with response stdout, rejects missing/duplicate output. Initial global0PASS2FAIL includes checkpoint-list DB_ERROR before index preparation; global fixture now prepares index and independent checkpoint issue is PCTX14 backlog, not fixed/qualified here. Initial parser2PASS1FAIL retains pre-group Pack output refusal; repaired3PASS. Related56PASS and added actual Pack1PASS; controlled guard loss actual0PASS1FAIL/exact restore. Format/locked all-target Clippy PASS. Final native full82923 terminal101: 459PASS/3 retained original startup FAIL. Evidence: pctx01-global-argument-verification.json. Independent scoped review no blocker. Fresh command required per parse; full frontend/phase/platform/startup gates remain open. No next official task or Actions.


## PCTX01 primary delivery versus post-delivery accounting warnings

Specification §14 separates results from diagnostics and fixes storage/execution error7. Help/version are successful only when their native text can be printed. Native hook and stream objects retain their own protocol shape while sharing terminal-safe reversible JSON escaping. A primary delivery failure returns7 rather than panic or absent-success. Compact stream fields must keep row boundaries even when diagnostic escaping preserves newline/tab for multiline prose.

An accounting warning follows completed delivery; suppressing a failed diagnostic write must preserve the established outcome and unknown measurement, never panic, infer recording or repeat execution. This does not qualify hook first-import failure timing or all native transports/platforms.

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

A producer-owned spawn observer distinguishes prelaunch refusal from unknown
post-spawn outcome. Error codes alone cannot attest whether a child started.
Complete processing permits child-code propagation; fatal processing errors and
partial capture preserve separate PCTX status/exit while retaining child truth.
Already classified outer project errors keep their exact typed exit/message.
Publication failure does not establish readable retained output or trigger retry.
Basis: §40/AC63 common frontend contract; [proof](evidence/pctx01-run-exit-verification.json).


## PCTX01 Run refusal and capacity truth

Run minimum error capacity must measure its known prelaunch fields; an artifact
ID is not the authority for execution-state truth. Existing failure classification
takes precedence over a presentation-size issue when shortening an error.
Admitted oversized results retain the presentation BUDGET_TOO_SMALL contract.
Only parsed Run intent receives Run prelaunch metadata.
Basis: §8/40/AC63; [boundary proof](evidence/pctx01-run-budget-verification.json).


## PCTX01 isolated delivery worker completion versus timeout

Observed native macOS normal-ended WNOWAIT worker group signalling returned
EPERM (see pctx01-delivery-ownership-completion-observation.log). This is not evidence
that descendants are absent. Do not require or interpret a zombie-only group
signal as cleanup qualification. Normal fixture bodies await their CLI children;
only timed-out live groups use the cancellation proof. A reserved leader identity
is retained through nested absence observation before final reap; error paths
use RAII best-effort cancellation/reaping without claiming successful verification.
FIFO O_RDWR distinguishes real nested cancellation from EOF caused by root death.
The root-only guard-loss test must fail and clean its still-owned group. Product
process supervision, native startup budgets and all original failure criteria
remain unchanged. Separate runner groups and other platforms stay unqualified.


## PCTX01 shared budget reduction boundary

Extract the existing CLI oversized-envelope reduction into render::budget_fallback
without changing admission or byte measurement. A pure boundary permits inserting
oversized presentation metadata into real native observations, then checking that
reduction preserves independently recorded child and typed-refusal truth. Caller
measurement and serialization remain necessary. The injected metadata is a test
condition and cannot be reported as naturally occurring frontend fallback or a
universal size bound. Native observer errors require a separate ownership audit:
manual run_cli's attestation callback always returns Ok; registered execution
uses fallible callbacks. Do not invent a production environment hook solely to
exercise an unreachable manual callback failure. Keep PCTX34/40 independent
execution features inactive while qualifying the required PCTX01 frontend status.


## PCTX01 Outline capability coverage propagation

Specification §4 distinguishes unsupported structural analysis from successful
text reads/searches; §8 defines complete/partial/unsupported coverage; §14 maps
necessary unavailable capabilities to exit6. Aggregate existing Outline file
coverage in the common frontend. When source coverage is complete, nonempty all-unsupported files return error,
coverage unsupported and CAPABILITY_UNAVAILABLE6 while preserving file metadata.
Mixed supported/unsupported requests return partial3 with an unsupported reason.
Supported empty structures stay ok0; syntax-partial stays partial3. No analyzer
or generic other-command status rule is added. Actual JSON/compact/Markdown and
text no-match fixtures qualify this boundary.

Manual run_cli's attestation callback always returns Ok. Registered callbacks
are fallible; runner::spawned sets in-memory pid/process_group before durable
publication can fail, and caller branches use job.pid to preserve started truth.
This is a source audit, not native fault injection or general cleanup evidence.
No additional runner feature or product fault environment hook is introduced.

Independent review found that visible all-unsupported files do not establish a
complete source universe after strict refresh skips supported sources. The native
oversized-Python plus unsupported-Ruby regression reproduced erroneous6 versus3.
Source incomplete coverage now takes precedence: retain partial3, skipped-source
reasons and per-file unsupported metadata. Final combined regression includes
JSON/compact/Markdown, and controlled loss of this precedence fails again.

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

## Serial-task rule reconciliation — 2026-10-08

The latest explicit nine-point user instruction overrides the older goal clause
allowing a ready-task switch after local work is exhausted. An external blocker
now requires exact evidence and required action while retaining the same official
task. Only an identified mandatory unfinished prerequisite permits a temporary
switch to that single prerequisite and return. Fixed PCTX01 gates and residual
counts are unchanged; historical next-action lists do not activate other tasks.

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


## PCTX01 R10: preserve input-dependent Event keys — 2026-10-08

Specification §14/§38 common admission must reuse existing producer contracts.
The frozen R10 registry called every explicit adapter Event key pure, but
`import` parses the event and returns PreToolUse protection before key use.
Therefore only agent and hook/file transport grammar are unconditional. Event
key validation remains after parsing/protection and before receipt lock/DB/replay.
Native tests prove invalid unused keys retain protection without execution or
permission, while PermissionDenied empty key refuses before state effects and
valid same-key replay is stable. Denominator12 unchanged. Existing label256
UTF-8-byte/control/secret grammar and whitespace acceptance, POLICY_DENIED9,
original expiry and default-clock scope remain unchanged. No business feature
was added. Latest serial instruction remains authoritative over older objective
text: external blockers do not authorize starting another official task.

Source/spec reconciliation: adapter's existing error helper maps POLICY_DENIED
to9, whereas specification §14 requires policy exit5. R10's positive test records
the current behavior, not specification conformance. This is a mandatory current
PCTX01-G05 residual; do not mark stable-error completion or move to another task.
Resolve the shared producer error classification with a complete error/transport
matrix after the pending integration, preserving the recorded initial behavior.


## PCTX01 R11: retain conditional schedule admission — 2026-10-08

Specification §14/§38 admission must preserve existing producer predicates.
RunLoop namespace is validated only by tick after eligible schedule lookup, so
adding unconditional label admission would reject accepted no-tick requests.
Purpose is checked only for keep-awake, and its predicate permits embedded
controls; Recover reasons likewise only require trim-nonempty/2048-byte bound.
Inspect/Uninstall keys and occurrence/revision/attempt are state lookups without
new lexical restrictions. RFC3339 parsing is pure; future Tick rejection depends
on current time and stays producer-side. Native no-tick/future/replay and pure
exception tests plus controlled narrowing losses preserve these distinctions.
The frozen residual denominator12 is unchanged; local12/12 does not close whole
platform or official PCTX01 gates. Latest serial rule prohibits switching official
tasks for external blockers. No schedule feature, native registration or clock
was added.


## PCTX01: contextual adapter exit classes — 2026-10-08

Read §14 with §27/§38, which explicitly extend conflict9 and gate10. Adapter's
blanket non-input9 incorrectly classified policy/path/capability/input cap/schema
and changed original bytes. Restore semantic classes, retaining actual state9.
Same CONFIG_CONFLICT/PLAN_MISMATCH names have different causes, so classify at
call site where necessary rather than globally normalizing every code. Input
128KiB/settings cap is2, not minimum-context budget8. Persisted DB JSON syntax
errors cannot inherit user serde input2; generic DB_CORRUPT7 and checked shapes
prevent malformed success/panic without exposing stored content. Incomplete
intent still requires reconciliation9; no rollback claim.

Source audit also resolves frozen C004 classification: provided plan/expect hash
length/hex/equality is pure; comparison with stored bytes/identity is stateful.
Existing Install acquires a lock before the pure predicate. Record this required
G03 residual under the already existing leaf C004; fixed public denominator140,
historical residual label groups12 and official gate denominator10 unchanged.
Local12/12 is not all-leaf completion. Next implementation reuses existing
predicate without new business functionality or actor/state bypass.


## PCTX01: preserve Install hash grammar before effects — 2026-10-08

C004 reuses exactly the existing pure predicate in shared admission and retained
producer defense. Equality is byte-exact and ASCII hex accepts upper/mixed case.
Case normalization would alter the request contract; file-content digest matching
remains stateful and may still reject an admitted uppercase digest. No new label
or secret-detection grammar. Direct original-expiry priority stays ahead of the
pure predicate. The existing lock-deadline test used invalid hash input, so it now
uses valid64 hex to test actual contention with the same50ms request. Invalid
requests get input2 before authority/discovery/lock; valid requests retain owner5.
Fixed denominators10/140/12 and latest serial-task rule remain unchanged.


## PCTX01: refusal coverage is execution coverage — 2026-10-09

§8 defines complete coverage by completed supported request execution. Parser/
admission/render failure therefore cannot inherit the default successful
envelope's complete coverage. Share error-envelope construction with partial/
code reason, keeping status error and original typed exit. Do not globally change
partial coverage into exit3, discard prelaunch/minimum metadata, reissue clocks
or alter streamed native hook/watch records. Existing state/delivery predicates
stay producer-owned. Unknown-option qualification is frozen140 leaves, not full
required/typed argument acceptance; public denominators10/140/12 unchanged.


Existing-leaf next-action audit: C001 Activity has an already implemented
nonnegative cursor guard in work::activity_inner and watch::run, but CLI preflight
has no Activity arm before discovery. This is a missing G03 admission for the
existing leaf, not a new feature or label group. Record G03-C001 with source
evidence; preserve finite Work original-expiry handling and separate streaming
behavior. Frozen140/12/10 denominators remain unchanged.


## PCTX01: Activity stream admission placement — 2026-10-09

The same pure cursor grammar needs two frontend placements. Normal finite
JSON/compact uses common preflight. NDJSON/follow uses stream admission after
existing output/route refusal and before discovery, preserving stderr error
records. JSON-follow refusal remains ahead of cursor; Markdown unsupported
refusal remains ahead of all semantics. Direct Work original expiry first stays;
Watch checks a supplied expiry before cursor/stdout but gains no clock. Do not
collapse these branches into a universal JSON preflight or new watch timeout.
Existing C001/fixed140/12/10 denominators unchanged.


## PCTX01: revalidate native environment availability — 2026-10-09

A read-only Docker inventory and ephemeral offline capability probe show an
available Linux arm64 engine/image with git/cc but no Rust. Replace the blanket
Linux-unavailable assumption with Rust setup pending. Environment execution is
not native PCTX qualification, nor Linux x86_64/Windows proof. Override the cached
runner's registration entrypoint and mount no host credentials/socket. Continue
G09 under PCTX01 using isolated toolchain/build storage and one heavy job.
Independent source/spec audit also identifies existing G05/G06 persisted Work
event JSON syntax classified as input2; contextual corruption7 is required with
accepted JSON types/cursor/clock unchanged. No new feature/denominator.


## PCTX01: storage-origin errors and busy retryability — 2026-10-09

Specification §9 explicitly requires INDEX_BUSY with retryable state after the
existing5s DB wait cap; §14 classifies DB_CORRUPT as storage exit7 and invalid
caller JSON as input2. Error::new now marks only INDEX_BUSY retryable; this is
metadata, not automatic retry, renewed budget or longer lock wait. Activity
maps only persisted event JSON decoding failure to contextual DB_CORRUPT7 while
accepting every valid JSON Value. Existing Markdown admission and NDJSON stderr
error frame (code inside data) retain priority/shape. A streamed valid earlier
page remains delivered when a later page fails; no stdout rollback or skipped
corrupt event is claimed. Whole/frozen denominators remain10/140/12.

Persistent Linux schedule INDEX_BUSY is not an omitted general query clock:
Tick is mutation/occurrence execution and keeps deadline None by default, with
existing5s SQLite wait cap. Source review finds execution_binding/exact_plan/
runtime_identity hashes inside IMMEDIATE claim/final transactions (§35/38,
PCTX36); causal lock-duration proof remains missing. Do not widen the wait or
add a query clock to force this test green. Independent scheduling repair stays
backlog while PCTX01 owns its error envelope/classification.

## PCTX01 stored Task and receipt JSON classification (2026-10-09)

Specification §14 distinguishes caller argument errors2 from storage errors7;
§27 retains idempotency, revision and lease conflicts9. Work's five DB-origin
conversions used the generic caller serde classifier. A local generic contextual
decoder changes only those conversion failures to DB_CORRUPT7, with fixed safe
messages. It preserves typed TaskDefinition decoding, nullable optional Value
fields, arbitrary valid receipt JSON, exact replay before lease checks and
request-hash conflict before response decoding. This is existing PCTX01-G05
coverage, not new Task/Report semantics or a gate-denominator change. Injection
connections are closed before prepared-state snapshots to avoid treating their
WAL lifecycle as product effects. Cold first-use rollback and new receipt shape
validation are not established by these fixtures.

## One-time PCTX01 closure freeze (2026-10-09)

The updated user objective replaces incremental similar-path exploration with a
single finite whole-G01–G10 register:40 stable detail IDs, bounded local closure
23/40, whole0/10. Existing10 gates/140 leaves/172 help paths/historical12 R groups
are unchanged; first whole-detail denominator40 decomposes mandatory common
contracts rather than adding features. The 140-route response/error/clock crosswalk
and91 decode-site source inventory are review inputs, not91 defects. Unknown
origins are resolved together in G05-D04, not by extending the implementation
queue repeatedly. Independent producer contracts keep their task owners.

§23 requires macOS arm64/x86_64, Linux x86_64 and Windows x86_64. Linux aarch64
is follow-up evidence only. Original macOS startup failures remain common product
contract failures; no unsupported environmental exemption. PCTX36 concurrent Tick
failure is retained separately because occurrence claiming is independent from
common INDEX_BUSY envelope metadata and Tick has no general query clock; no whole
suite pass or causal hash-phase claim. Repeated full platform/mutation gates on
every small patch are superseded by focused/static checks and justified grouped
integration candidates. No current-slice Linux full rerun or Actions was started.

## PCTX01 persisted-error proof reconciliation (2026-10-09)

Fixed G05-D04 owns stored-error classification; it does not acquire producer
acceptance scope. The two-plan-check rule led to one all48-site reconciliation
before further changes. Runner check-plan had a reproduced common error-truth
defect: DB_CORRUPT7 became successful owner_binding_required. Propagate errors7;
retain missing5/stale9 blocked plan behavior. Registry defaults already accept
{} and [] (missing-workspace6); null is incompatible7. Preserve that grammar.
Compare logical SQL/schema and non-DB files for SQLite writer/checkpoint paths;
do not claim physical DB/WAL/SHM immutability. Existing slot acknowledgement
publishes intent before decoder failure; retain intent rather than manufacture
rollback. Source and runtime residuals are in pctx01-storage-residual-
reconciliation.json; no new fixed acceptance condition or platform exemption.

## PCTX01 G05-D04 local closure (2026-10-09)

Final six frozen Schedule storage boundaries have bounded runtime proof. Close
only the local common decoder/error condition after independent all48/43 audit;
release-platform qualification stays in G09, actual callback faults in G05-D02,
and independent schedule/recovery correctness with its producer owner. The
fixed40 denominator remains; detail count23 to24. No Schedule algorithm change,
OS registration, stricter accepted JSON schema or discarded initial evidence.

## Control Backup artifact destination (2026-10-09)

Specification §14 response contracts and §33/PCTX26 explicit backup require the
archive and command response to remain distinct. Actual CLI evidence at baseline
a0e89b9 showed a valid archive published before main attempted to write a response
to the same create-only destination, yielding false exit9. The shared response
router now treats Control Backup like existing Pack Create: output selects the
artifact, success/error responses use stdout. Archive format, producer checks and
create-only collision refusal remain unchanged. This is PCTX01-G04-D03 common
delivery ownership, not independent PCTX26 implementation or completion. Evidence:
[evidence/pctx01-caller-schema-verification.json](evidence/pctx01-caller-schema-verification.json).

## Grammar versus operational execution capacity (2026-10-09)

PCTX01-G03-D03 uses existing source-independent argument grammar; valid parsed
requests may still fail operational response capacity8. Run metadata requires
3000bytes, Unix registered Check evidence8192bytes, and local Helper evidence
8192bytes. Helper owner/task checks precede its floor; cloud/native queued intent
bypasses that local execution floor. Moving these to frontend argument2 would
change policy/state/platform precedence. Keep original guards and clocks, record
these boundaries in G04-D03/G05-D06 and prove no child/resource launch. The common
minimum complete error-envelope floor remains argument2. Source§14/§40 and actual
[capacity proof](evidence/pctx01-grammar-crosswalk-verification.json) qualify this
classification; endpoint/state combinations remain open there.

## PCTX01 cooperative user cancellation (2026-10-09)

Specification §14 requires CANCELLED130 while §38 requires observed native child
truth and owned resource cleanup. Native baseline SIGINT during held stdin ended
with signal2 and zero stdout bytes. Install a CLI-owned SIGINT/Windows CTRL_C_EVENT
atomic callback; library use and private guardians do not install handlers. The
callback only latches a flag. Existing immutable deadline/project boundaries and
execution/watch loops observe it. No separate process supervisor or new task is
introduced. Narrow thread-local finalization guards finish durable spawn ownership,
reap/capture publication, proven no-child cleanup and response delivery. They
suppress cancellation only, not original expiry or actual storage errors.

Cancellation produces wrapper130 independently of native SIGKILL status. Artifact
termination cancelled maps to cancelled check evidence before native-signal failed
classification. Unknown/living groups retain slots; direct-root reaping alone is
not resource-release proof. CheckRun no-child release precedes fallible DB recording.
SIGTERM/CTRL_BREAK/console closure are not converted into this Ctrl-C contract.
Windows implementation follows [Microsoft's console handler API](https://learn.microsoft.com/en-us/windows/console/setconsolectrlhandler),
but native console delivery still belongs to required G09 Windows validation.
Cancellation is cooperative, including disk I/O and SQLite busy waits.

The fixed G05-D06 remains open for its full exit matrix, cancellation/publication
races and Helper/guardian paths. Existing G05-D02 owns broader callback/storage
fault truth; G08 owns phase/race/deadline completion. Counts27/40 and0/10 remain.
Evidence: [cancellation verification](evidence/pctx01-cancellation-verification.json).

## G05-D06 shared exit qualification and unpublished evidence (2026-10-09)

Use one fixed category-to-authored-control matrix for exits0/2/3/4/5/6/7/8/9/10/130
and the explicit Run child exception. Existing source-dependent representative
controls and shared facade branches qualify classification; this does not acquire
every independent producer's acceptance scope. Exit4 now proves STALE_INDEX/error
and unchanged source/config, not merely process4. Current90 focused native tests
and independent review qualify local G05-D06; G05-D02/G08/G09 remain open.

A registered cancelled child with failed artifact publication previously lost its
observed native result because the evidence facade reread the absent artifact.
Known spawned/raw-unavailable execution now records a non-gating supervision error
and returns the original execution truth. Publication/processing failure7 wins
over wrapper cancellation130, while termination cancelled and native signal remain
independent data; the original command is never repeated. Broader postspawn callback
or evidence-DB failures still belong to G05-D02. No acceptance criterion is lowered.

Actual Helper metering probes the caller workspace rather than the provider artifact
workspace. Its explicit unknown-metering diagnostic preserves delivered exit130;
provider-workspace reread proves artifact/native truth without rerun. This independent
measurement routing defect is inactive PCTX47 backlog, not new PCTX01 producer scope.

The fixed register summary was stale25 while authoritative item states and linked
closed conditions proved27. Derive closed_details from item states, then add this
one closure to28/40; correct its execution cursor to existing open G05-D02. Frozen
IDs/denominator/help/leaves/Rgroups are unchanged. Evidence:
[exit matrix](evidence/pctx01-exit-matrix-verification.json).

## PCTX01-G05-D02 callback truth — 2026-10-09

A registered spawn-observer error happens after child admission. Killing and
waiting then returning only the callback error loses an actually observed native
outcome and captured output. Route that error through the existing common capture,
reap and artifact finalization path; suppress further callbacks, keep the processing
error independent from native exit/signal, and never run the command again. Actual
artifact publication refusal can replace the processing error while retaining child
truth and raw-unavailable status. Native exit23 observed without consuming wait status
must remain exit23 rather than be fabricated as SIGKILL.

This bounded repair does not close G05-D02. The read-only review's seven remaining
phase groups are substructure of its original postspawn contract, not new requirements
or IDs. Pipe/wait/join and receipt/runner completion faults still need implementation
and fault proof. Denominator40/local closed28/whole gates0 are unchanged. Initial
callback failure reproduction remains in evidence; focused passes do not qualify
Windows, full platform integration or retained startup/Schedule Tick failures.

## PCTX01-G05-D02 grouped capture fault truth — 2026-10-09

Native wait errors cannot certify root absence or exit0: retain available capture,
unknown termination/null exit/signal and a child_unknown durable receipt. Failed
observation invalidates the PID ownership assumption; do not signal it afterward.
Observe wait state again after callback errors before cancellation. Configure each
pipe independently; close a failed setup pipe rather than start a blocking worker.
Join both workers and retain the surviving capture even when the other panics.
Missing/lost streams are incomplete, not successful empty streams. Always enforce
the existing250ms capture finalization bound, including continuously successful
reads; an inherited pipe's continuous writer cannot extend finalization forever.

The finite continuously-readable fixture reproduces the original failure before
the repair. Native seven-scenario controls qualify the three existing pipe/wait/join
phase groups; no slot-release or full descendant-absence claim is made. Remaining
receipt/Runner phases stay open. After two updates without closing the entire
condition, responsibility/completion were reconciled: §14/38 common child/error
response boundary remains PCTX01, independent producer semantics stay inactive,
40 fixed conditions/28closed are unchanged, and no mandatory bar is lowered.

## PCTX01-G05-D02 job receipt durability — 2026-10-09

Output-job active/final atomic writes are mandatory observation receipts, not best
effort success signals. Active publication failure becomes monitored processing
failure; final native receipt is attempted before compact/artifact publication,
so its error is included in the artifact and blocks passing reread evidence.
Response reports both receipt phases and errors independently from artifact
availability, native status and resource ownership. Preserving captured output
never invents durable job state or grants gate evidence. No stored schema change.

Real directory-over-file obstruction reproduces the ignored error, then verifies
both failure phases and a successful passing control. Byte-exact retained prior
receipt assertions distinguish last-known metadata from failed new publication.
The related native regression also exposed an existing empty PID-file fixture
race; emit newline-delimited PID and wait for the complete record in both sibling
fixtures within their unchanged5s limit. Retain99PASS1FAIL alongside the repaired
Runner evidence; original four1s-budget product startup failures are unchanged.
Seven existing phase groups/four qualified/three Runner groups remaining; no new
condition IDs, independent producer expansion or whole-condition closure.

The final receipt control also reproduces the candidate's cancellation-precedence
mistake: final receipt failure retained CANCELLED130 instead of storage7. Repair
tracks observed cancellation independently from primary processing error, so
receipt failure wins while native cancelled termination/SIGKILL remain. This is
the existing G05-D06 processing/publication precedence applied to G05-D02, not a
new condition or reopened exit category. Candidate failure remains in evidence.

## PCTX01-G05-D02 Runner finalization and local closure — 2026-10-09

Preserve an observed execution when guardian, slot-release publication, evidence
recording or Helper receipt fails. Shared finalization precedes DB/Helper persistence,
so recording failure cannot bypass cleanup. Guardian or ownership uncertainty keeps
slots unconfirmed; partially removed slots plus failed final journal publication
produce resources_released null, not false certainty. Failed check publication is
unverified/not-published and cannot grant gate evidence. Helper in-memory state and
receipt publication are separate. Processing failures still override cancellation.

Native controls use actual SQL contention, atomic-publication obstruction, completed
provider exit23, externally reaped guardian ECHILD and surviving artifact reread.
Backend Result::Err is explicitly an API-boundary model with a real completed process,
not an induced backend failure. Initial fixture CLI/selector mistakes are retained and
corrected without relaxed assertions. All seven original phase groups represented;
103 focused/static/review proof closes bounded local G05-D02, increasing28→29 of40.
Platforms/general race/independent producer acceptance remain separately open; no
whole PCTX01 completion or new ID/denominator. Next existing condition is G04-D03.
