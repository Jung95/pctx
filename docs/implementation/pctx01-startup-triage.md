# PCTX01 startup failure: consolidated triage

PCTX01-G05-D05 remains an unresolved product/contract failure. Detail closure is
33/40; whole gates0/10. This record consolidates existing evidence, not a new task,
requirement, test run or environment exemption. All prior failures remain valid.

## Frozen acceptance and source boundary

Resolve the four original `tests/project_deadline.rs` startup tests with the
original one-second query deadline,8 workers×16 queries,owned process group,
independent streams and overflow assertions. Preserve native executable resolution,
child cwd/environment,retained identity and cancellation. No warmup,budget extension,
interpreter substitution,security-policy bypass or retry-to-pass can satisfy it.

The historical [source reconciliation](evidence/pctx01-startup-policy-ownership-reconciliation.json)
found no evidenced PCTX correction:the current supervisor supplies the original
Deadline before launch and while observing streams/group/exit. Its deadline failure
reports are truthful. This does not establish that PCTX has no defect.

## Evidence and its limits

| Measurement | Recorded result | Meaning |
| --- | --- | --- |
| One-auth owned-target capture5159 | Original concurrency98outcomes:96success,2TIMEOUT | Actual expired17810/17812 sampled in script-policy wait;instrumented diagnostic,not qualification |
| Forward ordinary-user comparison | Declared open3PASS/1FAIL→closed4PASS | Host-associated difference,confounded by order/time |
| Partial reverse | Declared closed4PASS;open not run | Input-language interruption;incomplete comparison retained |
| Recovery reverse | Declared closed3PASS/1FAIL→open3PASS/1FAIL | Closing app is not a sufficient observed cure;second phase is not universally passing |

[Exact kernel evidence](evidence/pctx01-startup-auth-all-four-capture-analysis.json)
and [raw trace](evidence/pctx01-startup-auth-all-four-capture-stage-2-trace.txt) bind
UID,parent,native start and unique PID for17810/17812. Both show
`AppleSystemPolicy::procNotifyExecComplete → evaluateScript → waitForEvaluation`
and evaluation-manager waiting. First observed samples are about294ms after native
fork,with millisecond wall-clock precision/stability limits. Child report endings
near cancellation do not prove all142samples pre-cancel or a continuous whole1s wait.
The reason for delayed evaluation remains unknown;no operational daemon was observed.

[Forward](evidence/pctx01-startup-host-control-comparison-verification.json),
[partial reverse](evidence/pctx01-startup-host-reverse-comparison-verification.json)
and [recovery reverse](evidence/pctx01-startup-host-reverse-recovery-comparison-verification.json)
remain separate. Product sources match;binary is identical within each comparison,
but the reverse binary differs from the forward build. Host absence is human-declared,
not process-inventory verified. Global policy/cache/contention,time,remaining helpers
and app-reopening effects remain confounded. Failed concurrency runs are censored;
no full attempt denominator or comparative failure rate is available.

## Corrective boundary and resumption

No applicable supported correction was found in the bounded
[official-documentation check](evidence/pctx01-startup-host-correction-docs-check.json).
That absence is not proof no correction exists. The historical startup evidence does not justify a causal startup patch or a
host/security change. The later final-return repair below addresses a separate
proved deadline defect. Do not repeat consumed commands,
known-wait kernel capture or unchanged host controls. All generated collectors and
comparison binaries are removed;raw evidence remains. No live heavy job.

Resume corrective implementation only with evidence identifying a specific causal
PCTX operation,or a documented applicable supported platform correction. Validate
any correction against the unchanged original tests and integration candidate;
a later isolated pass cannot erase retained failures. Additional observation of
operational services or OS-policy changes exceeds the current owned-target approval.

Required native macOSarm64 qualification still fails. macOSx64,Linuxx64 and
Windowsx64 remain unverified;supporting Linuxarm64 is no substitute. G10 integration
and final closure remain open. No other official task is activated.

[Consolidation hashes](evidence/pctx01-startup-consolidated-triage.json) bind this
record to the inspected source and evidence inputs.

## Apple policy documentation check

The [bounded primary-source check](evidence/pctx01-startup-apple-policy-docs-check.json)
fetched Apple's [Terminal and script protections](https://support.apple.com/en-ie/guide/security/sece3b202c4b/web).
For macOS26.4+,the document describes independent protections including descendant
behavior inspection after pasted terminal commands. This explains why changing UI
app presence alone is not a comprehensive control of documented process-tree policy
inputs,but does not attribute these timeouts to that mechanism. The AppleScript/JXA
section is not proof about the shell-script fixtures. No matching stack explanation,
latency guarantee or applicable correction was identified. Another launch-constraint
page was not readable enough to support a correction claim. No new test,process
observation,authentication or policy/provenance manipulation occurred.

## Exact Rust source review and separate final-return repair

[Source review](evidence/pctx01-startup-rust-spawn-source-check.json) compares the
installed Rust commit with PCTX launch configuration. Process-group0 is supported
by the posix_spawn path; no custom pre_exec or forced suspension exists here. This
is static evidence, not a runtime backend trace or a policy-delay explanation. No
applicable supported platform correction was established by the bounded search.

[Final-return correction](evidence/pctx01-query-final-deadline-verification.json)
adds the missing original-deadline check after real root reap and before success.
Its controlled native-boundary regression fails before and passes after correction.
Released identity prevents signaling after reap. Original startup tests remain
unchanged; integration11PASS/3FAIL still reports zero-output pre-exit timeouts,
not this late-return defect. G05-D05 remains open;33/40details,0/10gates.
