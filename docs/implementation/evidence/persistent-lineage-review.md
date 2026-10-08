# D040 independent review

Read-only agent requirements reviewed documents/context/session and direct/CLI fixtures. No files edited or tests run by the reviewer; runtime results are separate.

Initial blocker: explicit reinstatement treated absence from the policy-filtered inventory as deletion, allowing a hidden accepted replacement to reactivate its predecessor. Integration now requires a currently visible observation bound to the same policy before restoration. Direct and actual CLI fixtures combine reinstatement with replacement exclusion, retain historical references and omit both bodies. Follow-up review found no remaining functional blocker in these scoped contracts.

Defensive limitation: the8MiB aggregate cap bounds serialized valid-history metadata, not hard RSS. SELECT currently materializes one subject's rows before validating payload size. Valid writer rows bound that temporary batch to about4MiB; corrupt oversized TEXT requires pre-materialization SQL size validation or bounded streaming. This remains a concrete follow-up, with a malformed/oversized-row regression required. Complete AC77 and full PCTX validation remain unfinished.
