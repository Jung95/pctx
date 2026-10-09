# Paired authentication retry result — 2026-10-09

PCTX01-G05-D05 remains `product_failure`,33/40 detail conditions,0/10 whole gates.
The human reported being absent during the preceding attempt and explicitly
requested this retry. Prior failure receipts were preserved; only pre-test
readiness allowance changed180s to300s. No original deadline/concurrency changed.

The root collector42466 executed and wrote a terminal receipt with AssertionError
before publishing ready. Wrapper42454 exited0 and was reaped. No original test
started, no sampler launched, no raw directory created, no samples retained.
The coordinator's `collector_terminal:false` is a bookkeeping limitation: it
exits its readiness loop before setting that field when the wrapper ends. The
root-owned terminal receipt and later audit establish this collector terminated;
the raw coordinator receipt remains unchanged. Authentication UI behavior was
not observed, and wrapper success does not imply test or diagnostic success.

A subsequent exact-name, minimal identity audit finds one syspolicyd candidate665
with the expected executable path (ordinary-user BSD record unreadable) and
zero XprotectService candidates. This establishes endpoint absence only at the
audit time. The generic root AssertionError does not establish which admission
assertion failed earlier. Do not retroactively attribute it solely to absence.

Both generated wrapper and collector PIDs are absent. The unused ordinary-owned
prepared source/mailbox was removed after checking the root receipt, no test
consumed marker, no raw directory and unchanged product/test/lock hashes. No
service launch, service restart, security setting change or product patch occurred.
The shared target directory still contains the prior diagnostic executable;
normal-source rebuild is required before any future qualification.

Next remains G05-D05: add precise bounded admission-stage receipts, then design
observer readiness around the transient AnalysisService lifecycle without
warming/creating the endpoint or changing original test conditions. Do not run
another unchanged batch automatically. Separate diagnostic success from repair
and verify all original acceptance conditions before closing this task.

Evidence: [raw coordinator](pctx01-startup-pair-stack-retry-result.json),
[cleanup and current identity audit](pctx01-startup-pair-stack-retry-audit.json),
[consumed receipt](pctx01-startup-pair-stack-retry-consumed.json),
[scope](pctx01-startup-pair-stack-retry-scope-plan.json).
