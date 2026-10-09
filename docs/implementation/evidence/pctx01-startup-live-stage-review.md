# One-shot live policy-stage result, 2026-10-09

PCTX01-G05-D05 remains `product_failure`: 33/40 details, 0/10 whole gates.
No product repair, security change, administrator authentication or Actions run.

## Execution and preserved contract

Human approved one live syspolicyd PID665 capture: three stage prefixes only,
7 seconds / 128KiB / 256 events, after an owned readiness pulse. The original
concurrency test remains byte-identical: native execution, 1000ms per query,
8 workers × 16 planned calls, original groups/streams/overflow/cancellation.
The original test creates one fresh fixture per worker, then uses it for that
worker's 16 calls; diagnostic instrumentation does not add fixture reuse.
The shared target contains an instrumented diagnostic artifact and requires a
normal-source rebuild before product acceptance.

The first native libproc BSD identity preflight returned insufficient bytes
before scope consumption, subscriber launch or test. Its errno was not captured;
a permission cause is not established. A different minimal native SDK
KERN_PROC_PID + proc_pidpath method succeeded under ordinary UID502, with exact
PID/UID/birth/path admission before and after capture. This used the approved
identity scope and did not bypass authentication. Independent passive review
accepted the revised admission, retaining the separate-read PID-race limit.
Owned readiness and driver-interruption controls passed before execution.

## Result and interpretation

The single original test exits101: 98 observed outcomes, 96 OUTPUT, two TIMEOUTs
(PIDs38966/38967). The remaining30 planned calls are censored. Both failures
retain zero stdout/stderr and no root exit observation; neither is covered by a
captured stage hash. The outer watchdog did not fire.

The collector admitted its readiness pulse, received8773 bytes /10 parsed events,
and retained6 events for three owned fixture hashes. Two unmatched tuples were
discarded and no service bodies or unowned hashes were saved. Adjacent same-fixture
XProtect-result and scan-complete messages are16/24/16 microseconds apart.
Each hash belongs to16 successful invocations, so this is a fixture-level
observation, not a unique PID/evaluation pairing. These gaps do not measure the
preceding XProtect analysis or distinguish its overlap with PolicyScanner.

The listener exited0 early at6.472017374 seconds; the driver was still active.
Coverage remains unknown. Missing events cannot establish that a scan, XProtect
analysis or forced-rule work did not occur. This consumed scope is not retried
or extended. The earlier185.029ms failed-scan result remains unpartitioned.

## Cleanup and next action

Listener38944, helper38943 and driver38952 were reaped. Root38953 was reaped with
original exit101; all102 known root/recorded-child/tool PIDs were absent, all
recorded timing points had zero drops and valid clocks. Root/named-PID checks
do not prove universal descendant cleanup. The0700 prepared source/records and
identity helper were removed after hash verification. No live diagnostic job or
administrator prompt remains.

Next work stays G05-D05: identify a supported correction using a genuinely new
operation-level observable or vendor-confirmed cause. Existing service scopes
are consumed; another collection requires a concrete changed hypothesis and
appropriate finite approval. Do not repeat this test/capture, renew deadlines,
substitute an interpreter, prewarm fixtures or treat diagnostic success as repair.
All original four startup conditions and required platform/gate evidence remain
mandatory. No next official task is selected.

Evidence: [capture](pctx01-startup-live-stage-result.json),
[original outcomes](pctx01-startup-live-stage-forward-result.json),
[fixture-level join](pctx01-startup-live-stage-join.json),
[cleanup/consumption](pctx01-startup-live-stage-consumed.json),
[scope](pctx01-startup-live-stage-scope-plan.json).
