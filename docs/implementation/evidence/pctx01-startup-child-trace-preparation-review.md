# PCTX01-G05-D05 — child-inclusive trace preparation, 2026-10-09

Only D05 is active. Completion still requires correction and all four original startup contracts; PCTX01 remains33/40 details and0/10 gates. No product source or original test has changed.

## Different hypothesis and concrete command

The prior successful recording supplied global numeric kernel events but detailed wait/syscall/Time Profiler tables contained only the launch wrapper. Extending its duration does not repair target selection. The installed xctrace help explicitly supports `record --all-processes` and `--notify-tracing-started`. The prepared coordinator uses:

```
xctrace record --template 'System Trace' --all-processes --time-limit 7s \
  --output PRIVATE_ROOT/capture.trace --recording-options OPTIONS_JSON \
  --notify-tracing-started UNIQUE_OWNED_NAME
```

A unique Darwin FD listener registers before recorder launch. Only the matching4-byte big-endian token permits one invocation of the unchanged original test. Missing readiness, recorder early exit or30s readiness limit prevents test launch. The recorder has60s wall limit; private output soft stop1GiB checked50ms. The original launcher's root watchdog35s and coordinator40s receipt wait remain independent. OSLog exclusion and kernel/wait/context-switch sampling options are preserved. No `--no-prompt`, security change, service restart, persistent administrator tool or password handling.

Readiness is not proof that stacks for every child were captured or that the whole test fits inside7s. Actual process identity, time coverage, missing/dropped events and stacks must be assessed after recording; unknowns stay unknown. All-process mode is a supported selector, not yet demonstrated on this workload.

## Scope and approval boundary

All-process Time Profiler can collect other apps' user/kernel stacks and symbols as well as process/path/VM/file/scheduler metadata. Prior approval was once-only and detailed tables were wrapper-targeted; this wider stack collection needs fresh explicit approval. Recording is blocked before marker/listener startup until `human_scope_approval=approved`. Preparation review is approved, execution approval pending. Raw output stays mode0700, no commit/upload. Analyze/retain only identity-bound owned test processes and previously approved syspolicyd/XprotectService numeric data; discard unrelated rows/frames. Instruments-managed external cache and transient output overshoot are not strictly capped by the coordinator. Real system authentication may still be required.

## Controls and review

The installed notify.h warns polling registrations can produce false positives; the coordinator therefore uses token-matched FD delivery. Owned notification-only control registered/post/read/cancelled successfully; no recording, product test or service observation. Polling registration was refused in sandbox; first FD control attempted a redundant close after cancellation. The corrected control confirms libnotify cancellation owns/closes this unique FD; no repeated close by numeric descriptor.

Independent code review found and resolved three issues: cleanup exception isolation, explicit recorder/wrapper terminal uncertainty, and possible exception-path leakage. Six mocked coordinator cases passed: normal one launch, missing notification, recorder early exit, listener cleanup error, wrapper wait error and recorder cleanup error. These are tool controls, not product tests; fake processes/notifications never invoke xctrace. Initial mock lacked subprocess constants, failed before launch, and was corrected before the reported six-case result. Syntax, token mismatch/short-frame rejection, scope gate, file/source pins and absence of consumed marker/trace are checked. Known PID audit/private raw deletion remains obligatory after any real capture; unknown descendant enumeration cannot be claimed complete.

## Resume

Prepared plan: [scope and bounds](pctx01-startup-child-trace-plan.json). Runner defaults to preparation only. Empty private directory is retained, no live job, no capture/count consumed. Next only within D05: obtain explicit approval for this once-only7s all-process stack collection, run once, reduce exact-owned data, audit cleanup, then determine whether a cause or applicable correction is supported. No other official task starts.
