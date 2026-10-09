# Lifecycle-aware stack diagnostic result — 2026-10-09

PCTX01-G05-D05 remains `product_failure`,33/40 detail conditions,0/10 gates.
No product/platform repair or full acceptance closure is established.

The lifecycle-aware observer became ready after admitting syspolicyd665. It
admitted owned parent43004/child43018 with an11.559ms trigger-to-admission gap,
then found one exact XprotectService candidate on its first endpoint query before
the original deadline. No endpoint launch/warmup or renewed product deadline was
performed. Unlike the preceding attempt,the original test actually ran.

Original1000ms/8×16 concurrency test exited101 with98 observed outcomes:
96OUTPUT,2TIMEOUT (43019,43025),30 of128 planned calls censored after failure.
Both failed children had no stdout/stderr or root-exit observation near the
original deadline; native snapshots report running0 and one thread. This is
consistent with pre-execution waiting but does not alone identify policy scan
work or exclude other causes. Parent timing and service sampling perturbation
remain. This run cannot qualify product repair even if another later run passes.

Samplers43027/43028 each exited0 and were reaped. The reducer then raised
ValueError for XprotectService at `sample_reduce`. Both samples were discarded;
no retained UUID/code-offset/CPU record is available for causal interpretation.
No raw directory is retained. Previous parser fixed exception strings were not
exported by the collector; exact header/image/frame refusal reason is unknown.
Discarded raw cannot be reconstructed or retried offline. Do not infer a format
mismatch or loosen identity/image checks without evidence.

Collector43000 wrote a validated terminal receipt. Driver43003 and authentication
wrapper42959 exited0 and were reaped. Audit confirms104 known generated PIDs
absent; unused temporary source/mailbox removed. This is not universal descendant
proof. Product/test/lock hashes remain unchanged. Shared target contains an
instrumented diagnostic executable; normal-source rebuild before qualification.
The inherited scope-plan limit saying both services must exist at readiness is
stale: the executable collector and current identity_admission explicitly defer
XprotectService until after trigger. This erratum preserves the executed plan
rather than silently rewriting evidence.

Next parser candidate uses fixed ParseFailure codes for raw cap,source headers,
boundaries,image admission/range and frame admission/cap. Collector candidate
exports only those enumerated codes,never raw exception text. Synthetic refusal
and privacy controls pass. They do not identify this prior failure or authorize
recovery of deleted raw. No further live batch is automatically repeated.
Next within D05: review these candidates and use bounded reason-preserving
collection to obtain interpretable operation evidence; keep original acceptance
and cleanup obligations unchanged. No next official task or Actions run.

[Original test result](pctx01-startup-lifecycle-stack-forward-result.json),
[collector/coordinator receipt](pctx01-startup-lifecycle-stack-result.json),
[admission](pctx01-startup-lifecycle-stack-admission-receipt.json),
[cleanup](pctx01-startup-lifecycle-stack-cleanup-audit.json),
[parser candidate control](pctx01-startup-lifecycle-stack-parser-next-control.json),
[preparation and independent review](pctx01-startup-lifecycle-stack-preparation.md).
