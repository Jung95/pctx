# PCTX01-G05-D05: internal scan path and platform correction review

Baseline77a7310. Installed macOS26.7.1/build25G241. syspolicyd SHA256 remains
f515a5958a5172c28bc19c787878e192b0c2a8f60ffad3975bf8ef1098baa84f.
Only static installed files, public Apple sources and one freshly approved
finite historical stage-log query were read. No test rerun, live sampling,
admin authentication, policy/service change, update or vendor submission.

## Confirmed static path

EvaluationManager serial workQueue invokes performScanWithArguments. That
method can launch XProtect work, run synchronous PolicyScanner, then wait for
launched XProtect completion before logging GK scan complete.

| Work | Installed call evidence |
| --- | --- |
| Policy/signature validation |0x10005b3ac calls PolicyScanner performScan:allowSecAssessment:doDeepTraversal:withProgressReporter:withCancellationHandler:withErrorSet:. Scanner calls SecStaticCodeCheckValidityWithErrors and signature/policy helpers. Actual branches/cost unknown.|
| Malware analysis |XPScanner at0x10006b978 constructs XProtectScan withURL, setsOnlyForcedRules, and calls beginAnalysisWithResultsHandler:feedback: at0x10006baac. Callback completion can precede or follow waiting.|
| Joined asynchronous completion |Successful upfront launch at0x10005b8f0 setsw20=1 and runs PolicyScanner. A launched-scan branch at0x10005b408 waits on its semaphore with DISPATCH_TIME_FOREVER. Forced post-policy launch0x10005ba50 can reach the same wait.|
| Completion |Normal XP callback logs GK Xprotect results before semaphore signal0x10005c020. GK scan complete occurs after PolicyScanner and any applicable XP wait.|

Independent /root/work_control review confirms these branches/selectors and
ordering. Extracted address ranges include associated blocks and cleanup paths;
they are not single uninterrupted method bodies. Static presence is not proof
that a particular branch ran for PID35071. No fixed100ms sleep, individual YARA
rule cost, network delay or filesystem bottleneck is established by this review.
[Reproducible call map](pctx01-startup-scan-cost-static.json).

## Finite stage read

Human approved one3-second historical interval12:43:38–12:43:41UTC,source665,
16exact owned fixture hashes, only GKscan-complete/XProtect-result/forced-result
messages. Exit0,raw2bytes(emptyJSON),0events; query37298 reaped/absent. Raw
memory discarded, no broad fallback or retry. Scope consumed. Missing stages
are inconclusive: not evidence that XProtect was skipped or completed quickly.
[Receipt](pctx01-startup-scan-stage-read-result.json).

Therefore this step identifies internal candidate work but cannot partition
the185.029ms directly joined interval. Even with stage records, upfront XP work
can overlap PolicyScanner; result callback timing alone would not assign all
elapsed time to an expensive XProtect rule. Previous direct evidence of a late
owned policy scan and accumulated serial scan delay remains unchanged.

## Applicable platform correction

No matching publicly documented correction is established for these fresh
65-byte shell scripts on this build. This does not prove no fix exists or that
all internal fixes are disclosed.

- Apple's26.7.1 update notes provide general bug/security wording. Its September
 28,2026 security bulletin describes a CoreGraphics bounds correction; neither
 establishes a script-scan latency fix. [Update notes](https://support.apple.com/en-kw/122868),
 [security bulletin](https://support.apple.com/en-mide/149228).
- Apple Developer Tools engineering confirms that long Gatekeeper scans can
 block other launches. The documented Xcode16.4/macOS15.6 corrections concern
 simulator dyld shared caches/runtime-image I/O; they do not establish a
 correction for this fresh-script workload. [Apple engineer discussion](https://developer.apple.com/forums/thread/807025).
- Apple's XProtect AppleScript/Xojo investigation concerns application-main-queue
 synchronous initialization, a different runtime and wait mechanism. Its
 preinitialization workaround does not preserve this task's conditions.
 [Apple DTS discussion](https://developer.apple.com/forums/thread/810258).
- XProtect signatures update independently from OS updates. Current static
 bundle metadata is XProtect.bundle5366 and XProtect.app163,CFBundleVersion1
 each. This identifies selected installed metadata only, not historical full
 ruleset, update eligibility or an outdated-component cause.
 [Apple Platform Security](https://support.apple.com/en-jo/guide/security/sec469d47bd8/web).

## Remaining work

PCTX01-G05-D05 stays product_failure; fixed33/40 details,0/10 whole gates.
A product patch is not justified yet. Precise elapsed-time attribution requires
an observable boundary for the owned scan's policy and XP execution, including
overlap and callback/wait timing. No retained-child workaround, service-wide
trace or previously consumed service collection may be reused. Prepare any new
service observation concretely and obtain its own scope approval before use.
An Apple report can ask about this exact serial path/build/component metadata;
the repository vendor-report draft is updated but not submitted. Any claimed
correction must satisfy all unchanged original tests and required platforms,
with no warmup, retry-to-pass, fixture change or security exception.
