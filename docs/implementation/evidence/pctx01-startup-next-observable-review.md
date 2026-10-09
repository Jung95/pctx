# PCTX01-G05-D05: next observable review

Baseline25e5c91. Static file analysis and owned-file hash timing only. No original
test, service/log query, administrator authentication, framework invocation,
security change or vendor message. Product status stays failure,33/40,0/10.

## Findings

1. The GK scan-complete payload's two unsigned values and integer are obtained
   from deepPolicyMatch (0x10005b438), topLevelPolicyMatch (0x10005b444), and xpResult
   (0x10005b454). They are result fields, not three phase durations. Collecting
   these omitted values would not solve elapsed-time attribution.
2. GatekeeperScan emits begin0x10005ad58 and end0x10005b638 with the same stored ID.
   It encloses the scan, including policy work and applicable XP wait. ID-based
   correlation can distinguish evaluations, but this interval alone cannot split
   overlapping internal work. The Path signpost may contain a path, so no new
   signpost query is performed under the consumed stage-message scope.
3. timeIntervalSinceDate0x10005b6cc feeds scan telemetry submission0x10005b6f4.
   It is an elapsed whole-scan candidate, not a PolicyScanner/XP partition. No
   analytics database or unrelated application data is opened.
4. XprotectFramework's installed __TEXT is present in the arm64e shared cache,
   though the ordinary framework executable path is absent. The map/native
   mapping table locate78,336 bytes in dyld_shared_cache_arm64e.01, SHA256
   9190424e931205f6d2818a1b5c701131c2a645622ea9f241f47f007552819de7.
   Static code references separate match/rule/sync queue names, the message
   that interpreter scanning requires in-process scans, and the message that
   async in-process scans are unsupported. Text also contains an AnalysisService
   endpoint and performAnalysisOnFile...withReply...forcedRulesOnly selector.
   This is a new internal branch lead. It does not prove an out-of-process scan,
   which branch ran, which service owns the endpoint, or its cost in the failure.
5. Initial entry-relative method-name decoding failed. Apple's objc_opt_t version16
   defines a shared selector base offset; using it resolves170 code-range method
   entries. Thirteen candidates outside the executable code range are excluded; their
   protocol/class ownership is not established. Selector slots
   use validated cache slide version5 and34-bit runtime offsets. The static
   reproducer verifies these formats and the extracted text hash.
6. beginAnalysisWithResultsHandler:feedback: is at0x1c35b0184. Its entry tests
   byte0x42; the decoded requireInProcess getter reads the same byte. When true,
   async scanning is refused and returns false. The admitted async path builds a
   block at0x1c35b07b0; that block calls remoteObjectProxy at0x1c35b07cc, then
   performAnalysisOnFile...withReply... at0x1c35b07f4. Thus this static path crosses
   an IPC boundary, rather than containing all analysis inside syspolicyd.
7. The installed XprotectService.xpc Info.plist declares the exact
   com.apple.XprotectFramework.AnalysisService identifier. Its executable is
   /System/Library/PrivateFrameworks/XprotectFramework.framework/Versions/A/
   XPCServices/XprotectService.xpc/Contents/MacOS/XprotectService, SHA256
   d8cf6a1ac365a6c81ab695a6ec62ed914df54926497e8ea1b43315265ac7ba8c.
   This identifies an installed endpoint, not a live PID/start identity. No
   endpoint was invoked, sampled or logged. It does not establish the failed
   child's executed branch or request/queue/scanning/reply cost.
8. Temporary extracted binaries/disassembly were removed. Saved selected
   disassembly and numeric maps are static code only; no cached code executed.

The previous driver's4.7-second start-to-first-owned-child interval cannot be
assigned to hashing or policy from current evidence. One warm-cache static read
measures the13,963,456-byte test binary hash at8.733ms; other input hashes are
sub-ms. This does not reproduce cold startup or the removed build-log/source
preparation, and does not identify the previous gap's cause. Do not describe it
as confirmed preflight overhead or fixed coverage.

## Next within this same task

Continue static analysis of the identified bundled analysis implementation: map
request admission, scan dispatch and reply construction, checking for inner
timing/identity boundaries. No new live-service access is authorized by this
static mapping or by prior consumed scopes.
A whole-scan callback/log pair or another unchanged original test would add no
required phase attribution, so neither is executed. Any new live operational
collection must name exact targets, new observable, privacy limits and duration;
the consumed scope is not transferable to the named analysis endpoint.

Apple documents signpost IDs as distinguishing otherwise identically named
intervals/events, not inferring hidden subintervals:
[Recording Performance Data](https://developer.apple.com/documentation/os/recording-performance-data).
Static endpoint strings are not a supported product API or repair recommendation.
The original four startup conditions and all release-platform gates stay mandatory.

Evidence: [static map](pctx01-startup-xprotect-static-observables.json),
[owned-file timing](pctx01-startup-next-observable-preflight.json),
[prior call map](pctx01-startup-scan-cost-static.json).

Format references: [Apple Objective-C shared-cache definitions](https://github.com/apple-oss-distributions/dyld/blob/main/include/objc-shared-cache.h),
[Apple chained pointer definitions](https://github.com/apple-oss-distributions/dyld/blob/main/include/mach-o/fixup-chains.h).
[Reproducer](pctx01-startup-xprotect-map.py.txt) and
[selected async disassembly](pctx01-startup-xprotect-async-disassembly.txt).

Independent passive review confirms the async branch/calls and preserves
failed-child and runtime-cost uncertainty. No product completion claimed.
