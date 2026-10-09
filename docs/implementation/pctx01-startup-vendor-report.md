# macOS script startup: minimal evidence report

Draft for technical review; not submitted to any vendor. PCTX01-G05-D05 remains
an unresolved product acceptance failure. This report does not assign a vendor,
application or kernel root cause and does not request a security exemption.

## Contract and environment

Native macOS26.7.1 build25G241,Apple Silicon,ordinary-user execution,Rust1.99.0
commitb940084d7eb6a299eb4bfeb8e34901bc051e7ac4. Query startup must preserve its
original1000ms request deadline,8workers×16queries,owned process group,independent
stdout/stderr and output-overflow refusal. No warmup,budget extension,interpreter
replacement,security-policy change or retry can qualify a repair.

The public repository is https://github.com/Jung95/pctx. Original acceptance source
is tests/project_deadline.rs,hash8b9fbc97fbd5acaf612b2d75b94cf810faa2b61614b654495082462b1941a8dc.
The latest diagnostic product source is7181e657042c75553bacb6eeb77b5ec2c07e81bc.
Its separate final-return deadline repair does not address prepayload startup wait.

## Established observations

- A previous owned-child kernel capture binds actual expired children to native
  process identities and samples AppleSystemPolicy::procNotifyExecComplete →
  evaluateScript → waitForEvaluation → ASPEvaluationManager::waitOnEvaluation.
  It establishes observed early waiting,not a continuous whole1s state,exact signal
  ordering or the cause of delayed evaluation.
- Reverse host-presence comparisons retain failures in both human-declared app
  states. Closing the UI is not a sufficient observed cure;helpers,order,time,
  cache and other host effects remain confounded.
- A separately approved syspolicyd-only finite sample batch runs the four unchanged
  tests once each at ordinary UID. Three pass;concurrency fails with two zero-output
  original-deadline expiries before root exit or group probing. Service native
  identity/header checks pass,but sample attachment has a residual PID-reuse race.
- Service results contain aggregate IPC/semaphore waits and sparse signature
  functions. Most frame rows remain unknown after privacy reduction. CPU values
  are retained raw with a conditional Mach-timebase conversion. None identifies
  which evaluation belongs to an expired child or proves queue saturation/deadlock.
- SystemPython3.9.6 monotonic timestamps have process-local origins. Do not directly
  align collector/driver timestamps or claim exact sample/test overlap. Within-
  process durations and IPC ordering remain usable.
- One approved default-masked historical log query,limited to2sUTC and explicit
  references to the two expired owned PIDs,returns0events. Missing records are
  inconclusive;no broader query/private logging mode was used.

## Current correlation limitation

Static inspection of this installed policy-service binary finds separate PID,
evaluation ID and signpost ID. Its conditional Path and EvaluationID signposts
share the signpost ID; the prior PID-labelled query excluded this channel. This
is not evidence that events were emitted or retained. A new owned-fixture mapping
run preserves the original concurrency test and expires one child at the original
deadline. Mapping is instrumented and does not qualify a repair. A target-specific
finite signpost read is prepared,not executed,pending separate content-scope approval.
No vendor submission or security change follows this draft.

A later human-requested unmodified-product recheck again yields3PASS/1concurrency
FAIL with two zero-output original-deadline expiries.26aggregate memory samples
have normal published-XNU pressure flag1,unchanged swap use and zero swap-in/out/
pageout deltas. This does not support active swap thrashing in that interval,but
cannot exclude unsampled transients or establish a policy-service cause.

## Evidence and requested guidance

See [consolidated triage](pctx01-startup-triage.md),
[owned-child kernel analysis](evidence/pctx01-startup-auth-all-four-capture-analysis.json),
[service verification](evidence/pctx01-startup-service-verification.json),and
[historical target-log verification](evidence/pctx01-startup-target-log-verification.json).
Raw service samples were removed;published reductions omit arbitrary messages,
paths,addresses and unknown strings. The named tools and expired owned children in the
[service cleanup receipt](evidence/pctx01-startup-service-cleanup.json) are absent;
the audited generated directories listed there were removed. The
[historical query verification](evidence/pctx01-startup-target-log-verification.json)
also confirms its named reader/log processes are absent. No security
setting,service restart or persistent privileged component was introduced.

What supported target-specific interface can correlate a child awaiting script
policy evaluation with its evaluation request/completion without enabling private
logging,observing unrelated processes or changing security policy? Is there a
known applicable platform correction for delayed completion under this native
launch contract? A corrective proposal must preserve the original acceptance
conditions and be verified without the observer;diagnostic PASS alone is insufficient.
