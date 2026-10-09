# Bundle-label validation and operation finding — 2026-10-09

PCTX01-G05-D05 remains `product_failure`,33/40 details,0/10 whole gates.
Fixed bundle label mapping successfully admits the actual XprotectService image:
213 numeric frames retained (88syspolicyd,125XprotectService),no raw data or symbols.
The diagnostic reducer defect is verified corrected for this batch,not product
startup delay. Original1000ms/8×16 test exits101:98 outcomes,96OUTPUT,
2TIMEOUT44251/44252;30 of128 planned calls censored,no outer watchdog.

Both installed binary UUIDs match retained sampled image UUIDs. Syspolicyd frame
0x5b40c follows installed call0x10005b408 dispatch_semaphore_wait; dispatch/kernel
frames below it support service-level semaphore waiting. Its separate0x5b3b0
branch carries PolicyScanner work. XprotectService assessment-handler chain
reaches XPMalwareEvaluation initialization:0xc9f0 follows yr_compiler_add_string,
and0xcce8 follows yr_compiler_get_rules. The initializer also calls
 yr_compiler_create before those sites. This establishes observed rule-compilation
and rule-object preparation activity,not just nearest-method speculation or
proof of file-matching work. Static calls/selected disassembly are retained.

Syspolicy wait weight57 and XP initializer weight57 cannot identify a shared
request. Inclusive weights32/24 under compiler calls are not milliseconds or
CPU percentages. Frame reduction omits unallowlisted inner images and symbols;
compilation-call subtrees may contain their own waits. The2-second observation
extends beyond the original1-second deadline and overlaps multiple requests.
No timestamped per-request interval/hash join links this work to either expired
child or partitions the historical185ms scan. A PCTX patch is not justified by
this service-level evidence alone. Independent passive review confirms these
findings and request-binding limitations.

Initial ad-hoc static otool parser misread relative IMP fields; its invalid map
was discarded before interpretation. Existing relative-pointer decoder plus
exact call-site disassembly replaced it. Initial thin-only UUID parsing rejected
universal MachO; arm64 slice selected and both UUIDs then verified. Neither
attempt executed service code or changed system settings. Selected static strings
include compiler-create error and scan-error messages; string presence does not
prove a successful-operation interval event. No new live log scope was used.

Root terminal receipt successful;both samplers exit0/reaped,driver/authwrapper
exit0/reaped.104 known generated PIDs absent;raw deletion confirmed by receipt
and private source/mailbox removed. Known absence is not universal descendant
proof. Product/test/lock unchanged; shared target diagnostic,normal-source rebuild
before qualification. One finite scope consumed,no automatic generic-stack retry,
no Actions/security change/service restart/product repair.

Next remains D05: establish a timestamped XP compiler/preparation interval bound
to an owned failed evaluation/path,or vendor-confirmed applicable correction.
Review installed request identifiers/phase-observation mechanisms statically
before any new live collection. Another aggregated pair adds insufficient
request attribution. Preserve all original acceptance/platform requirements;
no next official task until full completion.

[Operation findings](pctx01-startup-bundle-stack-operation-findings.json),
[original outcomes](pctx01-startup-bundle-stack-forward-result.json),
[successful collector](pctx01-startup-bundle-stack-result.json),
[UUID validation](pctx01-startup-bundle-stack-static-uuid.json),
[static XP call map](pctx01-startup-bundle-stack-xpc-calls.json),
[cleanup](pctx01-startup-bundle-stack-cleanup-audit.json).
