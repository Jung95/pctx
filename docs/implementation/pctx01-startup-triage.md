# PCTX01 startup failure: consolidated triage

## Repair review at original conditions — current, 2026-10-09

PCTX01-G05-D05 remains product_failure; fixed33/40 details,0/10 whole gates.
No causal PCTX patch is supported yet: failed-path validation63us/spawn628us
versus98.88ms late policy result. Existing static evidence shows scan queue
already configured USER_INTERACTIVE QoS; caller-priority promotion lacks a
matching cause. Admission serialization cannot renew the original deadline.
Interpreter/prewarm/reuse/security exceptions do not preserve conditions.
Next: identify the expensive scan operation and an applicable platform/product
correction. No test rerun, service collection or OS/security change performed.
[Candidate decisions and full verification requirements](evidence/pctx01-startup-preserved-condition-review.md).

## Policy queue attribution — earlier, 2026-10-09

PCTX01-G05-D05 alone remains active and `product_failure`; fixed33/40 details,
0/10 whole gates. Completion still requires a causal repair and all original
startup/platform acceptance conditions. Source review found no explanatory
fixture-write/lifetime, child-pipe, pre-exec suspension or deadline defect.

Installed syspolicyd static analysis shows `scanTarget:` dispatching synchronous
`performScanWithArguments:withCodeEvaluation:` on serial `_workQueue` (offset
0x48). Its redacted PST path is `NSString.hash` of the path in the normal
volume/object-ID branch. Execution evaluation and completion queues are
concurrent; do not infer a single evaluation worker or an eight-worker cap.
Independent review confirms the selectors, queue type and call path.
[Static map](evidence/pctx01-startup-policy-queue-map.json).

An offline reconstruction joins all16 historical synthetic first-attempt paths
in two modes to stored policy events. Parent44072 is the only match within
candidate1..100000, both path aliases and the recorded generation rules.
The eight script-group scans are non-overlapping, separated by241–291us;
first scan to final result spans1.154068s. Expired child44203 is last, with
193.916ms of its own scan. This strongly supports accumulated serial policy
scan delay in that historical reproduction. It does not yet prove the current
original-supervisor cause: historical TMPDIR was reconstructed, hashes are
noncryptographic, and that harness has no exact UTC/deadline correlation.
[Joined evidence](evidence/pctx01-startup-policy-hash-joined.json).

A once-only isolated copy of the byte-identical original Rust concurrency test
retains1000ms,8×16,native exec/cwd/env,groups,streams/overflow and cancellation.
Exact-owned-fixture hashing occurs only after original cancel/reap; raw paths
are not exported. NSString/CFHash pure controls pass, including Unicode;
one Rust API unit passes. Original test fails101:113 observed outcomes,
112 OUTPUT and one TIMEOUT35071, zero streams/no observed exit. Remaining15
planned attempts are censored. No dropped timing points; all known114 parent/
child PIDs absent; product and original test hashes unchanged. Two native/wall
clock endpoint bounds overlap with a2us total envelope; an intermediate clock
step is not excluded. Post-reap diagnostics can affect later iterations and
are not qualification. [Verification](evidence/pctx01-startup-policy-forward-verification.json).

Human authorized the concrete fresh stored-service-log scope. It executed once:
12:43:38–12:43:41 UTC,16 exact owned hashes/GK scan-result messages, exit0,
17,401 raw bytes,16 admitted events/0 rejects. Raw memory discarded; reader
36440 reaped and absent. No admin authentication, sampling, configuration change,
fallback query or new product test. Scope is now consumed.
[Read verification](evidence/pctx01-startup-policy-forward-read-verification.json).

All8 first owned fixtures have one unique scan/result pair. Scan order:
35068,35067,35069,35066,35070,35072,35073,35071. Non-overlapping scans have
211–256us gaps and span1.092233s. All7 successful children have policy results
before their original deadlines; failed35071 is last.

| Failed35071 event | UTC, 2026-10-09 |
| --- | --- |
| Owned native birth | 12:43:38.070694 |
| Scan starts | 12:43:38.984119 |
| Original deadline bounds | 12:43:39.070264542–39.070266542 |
| Parent observes timeout bounds | 12:43:39.072253–39.072255 |
| Policy scan result | 12:43:39.169148 |

Birth-to-scan is913.425ms; scan lasts185.029ms, with only86.146–86.148ms
of the original budget left when it starts. Result is98.881–98.883ms after
deadline. Every recorded drain remains zero; no root exit is observed.
Cancellation starts15us after parent timeout observation. Independent reviewer
/root/work_control confirms the eight-path join and arithmetic.
[Offline join](evidence/pctx01-startup-policy-forward-joined.json).

This directly associates the expired original-supervisor owned fixture with a
policy scan finishing too late, supporting accumulated serial policy scanning
as the startup-budget failure mechanism. It does not reveal exact queue arrival,
kernel reply/acknowledgment/waiter release, or why individual scans take this
long (including XProtect involvement). Hashes remain noncryptographic and the
wall/native conversion assumes no hidden intermediate wall-clock step.
Instrumentation can perturb scheduling. No product repair/qualification follows.
Next within PCTX01-G05-D05: determine an applicable causal correction preserving
original1000ms/8×16/native exec/stream/group conditions. No policy bypass, warmup,
fixture replacement or deadline reduction; no unchanged rerun. Shared target
contains diagnostic artifacts and must rebuild originals before acceptance.
Private source/records/helpers were removed; selected numeric evidence retained.

## Late child chronology — earlier, 2026-10-09

Priority 3 used an ordinary-user native numeric alternative to privileged stack
sampling. One byte-identical original Rust concurrency test retains 1000ms,
8×16, argv/cwd/env, groups/streams/overflow and original cancellation/reap.
Only matching owned unreaped direct children with the exact synthetic fixture
are sampled in the remaining 350ms, at least 10ms apart, capped at 40 samples.
No interpreter replacement, retention, warmup, retry, admin authentication,
service observation, process inventory or security change.

The test fails101: 98 recorded outcomes,96 OUTPUT and two TIMEOUTs (32487/32489);
the 30 planned-but-unobserved attempts remain censored, not passes or failures.
Each failed child has24 fully admitted complete pre-deadline samples: thread
state3 (WAITING), one thread,numrunning0,unchanged rawCPU/syscall/context-switch
counters. Samples span about345ms to3.6ms before the deadline lower bound;
max observed read interval41/39us, no dropped records. Both failures have zero
streams/no observed root exit. This shows no sampled late waiting-to-running
transition or runnable-child CPU-starvation state. Between-sample transitions,
service-side starvation, continuous policy wait and exact wait cause remain unknown.
[Verification](evidence/pctx01-startup-late-state-verification.json),
[result](evidence/pctx01-startup-late-state-result.json).

One isolated control passes; independent pre-run and evidence review find no
material blocker. Product/tests remain unchanged. All98 recorded children and
parent are absent; private source/records/tool session removed. Shared target
contains a diagnostic binary: rebuild and hash-check original source before any
acceptance run. Diagnostic PASS would not qualify repair.

Priority 4 reads installed Xcode System Trace settings only, with no recording.
The first sandbox attempt aborts on cache permissions; ordinary UID sandbox
escape succeeds without admin authentication. Options do not prove target-only
collection; no broad trace is started. [Capability](evidence/pctx01-startup-system-trace-capability.json).
Priority 5 lifecycle events cannot identify this late wait cause and do not justify
an unchanged rerun. The privileged stack plan remains unexecuted; do not reuse
old retained-child helpers or widen service scope. Next unresolved work is exact
owned-target wait/request-response attribution and an applicable causal repair,
not another known-wait sample or a different official task.

**PCTX01-G05-D05 remains product_failure**, fixed33/40details,0/10whole gates.
Required acceptance/platform conditions remain; no exemption or closure.

## Prioritized investigation executed — earlier, 2026-10-09

Human authorized proceeding in the displayed priority order. Priority 1 consumed
its exact owned-path, 2-second UTC service-log scope once: exit 0, zero events,
no matching signpost ID and no second interval query. Missing records remain
inconclusive. The old private mapping directory was removed; scope cannot be reused.
See [finite read verification](evidence/pctx01-startup-signpost-target-verification.json).

Priority 2 ran one isolated diagnostic copy of the byte-identical original
1000ms/8×16 concurrency test. It failed (exit 101): 98 persisted outcomes,
96 OUTPUT and two TIMEOUTs, PIDs 31189/31195; the remaining 30 intended attempts
are censored/unknown. Both failures have zero streams and no observed root exit.
Native spawn durations are 0.979/0.819ms; largest adjacent parent timing gaps
6.289/6.277ms; terminal-to-group-kill gaps 2/16us. Parent polling continued; these
records do not support a long single parent spawn/drain/probe/wait stall as the
observed explanation. They do not identify the child's wait cause or prove a
continuous kernel state. No dropped points; both logged failure identities have
records; all known parent/child PIDs are absent. Instrumentation overhead and
post-cleanup recording can affect timing; this is not acceptance qualification.
[Result](evidence/pctx01-startup-phase-result.json),
[verification](evidence/pctx01-startup-phase-verification.json).

Priority 3 is the next diagnostic within the same task: establish late pre-cancel
child wait/runnable transitions with a target-only chronological collector.
[Concrete design](evidence/pctx01-startup-timeline-next-plan.json) is not execution
ready: spindump PID selection alone does not bind native start identity, and the
old late-capture helper retains children beyond timeout. Do not reuse it. Resolve
safe identity/lifecycle admission and fractional-duration behavior before any
single-auth batch. No root collector, service sample, broad profiler or unchanged
retest was started. Priorities 4/5 remain conditional alternatives, not substitutes
for this unresolved condition. No product/test source changed. The private phase session was removed after
PID absence verification; shared target contains a diagnostic binary and must be
rebuilt from original source before acceptance testing.

Only **PCTX01** remains active; **G05-D05 is product_failure**, fixed **33/40**
details and **0/10** whole gates. No next official task is selected. Completion
still requires a causal applicable repair and all original acceptance conditions,
including required platforms; diagnostic success cannot close this failure.

PCTX01-G05-D05 remains an unresolved product/contract failure. Detail closure is
33/40; whole gates0/10. This record consolidates existing evidence, not a new task,
requirement, test run or environment exemption. All prior failures remain valid.

## Frozen acceptance and source boundary

Resolve the four original `tests/project_deadline.rs` startup tests with the
original one-second query deadline,8 workers×16 queries,owned process group,
independent streams and overflow assertions. Preserve native executable resolution,
child cwd/environment,retained identity and cancellation. No warmup,budget extension,
interpreter substitution,security-policy bypass or retry-to-pass can satisfy it.

The historical [source reconciliation](evidence/pctx01-startup-policy-ownership-reconciliation.json)
found no evidenced PCTX correction:the current supervisor supplies the original
Deadline before launch and while observing streams/group/exit. Its deadline failure
reports are truthful. This does not establish that PCTX has no defect.

## Evidence and its limits

| Measurement | Recorded result | Meaning |
| --- | --- | --- |
| One-auth owned-target capture5159 | Original concurrency98outcomes:96success,2TIMEOUT | Actual expired17810/17812 sampled in script-policy wait;instrumented diagnostic,not qualification |
| Forward ordinary-user comparison | Declared open3PASS/1FAIL→closed4PASS | Host-associated difference,confounded by order/time |
| Partial reverse | Declared closed4PASS;open not run | Input-language interruption;incomplete comparison retained |
| Recovery reverse | Declared closed3PASS/1FAIL→open3PASS/1FAIL | Closing app is not a sufficient observed cure;second phase is not universally passing |

[Exact kernel evidence](evidence/pctx01-startup-auth-all-four-capture-analysis.json)
and [raw trace](evidence/pctx01-startup-auth-all-four-capture-stage-2-trace.txt) bind
UID,parent,native start and unique PID for17810/17812. Both show
`AppleSystemPolicy::procNotifyExecComplete → evaluateScript → waitForEvaluation`
and evaluation-manager waiting. First observed samples are about294ms after native
fork,with millisecond wall-clock precision/stability limits. Child report endings
near cancellation do not prove all142samples pre-cancel or a continuous whole1s wait.
The reason for delayed evaluation remains unknown;no operational daemon was observed.

[Forward](evidence/pctx01-startup-host-control-comparison-verification.json),
[partial reverse](evidence/pctx01-startup-host-reverse-comparison-verification.json)
and [recovery reverse](evidence/pctx01-startup-host-reverse-recovery-comparison-verification.json)
remain separate. Product sources match;binary is identical within each comparison,
but the reverse binary differs from the forward build. Host absence is human-declared,
not process-inventory verified. Global policy/cache/contention,time,remaining helpers
and app-reopening effects remain confounded. Failed concurrency runs are censored;
no full attempt denominator or comparative failure rate is available.

## Corrective boundary and resumption

No applicable supported correction was found in the bounded
[official-documentation check](evidence/pctx01-startup-host-correction-docs-check.json).
That absence is not proof no correction exists. The historical startup evidence does not justify a causal startup patch or a
host/security change. The later final-return repair below addresses a separate
proved deadline defect. Do not repeat consumed commands,
known-wait kernel capture or unchanged host controls. All generated collectors and
comparison binaries are removed;raw evidence remains. No live heavy job.

Resume corrective implementation only with evidence identifying a specific causal
PCTX operation,or a documented applicable supported platform correction. Validate
any correction against the unchanged original tests and integration candidate;
a later isolated pass cannot erase retained failures. Additional observation of
operational services or OS-policy changes exceeds the current owned-target approval.

Required native macOSarm64 qualification still fails. macOSx64,Linuxx64 and
Windowsx64 remain unverified;supporting Linuxarm64 is no substitute. G10 integration
and final closure remain open. No other official task is activated.

[Consolidation hashes](evidence/pctx01-startup-consolidated-triage.json) bind this
record to the inspected source and evidence inputs.

## Apple policy documentation check

The [bounded primary-source check](evidence/pctx01-startup-apple-policy-docs-check.json)
fetched Apple's [Terminal and script protections](https://support.apple.com/en-ie/guide/security/sece3b202c4b/web).
For macOS26.4+,the document describes independent protections including descendant
behavior inspection after pasted terminal commands. This explains why changing UI
app presence alone is not a comprehensive control of documented process-tree policy
inputs,but does not attribute these timeouts to that mechanism. The AppleScript/JXA
section is not proof about the shell-script fixtures. No matching stack explanation,
latency guarantee or applicable correction was identified. Another launch-constraint
page was not readable enough to support a correction claim. No new test,process
observation,authentication or policy/provenance manipulation occurred.

## Exact Rust source review and separate final-return repair

[Source review](evidence/pctx01-startup-rust-spawn-source-check.json) compares the
installed Rust commit with PCTX launch configuration. Process-group0 is supported
by the posix_spawn path; no custom pre_exec or forced suspension exists here. This
is static evidence, not a runtime backend trace or a policy-delay explanation. No
applicable supported platform correction was established by the bounded search.

[Final-return correction](evidence/pctx01-query-final-deadline-verification.json)
adds the missing original-deadline check after real root reap and before success.
Its controlled native-boundary regression fails before and passes after correction.
Released identity prevents signaling after reap. Original startup tests remain
unchanged; integration11PASS/3FAIL still reports zero-output pre-exit timeouts,
not this late-return defect. G05-D05 remains open;33/40details,0/10gates.

## Completed service-side diagnostic — current, 2026-10-09

[Verification](evidence/pctx01-startup-service-verification.json): approved one-auth
syspolicyd-only batch completed,four unchanged ordinaryUID502 tests3PASS/1FAIL.
Concurrency expired26165/26166 at original1s with zero streams,no EOF/root exit/group
probe. All frozen hashes and logs match;all reported generated tools/parents/expired
PIDs absent. Private raw/root-tool directories and isolated session/binary removed;
shared target retained. No OS/service/security change or product repair.

SystemPython3.9.6 clocks have process-local origins: no direct root/driver alignment
or exact overlap claim. Unknown frames dominate reduced samples;raw CPU counters
and conditional conversion cannot attribute evaluation to either child. No queue
saturation/deadlock or applicable platform correction established. Do not repeat
this consumed capture. Next requires target-specific policy request/response evidence
or a supported vendor correction;message/log tracing or another service exceeds
this approved stack/counter scope. PCTX01-G05-D05 product_failure;33/40details,
0/10whole gates. Only PCTX01 active;no live job,authentication or operator action.
Earlier authentication-pending instructions are superseded.

## Historical target-log query completed — current, 2026-10-09

[Verification](evidence/pctx01-startup-target-log-verification.json): human-approved
single ordinaryUID502 query for the explicit two expired PIDs in the exact2sUTC
window completed exit0 with0matching events. Script/plan/control hashes match.
No administrator authentication,new test,retry,broader fallback,raw disk data or
OS/service/security/logging change. Generated reader/log PIDs absent.

Zero matching default-masked records are inconclusive: no cause,successful launch,
policy exemption or correction follows. PCTX01-G05-D05 remains product_failure;
33/40details,0/10whole gates. Only PCTX01 active;no live job or user action pending.
All earlier authentication/log-approval instructions and commands are consumed.
Next needs target-specific evaluation mapping or an applicable supported platform
correction. Do not repeat unchanged diagnostics. A minimal vendor evidence report
is prepared;external posting/messaging is not authorized or performed.

## Current static correlation and owned-path mapping — 2026-10-09

The [installed-binary audit](evidence/pctx01-startup-policy-static-map.json) confirms
that the script log has no explicit PID field and evaluation IDs differ from PIDs.
The current gatekeeper implementation conditionally emits Path and EvaluationID
payloads sharing a separately generated signpost ID. Local log(1) omits signposts
unless requested. Historical primary reverse-engineering provides architectural
comparison only ([Objective-See,2021](https://objective-see.org/blog/blog_0x64.html));
no old vulnerability,debugging advice or security bypass is applied here.

The previous query's empty result did not test this channel. Current completion
callback sends its result before its end signpost; driver failure is possible.
An end cannot prove kernel acknowledgement,waiter release or payload execution.
No current latency cause or applicable platform correction follows static analysis.

New [owned-path diagnostic](evidence/pctx01-startup-path-map-result.json): original
concurrency test once,ordinary UID502,unchanged source/1000ms/8×16,native exec,
cwd/env/group/streams/cancellation. Exit101,113mapped outcomes,oneTIMEOUT27817.
Counts are censored. Mapping adds synchronous work within the original clock;
outcome writes occur after cleanup. PID/native start/parent/UID and exact synthetic
fixture content are admitted; files are not an atomic executed-content proof.
Missing observations are unknown. No source patch or acceptance qualification.

The [own synthetic schema control](evidence/pctx01-startup-signpost-schema-control.json)
validates4same-ID records and actual JSON fields without service observation.
[Next finite scope](evidence/pctx01-startup-signpost-target-scope-plan.json) is prepared
but awaits separate approval: target-specific existing Path→signpostID→EvaluationID/
interval records,one2sUTC window,no private logging or wider fallback. At most two
queries; zero or ambiguous IDs stop. PCTX01 remains33/40details,0/10whole gates.

## Human-requested memory recheck — 2026-10-09

[Verification](evidence/pctx01-startup-memory-recheck-verification.json) records3PASS/
1concurrencyFAIL from unmodified product/tests,each once at original1000ms/8×16.
Expired28848/28853 have zero streams,no rootexit/EOF/group proof.26passive aggregate
samples show16GiB RAM,normal published-XNU sysctl flag1,unchanged841.5reportedM
swap,deltaSwapins/Swapouts/Pageouts0. The sysctl emits converted notification flags,
not internal enum values. Exact installed-source match is not established.

This does not support shortage/swap thrashing in the observed interval;it does
not exclude unsampled transients or identify policy-service latency. No artificial
pressure,service/app logs,administrator authentication or settings change. Prior
failures remain. PCTX01-G05-D05 product_failure,33/40details,0/10whole gates.

## Debug-method research — 2026-10-09

[Research evidence](evidence/pctx01-startup-debug-methods-research.json) compares
current failures,Apple primary documentation and installed tool manuals. Research
only:no test,service log query,live stack/profile collection or authentication.
The earlier prepared signpost content-scope approval remains pending.

| Order | Method and question | Boundary |
| --- | --- | --- |
| 1 | Existing exact-owned-file Path→signpostID→evaluationID/interval correlation | Prepared finite read;separate approval. No/ambiguous records are unknown;end is not kernel acknowledgement. |
| 2 | Owned-parent native timeline for a specifically missing lifecycle phase | Isolated diagnostic copy,shared native clock,bounded in-memory data,persist after original cancel/reap;no arbitrary path/argv exports. |
| 3 | Target-only chronological kernel/user stacks for a new timing question | `spindump -onlyTarget -timeline` documented locally;exact identity,bounded authentication/cleanup. Bare PID still samples the whole machine. Previous known-wait capture is not a reason to repeat. |
| 4 | System Trace for blocked/runnable/preempted and VM/I/O distinctions | Validate actual collection scope first;attach/UI filtering does not prove target-only collection. Broader/service-inclusive capture needs separate scope approval. |
| Supporting | PID-specific kqueue NOTE_EXEC/NOTE_EXIT | Registration race,aggregation and receipt-time limits;exec event is not payload/policy-completion proof. |

Apple's [System Trace explanation](https://developer.apple.com/videos/play/wwdc2016/411/)
distinguishes blocked from runnable delays that ordinary CPU sampling may miss.
[Signpost guidance](https://developer.apple.com/videos/play/wwdc2018/405/) supports
interval correlation and warns of instrumentation overhead. These tool descriptions
are not evidence of the actual cause on this host.

`fs_usage` and `dtruss` require privileged tracing;only consider a specific missing
I/O/syscall hypothesis after collection-scope review. EndpointSecurity requires
Apple-issued entitlement and TCC authorization;installing a security client is not
a routine debugger shortcut. Local sysdiagnose manual describes system-wide logs,
profiles and dumps;passing PID adds heap/VM data rather than narrowing all capture.
Do not use broad captures or weaken protection under target-only consent.

Static checks find selected Xcode27.0 and its xctrace file. Runtime recording rights,
license,template scope and functionality are untested. No installation or configuration
change. A causal repair still requires an observed faulty transition,an applicable
correction and observer-free original acceptance tests. PCTX01 remains33/40details,
0/10whole gates;no task transition.
