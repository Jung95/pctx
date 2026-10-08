# Owner-bound restrictions and reporting exceptions

Role pause prevents claims, model wake and scheduled execution. Topic silence prevents delivery to its configured recipient. A reporting exception changes only a typed message's delivery decision; it never releases the underlying restriction or restores execution, host permission, leases or acknowledgements. Build, ContextGet and Pack have no implicit reporting category and remain subject to their existing delivery checks.

## Owner identity and legacy records

A newly authored restriction receives an immutable ID, original owner principal, revision and hashed input evidence. `role list` returns these owner-management records. `role pause`/`role resume` preserve their existing flags; only the original principal can change an existing restriction, including an inactive one. Resuming a nonexistent restriction is a no-op. A policy change invalidates earlier delivery fingerprints.

`PCTX_OWNER_PRINCIPAL` identifies the trusted local operator (default `owner`). The transport requires `PCTX_ACTOR=owner` with no nonempty run identity/capability. This is a local trusted-input boundary, not OS authentication or proof of a shared GitHub account's human identity. Host permission remains separate. Agent statements, message urgency and ordinary decisions do not establish original-owner authority.

Legacy restrictions retain `original_owner: null` and cannot be silently reassigned or released. After reviewing original policy evidence, the trusted operator may explicitly attest ownership with `policy attest-owner --from-file attestation.json`:

```json
{"schema_version":1,"id":"RESTRICTION_ID","expected_revision":1,"original_owner":"OWNER_PRINCIPAL","owner_evidence":"reference to original owner input"}
```

`policy release --from-file release.json` uses the same ID/revision/evidence fields, without `original_owner`. Wrong-owner and stale-revision operations fail without changing policy. Ordinary approvals cannot replace these commands.

## Explicit reporting rule

First prepare the exact message JSON with `reporting: {"category":"security","source_paths":["src/check.rs"]}`. The message type must match its category: `approval_prompt`, `permission_incident`, `deadline` or `security`. The body and optional metadata are redacted and normalized through the shared message schema. `policy report-fingerprint --from-file report.json` returns its opaque `payload_hash` and `bound_projection: message-v1`. The hash covers the complete delivered sender-controlled projection, including body, references, correlation/idempotency keys, priority, expiry, revision and reporting claims. A changed projection requires a separately authored exception.

The original owner records a rule with `policy exception-record --from-file exception.json`:

```json
{
  "schema_version": 1,
  "restriction_refs": [{"id":"RESTRICTION_ID","revision":1,"precedence":"exception"}],
  "role": "legal",
  "recipient": "CANONICAL_RECIPIENT",
  "topic": "incident",
  "source_scope": ["src/**"],
  "payload_hash": "HASH_FROM_REPORT_FINGERPRINT",
  "category": "security",
  "not_before": 1791440000,
  "expires_at": 1791440600,
  "reason": "explicitly scoped reporting rule",
  "owner_evidence": "reference to original owner input"
}
```

Replace IDs, hash and timestamps with actual values. Every conflicting restriction must be referenced at its current revision and owned by the author. Each reference's `precedence` is explicitly `exception` or `restriction`; missing precedence is an unresolved conflict. Core never infers urgency. Conflicting, undefined, stale, expired or revoked rules hold delivery with a bounded opaque policy status. Source patterns are relative; excluded source paths are refused. Source references are owner-reviewed attribution claims; this path does not fetch or verify source bodies or grant file access.

Rules have a finite validity interval of at most 30 days and an expiry at most 30 days after recording. Inputs are capped at 64 KiB, with up to 64 restriction references/source paths, 32 scope patterns and 4096 stored restrictions/rules. Owner evidence and reasons are stored as hashes. `policy exception-revoke --from-file revoke.json` requires ID, expected revision and owner evidence. `policy report-evaluate --from-file evaluation.json` is owner inspection of role/recipient/topic/category/source_paths/payload_hash, never an execution grant.

Actual Inbox read/ack derives the recipient and registered roles from the session, evaluates current controls under the request deadline, and binds the rule fingerprint in delivered metadata. All registered roles and canonical agent/name identities remain relevant. Both canonical agent and exact session destinations are checked; an authored rule must cover each applicable delivery scope. Ordinary messages remain withheld. A revoked or expired exception cannot be acknowledged, and a reporting exception leaves an underlying role pause active. No model is woken.

Operations use one cooperative 10-second default request budget and support `--timeout-ms`. Input reads use the shared bounded, nonblocking regular-file reader, enforce the 64 KiB protocol limit and do not leave workers behind. Native filesystem calls remain cooperative; final checks do not establish an atomic check-to-output guarantee.

Control backups retain original-owner metadata and historical rules. Restore invalidates active reporting exceptions and increments their revisions, preserves pause/silence, reinstalls immutable identity/revision/no-delete protections and invalidates ordinary grants/receipts. Legacy schemas remain readable with unknown ownership; unsupported or incomplete policy schemas are rejected. Native Windows/Linux, broader automatic memory delivery and authenticated external owner transports remain separate qualification work.
