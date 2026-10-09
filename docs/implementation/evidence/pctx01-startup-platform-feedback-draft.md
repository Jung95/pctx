# macOS26.7.1(25G241): native fixture startup exceeds application deadline; request supported XProtect attribution

Status: local draft only; no report submitted and no broad diagnostics collected.

## Reproduction and expected behavior

Project: PCTX, https://github.com/Jung95/pctx, evidence baseline`d3bde8a`.
Host: macOS26.7.1 build25G241, native arm64. SIP enabled.
Original integration test:
`concurrent_query_startup_preserves_group_streams_and_original_one_second_budget`
in `tests/project_deadline.rs`. Eight workers each plan16 native fixture executions.
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

Latest preserved-condition diagnostic:2026-10-09T20:00:37.844719–20:00:43.892494UTC,
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

- `pctx01-startup-xpc-signpost-result-review.md`: latest failure and association limits.
- `pctx01-startup-xpc-signpost-forward-result.json`: owned timing/outcome records.
- `pctx01-startup-policy-forward-joined.json`: earlier owned request/policy timing.
- `pctx01-startup-bundle-stack-review.md`: numeric service-frame evidence and limits.
- `pctx01-startup-xpc-public-binding-review.md`: public identifier candidate decisions.

These existing artifacts contain minimized numeric/synthetic evidence. No entire
log archive, private URL map, user files, systemwide trace or sysdiagnose is included.
Apple identifies [Feedback Assistant](https://developer.apple.com/feedback-assistant/)
as its bug-reporting route. Any submission or additional collection needs an
explicitly agreed recipient/data scope; this draft does not perform either.
