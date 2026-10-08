# PCTX01 R04–R06 independent review

Reviewer: /root/work_control, read-only final diff review. No edits or builds.
Scope: src/work.rs, src/session.rs, src/quota.rs, tests/control_preflight.rs.
No blocker found: existing predicate semantics, receipt/lease/order/session epoch,
owner/input contracts and supplied-original-expiry priority are preserved.
New tests cover all five report stages at4096 bytes with replay data/state, actual
1024-byte session reasons, quota key256 ingestion/replay/non-owner denial and six
direct invalid/expired no-effects cases. Both Ack provenances are grammar tests,
not new actual acknowledgement qualification.51PASS is parent-run native evidence.
Whole gates/platform qualification remain open. Parent checked terminal logs and
five actual controlled-loss test failures with exact source restoration.
