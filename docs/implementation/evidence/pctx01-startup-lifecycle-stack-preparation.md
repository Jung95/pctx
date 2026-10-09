# Lifecycle-aware stack diagnostic preparation — 2026-10-09

Active task: PCTX01-G05-D05,product_failure,33/40 detail conditions,0/10 gates.
The prior root observer terminated before readiness with a generic assertion;
a subsequent audit found no XprotectService candidate. This does not prove its
earlier absence. A transient endpoint cannot safely be required to exist before
the test without creating/prewarming it,which would change original conditions.

The revised finite observer admits only syspolicyd before readiness. The ordinary
original test then starts without waiting on service discovery. After admitting
its first fresh owned native child,the observer seeks the exact XprotectService
at most50 times,using exact-name discovery only. Each query timeout is at most
100ms and remaining original earliest deadline. Remaining time is recomputed
after parent identity checks,checked after discovery and before each sampler
launch. The product deadline is never renewed. Absent/ambiguous/late/mismatched
endpoint refuses both samples; no service invocation/restart/security change.

The two approved samples retain2seconds/10ms each,total4requested sampler-seconds,
near-concurrent launch,8MiB hard file cap,allowlisted image UUID/relative offsets,
no body/symbol/queue strings and reap-before-raw-removal ordering. New failures
retain only fixed stage/check codes and candidate/poll counts. Collector terminal
status derives from a validated root-owned result receipt,not wrapper exit status.

Seven fake-native/process controls verify partial sampler timeout/reap order,
stale trigger,endpoint absence,ambiguity,path mismatch,delayed endpoint success
and deadline crossing. These are modeled controls,not platform qualification.
The isolated diagnostic source builds with --locked --offline --no-run in14.51s;
original product/test/lock hashes remain unchanged and original test byte-identical.
An initial build command had no cargo on PATH (exit127,no build/test); existing
repository-local .toolchain/cargo and .toolchain/rustup were used successfully.
Independent passive review confirmed deadline and terminal corrections with no
material blocker. No service observation or execution by the reviewer.

Remaining limitations: cooperative checks cannot guarantee native spawn finishes
before deadline; narrow PID races remain. Endpoint discovery may miss the scan,
and samples may overlap unrelated requests. Service sampling perturbs timing.
No retained sample proves a product repair or complete request-cost attribution.
Shared target artifacts require a normal-source rebuild before qualification.

[Scope and pins](pctx01-startup-lifecycle-stack-scope-plan.json),
[modeled controls](pctx01-startup-lifecycle-stack-control.json).
