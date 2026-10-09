# macOS script startup: minimal evidence report

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
[Evidence and limits](pctx01-startup-triage.md).

## Contract and environment

Native macOS26.7.1 build25G241,Apple Silicon,ordinary-user execution,Rust1.99.0
commitb940084d7eb6a299eb4bfeb8e34901bc051e7ac4. Query startup must preserve its
original1000ms request deadline,8workers×16queries,owned process group,independent
stdout/stderr and output-overflow refusal. No warmup,budget extension,interpreter
replacement,security-policy change or retry can qualify a repair.

The public repository is https://github.com/Jung95/pctx. Original acceptance source
is tests/project_deadline.rs,hash8b9fbc97fbd5acaf612b2d75b94cf810faa2b61614b654495082462b1941a8dc.
The latest diagnostic product source is7181e657042c75553bacb6eeb77b5ec2c07e81bc.
Its separate final-return deadline repair does not address prepayload startup wait.

## Established observations

- A previous owned-child kernel capture binds actual expired children to native
  process identities and samples AppleSystemPolicy::procNotifyExecComplete →
  evaluateScript → waitForEvaluation → ASPEvaluationManager::waitOnEvaluation.
  It establishes observed early waiting,not a continuous whole1s state,exact signal
  ordering or the cause of delayed evaluation.
- Reverse host-presence comparisons retain failures in both human-declared app
  states. Closing the UI is not a sufficient observed cure;helpers,order,time,
  cache and other host effects remain confounded.
- A separately approved syspolicyd-only finite sample batch runs the four unchanged
  tests once each at ordinary UID. Three pass;concurrency fails with two zero-output
  original-deadline expiries before root exit or group probing. Service native
  identity/header checks pass,but sample attachment has a residual PID-reuse race.
- Service results contain aggregate IPC/semaphore waits and sparse signature
  functions. Most frame rows remain unknown after privacy reduction. CPU values
  are retained raw with a conditional Mach-timebase conversion. None identifies
  which evaluation belongs to an expired child or proves queue saturation/deadlock.
- SystemPython3.9.6 monotonic timestamps have process-local origins. Do not directly
  align collector/driver timestamps or claim exact sample/test overlap. Within-
  process durations and IPC ordering remain usable.
- One approved default-masked historical log query,limited to2sUTC and explicit
  references to the two expired owned PIDs,returns0events. Missing records are
  inconclusive;no broader query/private logging mode was used.

## Current correlation limitation

Static inspection of this installed policy-service binary finds separate PID,
evaluation ID and signpost ID. Its conditional Path and EvaluationID signposts
share the signpost ID; the prior PID-labelled query excluded this channel. This
is not evidence that events were emitted or retained. A new owned-fixture mapping
run preserves the original concurrency test and expires one child at the original
deadline. Mapping is instrumented and does not qualify a repair. A target-specific
finite signpost read was approved and executed once: zero exact-path matches,
no evaluation mapping and no second query. The consumed scope was not widened.
No vendor submission or security change follows this draft.

A later human-requested unmodified-product recheck again yields3PASS/1concurrency
FAIL with two zero-output original-deadline expiries.26aggregate memory samples
have normal published-XNU pressure flag1,unchanged swap use and zero swap-in/out/
pageout deltas. This does not support active swap thrashing in that interval,but
cannot exclude unsampled transients or establish a policy-service cause.

## Evidence and requested guidance

See [consolidated triage](pctx01-startup-triage.md),
[owned-child kernel analysis](evidence/pctx01-startup-auth-all-four-capture-analysis.json),
[service verification](evidence/pctx01-startup-service-verification.json),and
[historical target-log verification](evidence/pctx01-startup-target-log-verification.json).
Raw service samples were removed;published reductions omit arbitrary messages,
paths,addresses and unknown strings. The named tools and expired owned children in the
[service cleanup receipt](evidence/pctx01-startup-service-cleanup.json) are absent;
the audited generated directories listed there were removed. The
[historical query verification](evidence/pctx01-startup-target-log-verification.json)
also confirms its named reader/log processes are absent. No security
setting,service restart or persistent privileged component was introduced.

What supported target-specific interface can correlate a child awaiting script
policy evaluation with its evaluation request/completion without enabling private
logging,observing unrelated processes or changing security policy? Is there a
known applicable platform correction for delayed completion under this native
launch contract? A corrective proposal must preserve the original acceptance
conditions and be verified without the observer;diagnostic PASS alone is insufficient.

## Later owned-parent timeline

## Prioritized investigation executed — current, 2026-10-09

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
for this unresolved condition. No product/test source changed.

Only **PCTX01** remains active; **G05-D05 is product_failure**, fixed **33/40**
details and **0/10** whole gates. No next official task is selected. Completion
still requires a causal applicable repair and all original acceptance conditions,
including required platforms; diagnostic success cannot close this failure.

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
