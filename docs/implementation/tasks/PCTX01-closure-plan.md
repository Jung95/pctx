# PCTX01 one-time closure plan

## Repair review at original conditions — current, 2026-10-09

PCTX01-G05-D05 remains product_failure; fixed33/40 details,0/10 whole gates.
No causal PCTX patch is supported yet: failed-path validation63us/spawn628us
versus98.88ms late policy result. Existing static evidence shows scan queue
already configured USER_INTERACTIVE QoS; caller-priority promotion lacks a
matching cause. Admission serialization cannot renew the original deadline.
Interpreter/prewarm/reuse/security exceptions do not preserve conditions.
Next: identify the expensive scan operation and an applicable platform/product
correction. No test rerun, service collection or OS/security change performed.
[Candidate decisions and full verification requirements](../evidence/pctx01-startup-preserved-condition-review.md).

## Policy queue attribution — earlier, 2026-10-09

PCTX01-G05-D05 remains `product_failure`, fixed33/40 details,0/10 whole gates.
Static analysis confirms a serial policy scan queue; the16-path historical
join shows1.154068s accumulated scans, expired child last. Current original
concurrency diagnostic fails101:113 outcomes/112OUTPUT/1TIMEOUT35071;
15 attempts censored, no drops, all known114 PIDs absent, source unchanged.
Approved3-second/16-hash log read executed once:16 events, all8 owned paths
joined. Serial scans span1.092233s. Failed35071 starts scanning913.425ms after
birth;185.029ms scan result is98.881–98.883ms after its original deadline.
All7 successes have results before deadline. Independent review confirms the
join/arithmetic; raw logs discarded, query reaped/absent, no admin prompt.
Direct evidence supports serial policy scan backlog consuming startup budget.
No product repair or closure. Next: identify an applicable correction preserving
original acceptance conditions; exact queue arrival/kernel reply/scan cost
cause remain unknown. This finite scope is consumed; no unchanged rerun.
[Evidence and limits](../pctx01-startup-triage.md).

## Required platform and failure ownership

Specification §23 explicitly requires macOS arm64/x86_64, Linux x86_64 and
Windows x86_64. Current local macOS arm64 original-budget startup failures are
product/contract failures with unproven cause, not external-environment excuses.
macOS x86_64/Linux x86_64/Windows x86_64 need authorized real hosts or runners;
Actions remains constrained by the owner's monthly3000-minute instruction. No
Actions was dispatched. Linux aarch64 supporting full582PASS1FAIL belongs to its
fixed earlier source, not current new Task fixtures or required x86_64.

The concurrent Schedule Tick failure remains inactive **PCTX36**, with exact test
and log in pctx01-persisted-errors-verification.json. Tick has no general finite
query default and this occurrence-claim failure is distinct from the verified
INDEX_BUSY7/retryable envelope contract. It does not by itself require independent
schedule algorithm repair in PCTX01. It prevents reporting a passing whole suite;
final integration must demonstrate the common envelope/exit truth for the failure.
If evidence establishes a common-contract defect, repair that part in PCTX01;
otherwise retain the unresolved PCTX36 acceptance failure explicitly. Hashing
inside writer transactions is confirmed structure, not a proven cause.

## Frozen execution rule

1. G10-D02/D03 closed: independent review and current-document reconciliation. Review corrected the G05-D02 callback overclaim, which remains open without adding an ID.
2. Close the grouped parser/admission/stored-error/classification candidate,
   reusing already qualified contracts; no path-by-path discovery chain.
3. Close the remaining response/empty/control matrix.
4. Close remaining original-clock phase and native startup conditions.
5. Run necessary focused/static and one justified candidate whole gate, then
   required real platform matrix. Preserve all failures and exact source identity.
6. Close G10-D05 and PCTX01 only when every condition is proved; choose the next
   official task's finite list and continue the same Goal.

New findings go to the existing matching item or producer backlog; add a new ID
only with existing mandatory-spec omission and PCTX01 responsibility proved,
recording old/new denominator and reason. If two successive plan checks grow the
list or close nothing, stop implementation expansion and reconcile ownership,
conditions and blockers. Do not reopen closed conditions without a relevant
change or new defect. Small changes receive focused/static checks; full platform
reruns belong to grouped integration candidates. Controlled-loss experiments
are optional when detection strength is unclear, not a per-change requirement.

## Prior G05-D04 bounded evidence

G05-D04 local common decoder condition is closed. Frozen48 persisted sites and
43 preserved source dispositions reconciled; no new ID/denominator. Final
persisted22 and private Schedule2 tests pass, format/Clippy pass. Independent
closure review accepted this bounded scope. Prior42 sites and safe errors,
replay/identity/expiry and truthful effects remain linked; no universal rollback,
post-spawn callback, scheduling algorithm or release-platform claim. Evidence:
pctx01-storage-schedule-verification.json and residual-reconciliation.json.
At that closure detail24/40, whole0/10; superseded current count below.

## Prior G02-D04 bounded evidence

Real140-leaf parser1855 cases:582 accepted parser-only,1273 refused; native10184
combinations PASS, saved replay/related14 PASS, format/Clippy PASS. Seven authored
conflicts and fourteen accepted alternatives qualified; missing/value/numeric/
enum/empty/flag/raw cases and native globals context covered. Entire isolated
fixture/output snapshots preserved and stdin held open. Test-only changes; prior
failed generator/harness attempts retained. Independent scoped review accepted.
Evidence: pctx01-parser-verification.json and parser-cases.json. Detail25/40,
whole0/10, denominator unchanged. Next fixed G03-D03/D04 + G05-D06; required
platform and original startup failures retained. No next official task.

## Current G03-D04 input-cap candidate (condition open)

Shared regular-file cap+1 reader repairs growth after metadata for Work JSON/
Restore, Quota and Schedule inputs. Prior caps/error2/owner5/state9 and optional
original clock preserved. Unit4/native3/related85 PASS, format/Clippy PASS; scoped
review accepted the repair. Current seven-group/18-caller-site matrix records
remaining payload controls without new IDs. Evidence: pctx01-caller-bound-
verification.json and pctx01-caller-input-matrix.json. Detail25/40, whole0/10,
unchanged; G03-D04 remains open. Current Tick incidental pass does not erase
retained PCTX36 failure. Required platforms and original startup failures remain.

## Current bounded schema and backup response proof

Current **G03-D04 and G04-D03 remain open**. Authored input controls now
qualify72 Task/Schedule schema combinations,12 Quota schema/shape/empty-batch
combinations and6 Restore schema/project/checksum refusals, in JSON and compact.
All compare prepared logical SQL/schema and non-DB bytes. A real CLI Backup
exposed a common response-destination defect: it published the archive then
attempted to publish the response to the same file, returning9. Control Backup
now preserves the archive destination and delivers success/error envelopes on
stdout, as declared in help. Real success and collision9 preserve archive bytes.
The relative-path fixture failure, actual publication failure and initial compile
failure are retained; final focused35 PASS (binary7/caller5/frontend23), format
and locked all-target Clippy PASS. Independent review accepted the bounded repair.
[Current schema/output evidence](../evidence/pctx01-caller-schema-verification.json).
Prior cap repair remains qualified by its own source-bound evidence; whole input
and output conditions are not closed. Detail25/40 and whole0/10 are unchanged.

## Current G03-D04 bounded common-input closure

Current **G03-D04 is closed as a bounded local common-input condition**.
All18 frozen caller/config/external JSON sites in seven existing groups now map
to named current or unchanged-branch prior proof. Current actual caller13 PASS,
related40 PASS, preceding caller11 PASS and Runner29 PASS; the new memory control
passes1 after disabling fixture diagnostics. Initial encoding, absolute Pack path
and diagnostics-mode test premise failures are retained without product changes.
Operations10 readers cover100 syntax/shape/encoding combinations plus20 cap and
2 owner controls; Adapter required shapes/unsupported6 and Statusline version
priorities remain distinct. Inventory explicit schema2, source UTF8 class3 and
partial discovered-config observations are not conflated. Filter manifest refusal,
Pack project-relative external syntax, Handoff permitted metadata/body, real-lease
Check Report14 controls and memory4096/4097 no-child/no-heavy-grant are qualified.
Quota10000/10001 cannot fit the original1MiB cap: minimum required-key encoding is
217bytes; actual cap-first2 is proved, not accepted-count semantics. Archive256MiB
proof remains oversize admission, not allocated exact-cap/growth. Prepared logical
SQL/schema and non-DB comparisons do not claim physical WAL or first-use rollback.
Format/locked all-target Clippy PASS; independent fixed-site audit found no common
input gap. [Closure evidence](../evidence/pctx01-caller-groups-verification.json) and
[18-site crosswalk](../evidence/pctx01-caller-input-matrix.json) preserve producer
ownership and required-platform limits. Detail **26/40**, whole gates **0/10**.
G04-D03 remains open; the prior Backup output repair remains qualified.

## Current G03-D03 pure-grammar closure

Current **G03-D03 is closed as bounded local pure-grammar qualification**.
The [140-route crosswalk](../evidence/pctx01-grammar-crosswalk.json) preserves all
C001–C140 identities and maps actual validators, unrestricted lookup/parser-only
routes and input/state-dependent exceptions to source hashes and named proofs.
Current focused93 PASS includes exact1855-case/140-leaf parser replay and16
existing authored admission suites. Additional Runner capacity1 and existing Run
capacity1 PASS qualify explicit operational metadata-budget8 exceptions without
child/resource effects. These floors remain in producers: local Helper owner/task
checks precede capacity, and cloud intent bypasses the local floor. They belong to
existing G04-D03/G05-D06 response-budget/exit conditions, not invented argument2
restrictions or reordered policy/state/capability. Exact endpoint/state combinations
remain there. Format/locked all-target Clippy PASS; independent final mapping
review found no missing pure-grammar boundary. No runtime behavior changed.
[Current evidence](../evidence/pctx01-grammar-crosswalk-verification.json).
G03-D04 input closure remains qualified; independent producer semantics, G08 races,
G09 native platforms and four original macOS startup failures remain separate.
Detail **27/40**, whole gates **0/10**; denominator and140/172/12 counts unchanged.

## G05-D06 bounded cancellation candidate — 2026-10-09

Shared Ctrl-C handling and narrow finalization are implemented. Actual native
CLI cancellation4/registered2 PASS; baseline and earlier failures remain retained.
No condition closes: G05-D06 full exit matrix/cancellation-publication combinations
and Helper/guardian paths remain open. Required native platforms remain in G09,
original startup failures in G05-D05/G09-D01. Detail27/40 and gates0/10 unchanged.
See [cancellation evidence](../evidence/pctx01-cancellation-verification.json).

## G05-D06 bounded local closure — 2026-10-09

Native shared exit matrix84 + cancellation combinations5 + capacity1 PASS. Every
existing0/2/3/4/5/6/7/8/9/10/130/child-exception category maps to named controls and
current source/log identities. Helper/canonical guardian cancellation and manual/
registered cancellation-publication failure preserve native truth/resource ownership.
The registered absent-artifact reread defect is repaired; initial failures retained.
Format/all-target Clippy PASS and independent acceptance qualify only this local
condition. Detail28/40, whole0/10. Register summary25 was stale against27 existing
item states; derive corrected count and add one, with no denominator/ID change.
Next existing G05-D02 remains open; G08/G09 and original startup failures remain.
[Current matrix](../evidence/pctx01-exit-matrix-verification.json).

## G05-D02 current bounded callback proof

Native callback refusal now preserves captured output, reaped native status,
artifact identity and durable native receipt without rerun. Four scenarios include
already-completed exit23 and actual artifact-store lock refusal; the reviewed control
also verifies available streams in the response when no raw artifact was published.
The initial failing strengthened control is retained. G05-D02 remains open; its
existing postspawn contract has seven remaining phase groups in the linked manifest.
This records execution substructure, not another reorganization, new ID or denominator.
Detail28/40 and whole gates0/10 remain unchanged. Next exact action is the grouped
pipe/wait/capture completion repair and fault proof, followed by receipt/runner
finalization proof under the same condition.

## G05-D02 pipe/wait/join qualification

The linked capture-fault manifest records native missing/setup/panic faults on both
streams and actual ECHILD. Available capture/artifact handles and native or explicit
unknown outcomes survive without rerun; failed streams remain incomplete. Both
workers finalize, including continuous reads under the existing250ms stop bound.
The initial continuous-read failure remains in evidence. No PID/PGID signal follows
failed wait observation. No resource-release or complete descendant-absence claim.

Three original phase groups have bounded local proof; four remain: job receipt
publication, CheckRun backend cleanup, CheckRun completion, Local Helper completion.
Responsibility/completion were reconciled after two updates without closing G05-D02:
original common §14/38 fault response contract only, no independent producer
expansion, no new IDs and unchanged28/40/detail or0/10whole. Next exact action is
receipt publication fault qualification, followed by the three Runner groups.

## G05-D02 receipt durability qualification

Active/final receipt write errors are processing failures, separately reported
from native outcome and raw artifact publication. Final native receipt precedes
artifact save so failed durability cannot generate passing evidence on reread.
Real filesystem obstruction covers both phases with intact captured streams,
prior receipt bytes and one invocation; successful typed report remains a control.
Initial ignored-write failure and99PASS1FAIL PID-readiness fixture failure remain
in the manifest, alongside fixture-only repair evidence. Original1s-budget startup
product failures are unchanged. Four phase groups locally qualified, three Runner
completion groups remain. Fixed condition still open;28/40 and0/10 unchanged.

## G05-D02 bounded local closure — current

All seven fixed callback/postspawn phase groups now have bounded local controls,
including shared Runner cleanup independent of DB/Helper publication. Final103
focused PASS/static PASS/read-only review support closure; prior partial entries
above remain historical evidence, not the current cursor. Backend Err is modeled
at its API boundary with a real completed child; actual backend wait/pipe/capture
faults have independent controls. No whole-platform or general containment claim.
Detail29/40, whole0/10, denominator unchanged. Next is existingG04-D03 full envelope
and exact final bytes across fixed supported representation/destination classes.

## G04-D03 current bounded Read repair

Actual hidden-Unicode and Markdown final-document expansion is measured after
rendering and each UTF-8-safe reduction, with final truncation metadata and newline.
Usable excerpt/hash/identity survive; require-complete refuses3 on presentation
reduction. Native24case matrix and final70 related tests/static/read-only review
pass; initial lost-excerpt and Clippy failures retained. See
[evidence](../evidence/pctx01-read-final-budget-verification.json).
G04-D03 remains open for fixed fallback identity/validation, presentation omission
and metadata-only/destination conditions. Detail29/40, whole0/10 unchanged.

## G04-D03 current fallback qualification and responsibility audit

Established response metadata/data, known opened Project identity, explicit
presentation truncation and exact warning omission counts are locally qualified.
Generated-ID capacity reserve repairs actual735bytes>690 boundary failure; final71
related/static PASS, prior failures retained. General fallback overflow after
reduction and frozen destination/representation crosswalk remain open. After two
updates without closure, scope/completion were reconciled: same G04-D03 common
contract, no independent producer expansion/new IDs or denominator change.
[Evidence](../evidence/pctx01-fallback-provenance-verification.json);29/40, whole0/10.

## G04-D03 bounded metadata fitting — current

Exact replaced envelope fields are disclosed, with omitted validation rather than
invented freshness and unchanged execution/error/reread proof. Synthetic large
metadata and native240-byte workspace4case controls pass; final73 related/static
PASS. [Evidence](../evidence/pctx01-metadata-fit-verification.json). General final
overflow after metadata exhaustion remains open and is the exact next action;
do not expand another helper. Fixed29/40, whole0/10/scope/denominator unchanged.

## G04-D03 bounded local closure — current

Only **PCTX01** is active and remains `implementing`. **G04-D03 is closed for
its bounded local common contract**: detail30/40; whole gates0/10. Scope and
all frozen denominators remain unchanged. [Evidence](../evidence/pctx01-final-capacity-verification.json)
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

Next: **G06-D03 within PCTX01**, successful-empty versus capability/resource6
and incomplete3 for the already frozen producer families. Finish that condition
before G07-D03. No other official task starts. Original four macOS1s startup
product failures, required-platform gaps and inactive PCTX36/PCTX47 backlog
remain open. No Actions, whole/platform rerun or push retry. All jobs terminal.

## Current G08-D04 closure — 2026-10-09

Only **PCTX01** remains active (`implementing`). **G08-D04 is locally closed**
for the frozen five-phase common matrix: **33/40** details, **0/10** whole gates;
all fixed denominators unchanged. [Manifest](evidence/pctx01-phase-matrix-verification.json)
binds source/executables/logs: selected4 and related202 tests across25 suites PASS,
locked all-target Clippy -D warnings, format and independent read-only review PASS.
The related run explicitly filters the original4 startup failures; they remain
unresolved G05-D05, not replaced by this result. This is not whole-suite PASS.

The [five-phase crosswalk](evidence/pctx01-phase-matrix-crosswalk.json) separates
source guards, prior actual SQL/CPU/hash/accounting proof and new actual controls.
Registry now uses anchored regular/nonblocking input with the original chunk clock,
admitted-length growth bound and pinned/reopened version gates; no arbitrary new
project-count cap. Config retains1MiB. Actual config/registry/source mutations and
chunk expiry preserve stable errors and identity. Unixctime detects observable
rewrite/restoredmtime, not universal ABA or Windows guarantees. Actual8KiB/64KiB
read expiry requires1 chunk/no later reads and original200ms; FIFO refuses safely.
Final render/post-fitting admission checks the same clock, retaining complete
TIMEOUT7 data, normal budgets, nonempty Find timeoutpartial3 and established errors.
The output fixture expires after real rendering; it does not prove serializer
preemption or blocked native-write cancellation. Completed unavoidable write stays
true and retained-clock accounting uncertainty never reverses delivery.

First selected2PASS1FAIL was a fixture count error: growth needs the extra-byte
second read; product already refused4. Corrected count only; initial log retained.
Initial related invocation named nonexistent output_delivery and ran no tests;
corrected output_delivery_observation is in final202. All jobs terminal.

Next: **G05-D05 within PCTX01**, original four macOS startup product failures at
unchanged1s/8x16/stream/group criteria. Required-platform gaps, inactive PCTX36/47
backlog and publication approval boundary remain. No Actions or push retry.

## Historical G07-D03 closure — 2026-10-09

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

Historical next was G08-D04, now locally closed above; G05-D05 is next. Required
Windows console proof remains G09-D04. Original four macOS1s startup product failures,
other required-platform gaps and inactive PCTX36/PCTX47 backlog remain open.
No Actions, whole/platform rerun or push retry. All validation jobs terminal.

## Historical G06-D03 closure — 2026-10-09

Only **PCTX01** is active and remains `implementing`. **G06-D03 is closed for
its bounded local common contract**: detail31/40, whole gates0/10, unchanged
scope/denominators. [Evidence](../evidence/pctx01-producer-coverage-verification.json)
records final229 focused tests across18 suites, locked all-target Clippy -D warnings,
format and independent read-only review PASS. Source/binary/log hashes bind proof.

The frozen9-family [crosswalk](../evidence/pctx01-producer-coverage-crosswalk.json)
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

## Current G05-D05 diagnostic — 2026-10-09

Only **PCTX01** remains active (`implementing`); **G05-D05 remains a product
failure**, not closed. Detail33/40, whole gates0/10, fixed denominator40 unchanged.
[Owned-child snapshot evidence](evidence/pctx01-startup-thread-snapshot-verification.json)
adds pre-expiry task/thread observations; production and original tests unchanged.
Prior balanced/timeline512-attempt experiments and retained dyld samples were read,
not repeated. No startup pass is substituted for the original4 failures.

Initial raw8-concurrent/16-wave workload:127 within-budget,1 original1s expiry.
The expired child's exact PID/native start identity matches250ms/751ms observations:
task suspend_count0, no task userCPU/syscall progress, no output; actual cancellation
signal9 and bounded reaping retained. Initial thread bytes0 are unavailable, from
incorrect handle/ID-flavor pairing; zero fields do not describe actual thread state.
Corrected matching pthread-handle info yields9 valid waiting-state3 records with
task suspend_count0. Subsequent128 within-budget observations are diagnostic only;
no code repair or original regression qualification. Observer overhead is recorded.

Independent read-only review confirms counts/hashes, with two timing/ownership
qualifications: corrected waiting records concern later nonexpired children, not
the expired child; result clock precedes waitid, so within-budget is the recorded
pre-probe criterion, not exact post-probe latency proof. No promoting rerun.

The raw single-threaded launch driver differs from the actual multithreaded
supervisor. Waiting and task suspension counts do not identify blocked syscall,
kernel wait reason, policy cause or universal startup latency. Exact cause remains
unproved; no environment exemption. Strict native diagnostic compilation passes;
wrapper cache warnings and initial API-fixture error remain in evidence. No Rust
rerun, Actions, security/permission change or push retry. All jobs terminal.

A target-only spindump capability check on this execution shell was refused:
`spindump must be run as root when sampling the live system` (exit77).
Initial fractional-duration rejection64 is retained separately; no capture occurred.
No sudo or privilege change attempted. Exact kernel wait observation therefore
needs an authorized administrator capability; this does not excuse product failures.

Next action stays **G05-D05**: identify the exact owned child's pre-expiry wait
reason with an authorized read-only capability, or establish a concrete supervisor
defect. Do not repeat unchanged startup attempts or switch official tasks. Required
native-platform gaps and PCTX36/PCTX47 backlog remain separate and unresolved.

## Corrected original-supervisor observation — terminal80102

[Manifest](../evidence/pctx01-startup-native-clock-capture-verification.json) records
68 outcomes/64 successes/4 zero-output original1s expiries. Traced13700 succeeds;
11/12 policy-wait samples cannot qualify untraced expired children. Native-clock
correction and one capture succeeded, original test failed. No further automatic
privileged probe or unchanged retry. Only PCTX01/G05-D05 remains active and
product_failure;33/40 details,0/10 whole gates and all fixed denominators unchanged.
