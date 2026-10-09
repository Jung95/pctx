# PCTX01-G05-D05 — valid System Trace retry, 2026-10-09

The user approved one retry with a 1GiB logical output stop and 7 seconds (longer if necessary). The reviewed 7-second duration was retained. Recording exit0 and TOC export exit0 establish a readable trace. The original unchanged 1000ms/8x16 test exited101 with three reported TIMEOUT roots:47992,47994,47995. Trace overhead makes this diagnostic evidence, not qualification.

## Numeric result

Exact failed-PID process-info rows supply distinct trace-local numeric identities and one TID per failed PID. Unfiltered kernel scheduler tables contain 39/39/38 owned events. MACH_BLOCK(0x0f) to next owned MACH_DISPATCH(0x20) spans are 999.330584,999.345250,999.428500ms respectively. Their arithmetic and limitations were independently reviewed. These observations support a nearly whole-budget block-to-dispatch gap; they do not establish continuous blocked time, a wait-channel, policy request binding, compiler duration, or exclude memory pressure. Wakeups can be emitted by another process. Event times are trace-relative nanoseconds; process-info event-time0 is not claimed birth time.

The detailed thread-state/context-switch/syscall/ThreadActivity/thread-snapshot tables target only the launch wrapper47986. They contain no failed-child wait stacks. Longer collection alone does not correct this target-selection limitation. The OSLog export contains0rows, despite an os-log schema in the TOC.

## Export and cleanup

Sandboxed detail exports aborted because Instruments Analysis Core could not create its temporary cache (NSCocoaErrorDomain513/NSPOSIX1). Exporting the existing trace with tool permission succeeded; no additional recording or test occurred. Raw metadata and XML stayed in the mode0700 private directory. Only selected failed-child numeric events/identities and minimized receipts were retained. All22 known recorder/wrapper/root/failed-child/exporter PIDs were absent; private raw output (314483374 logical bytes including XML exports) was removed. This is not a universal descendant proof; successful child identities were not all retained. External Instruments caches were not inspected/deleted.

## Status and next action

D05 remains product_failure; PCTX01 stays33/40 details and0/10 gates. No product/test/lock change, security change, service restart, Actions execution, or task switch. The once-only recording count is consumed. A future trace must first correct target selection to include owned native child wait stacks, with a concrete bounded plan and any required collection approval; repeating or lengthening the wrapper-only trace is not justified. Completion still requires repair and all four original startup contracts, including PGID, streams, overflow, cancellation and reap, on required platforms.

Evidence: [numeric timeline](pctx01-startup-instruments-system-trace-retry-numeric.json), [recording](pctx01-startup-instruments-system-trace-retry-result.json), [TOC](pctx01-startup-instruments-system-trace-retry-toc.json), [executed plan](pctx01-startup-instruments-system-trace-retry-executed-plan.json), [cleanup](pctx01-startup-instruments-system-trace-retry-cleanup.json). First failed attempt evidence is preserved separately.
