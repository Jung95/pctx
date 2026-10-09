# System Trace attempt and raised output threshold

PCTX01-G05-D05 remains product_failure. Whole PCTX01 totals33/40 details,0/10
gates unchanged. Original four startup conditions remain mandatory.

The user approved one7second System Trace with broad raw collection explicitly
disclosed,ordinary OSLog excluded and original1000ms/8×16 test unchanged.
The normal-source binary hash matched the8.67s restoration receipt; no source
instrumentation or product/test change was added for this trace.

## Actual first attempt

The recorder emitted a completed-recording marker, but saving triggered the
64MiB logical-size stop threshold at observed400,006,733bytes. Recorder exit-9;
this is a **failed diagnostic**, not a validated trace. Temporary logical size
overshoot was disclosed before execution. Final retained private size before
removal was approximately32MiB; this is compatible with temporary buffer or save
activity but does not establish which caused the transient growth.

Original test root47358 exited101 and was reaped by wrapper47357. One reported
query47364 timed out at elapsed1000ms with0stdout/0stderr and no observed root
exit. Test-body time1.08s; wrapper interval6.1709555s. The recorder perturbs timing;
no new causal assignment to policy/YARA or product repair follows.

A bounded TOC export of the existing saved trace exited10 and yielded no usable
table schemas. No new recording was performed by that export. A completed-marker
line does not override recorder exit-9 or failed export.
[Recorder result](pctx01-startup-instruments-system-trace-result.json),
[export result](pctx01-startup-instruments-system-trace-toc-result.json).

Known recorder/wrapper/test-root/reported-failed-query/export PIDs were absent.
The private raw trace,recorder/test logs and target metadata were removed.
This audit does not enumerate every successful descendant. Raw system metadata
was never committed or uploaded; no other application's attribution retained.
No sudo,private-log activation,security change or persistent privileged service.
Instruments authorization state was not separately inspected.
[Cleanup](pctx01-startup-instruments-system-trace-cleanup.json).

## User-requested threshold change

The user explicitly requested raising the logical-size threshold. It is now
**1GiB(1,073,741,824bytes)**, read from the plan. Recording duration remains7s and
original test deadline/concurrency unchanged. This remains a100ms-polled soft
threshold:overshoot and Instruments-managed cache outside output root are not
strict byte-capped. No second recording has occurred.

The first executed plan/source are frozen separately; the reconstructed source
hash matches the first plan exactly. The first consumed marker is preserved.
Retry preparation uses a different private root and distinct consumed/result
receipts. Human retry approval is pending because the original approved count
was one. Independent passive review found no blocker in the revised preparation;
missing receipts or unknown descendant cleanup remain unknown.
[Executed plan](pctx01-startup-instruments-system-trace-executed-plan.json),
[retry plan](pctx01-startup-instruments-system-trace-plan.json).

Next remains D05: if separately approved,one retry with higher output capacity to
obtain a valid saved scheduling/syscall timeline. This changes capture capacity,
not the test or acceptance criterion. No other official task or Actions run.
