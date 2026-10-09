# macOS26.7.1(25G241): fresh script startup waits in AppleSystemPolicy beyond application deadline

Status: local draft only; no report submitted; approved broad trace collected privately and deleted after numeric reduction.

## Current send preparation

A minimized four-file ZIP and report are prepared in `pctx01-startup-platform-report.zip`
and `pctx01-startup-platform-report.md`; contents/hashes are in the payload manifest.
This longer historical draft is not the proposed attachment. Independent review
confirmed numeric/source consistency and data exclusions. Proposed recipient is
Apple Feedback Assistant web, exact payload only; external-send authorization remains
pending. No native app launch or automatic sysdiagnose.

Installed XProtect data5366/remediator163 read from fixed component Info.plists.
A bounded read-only macOS catalog lists27.0.1 build26A5434; no install/download
requested. Reviewed AppleScript workaround and public update notes establish no
matching fix for the owned kernel script-evaluation wait. See
`pctx01-startup-platform-correction-review.md` for source-specific limits.

## Reproduction and expected behavior

Project: PCTX, https://github.com/Jung95/pctx, original source baseline`37e8d0e`.
Host: macOS26.7.1 build25G241, native arm64. SIP enabled.
Original integration test:
`concurrent_query_startup_preserves_group_streams_and_original_one_second_budget`
in `tests/project_deadline.rs`. Each of eight workers creates one fresh shebang script, then plans16 executions
of that same path via native OS spawning.
Every query has one1000ms deadline and must preserve child PGID=PID and collect
both stdout/stderr. The generated fixture checks its process group and writes
fixed synthetic output. The application deadline is our product contract, not an
assertion of a macOS universal startup guarantee.

Build from normal repository sources with the locked dependencies. A direct
original-test command is:

```sh
cargo test --locked --test project_deadline concurrent_query_startup_preserves_group_streams_and_original_one_second_budget -- --exact --nocapture --test-threads=1
```

This command is a reproduction instruction, not a completed new run. Our diagnostic
instrumentation preserves the original test bytes/deadline and separately records
native phase timings; it is not a qualification run or standalone OS reproducer.

## Observed evidence

Latest once-approved System Trace: recorder0, original1000ms/8x16 test101 with
three reported failed roots49083/49084/49086. Each exact owned thread has a long
Blocked interval1000.066625–1000.103125ms and a context-switch-associated kernel
stack through AppleSystemPolicy::evaluateScript →waitForEvaluation →
ASPEvaluationManager::waitOnEvaluation →lck_mtx_sleep. Stack samples occur just
after blocked intervals end; cancellation may interrupt the wait, not complete
assessment. Direct child-policy wait is established; internal service request ID,
compiler duration and upstream cause remain unknown. All source/test pins unchanged.
The fixture is a fresh#!/bin/sh script, not a Mach-O executable. The parent spawn
observation rounds below1ms; no output/exit observed before deadline. Raw trace and
exports removed; minimized numeric result is attached locally.


Historical preserved-condition diagnostic:2026-10-09T20:00:37.844719–20:00:43.892494UTC,
98 recorded outcomes:96OUTPUT,2TIMEOUT(PID45676/45677),30 planned outcomes censored.
The Rust test-body time was1.05s; enclosing driver time6.047775s. Failed call spawn
phases0.911/0.850ms, total entry-to-terminal999.933/999.930ms, no dropped trace points.
This establishes unresolved startup outcomes, not their internal service cause.

Earlier request-bound syspolicyd evidence for a different failed fixture found
scan start913.425ms after birth and final result98.881–98.883ms after its deadline.
Separate service-level samples reached YARA compiler-add-string/get-rules and
syspolicyd semaphore wait. These do not bind compiler frames to either latest
failed request or partition its duration. Sampling weights are not elapsed time.

Installed XprotectService UUID`57DD6284-09FD-351F-A87A-F60ECFB75F70`, SHA256
`d8cf6a1ac365a6c81ab695a6ec62ed914df54926497e8ea1b43315265ac7ba8c`.
Its inspected whole-assessment signposts share an ID but use a default `%@` URL
payload. One approved exact-owned-URL stored query returned no rows. An independent
own NSURL control showed default payload redaction and public-payload visibility;
actual service privacy/emission state remains unknown. No private-log activation,
security exception, SIP change, service restart or persistent privileged tool was used.

## Questions for platform engineering

1. What supported diagnostic can correlate a generated owned executable's public
file digest or process identity to its XProtect assessment without collecting
other applications' private data?
2. Can that diagnostic identify compiler preparation versus rule scanning and
request queue residence with timestamped boundaries while preserving the original
1000ms/8×16 conditions?
3. Is there a supported platform correction for repeated assessment preparation
or queue delay on this build? No policy bypass, fixture warmup, extended timeout,
interpreter substitution or retry-to-pass is acceptable for this reproduction.

## Minimal evidence to review

- `pctx01-startup-child-trace-review.md`: exact-owned child policy-wait evidence and limits.
- `pctx01-startup-child-trace-numeric.json`: minimized PID/TID/UUID/offset/state observations.
- `pctx01-startup-xpc-signpost-result-review.md`: historical failure and association limits.
- `pctx01-startup-xpc-signpost-forward-result.json`: owned timing/outcome records.
- `pctx01-startup-policy-forward-joined.json`: earlier owned request/policy timing.
- `pctx01-startup-bundle-stack-review.md`: numeric service-frame evidence and limits.
- `pctx01-startup-xpc-public-binding-review.md`: public identifier candidate decisions.

These existing artifacts contain minimized numeric/synthetic evidence. No entire
log archive, private URL map, user files, systemwide trace or sysdiagnose is included.
Apple identifies [Feedback Assistant](https://developer.apple.com/feedback-assistant/)
as its bug-reporting route. Any submission or additional collection needs an
explicitly agreed recipient/data scope; this draft does not perform either.
