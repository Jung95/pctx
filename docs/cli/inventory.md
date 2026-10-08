# Static project inventory and document drift

```sh
pctx inventory scan --max-files 2000 --max-bytes 2097152
pctx inventory profile --path .pctx/profile.json
pctx inventory audit --registry .pctx/docs.json --since event-1
```

These are read-only application methods. Scripts, JavaScript/TypeScript configuration, workflow commands, model providers and package managers are never executed. Files use the same current exclusion, symlink, encoding and freshness checks as the shared reader. Inventory reports actual content hashes and inferred check **candidates**, not successful checks, registered runner permissions or release exemptions. Profiles and document registries are project-local inputs; neither can authenticate an owner or grant operations.

The static subset includes npm package names, versions, dependency declarations, scripts and workspace patterns; pnpm workspace patterns; Cargo packages, members, dependency declarations and configuration aliases; GitHub Actions job/step metadata; strict JSON TypeScript aliases and static app configuration; Markdown headings and root/nested AGENTS/CLAUDE/ADR document hashes. Supported Python/JS/JSX/TS/TSX code exports are syntax metadata only. Rust, Swift, SQL and Terraform sources retain path/hash metadata with structure coverage marked unknown. CI environment values, credentials, source bodies and native dialogue are omitted. Native rule loading is unknown.

Workspace matches and reverse dependencies are candidates based on observed package declarations. Exclusion patterns, Cargo inherited versions, configuration inheritance, malformed manifests, JSON-with-comments, dynamic CI expressions and executable configuration produce explicit partial coverage. No configuration is imported to discover its runtime value. Scripts containing recognized check tools suggest parser/resource candidates, but an unregistered check stays unregistered. Missing required full-project gates cannot be waived by package scope suggestions.

`--max-files` bounds attempted candidate reads; `--max-bytes` bounds selected source text. The shared reader independently enforces its configured per-file bound and checks mutations during each read. The path enumeration itself uses the shared walker; this module does not provide an independent bounded directory walker. Limits and omissions are explicit. The observations are individually verified; this is not an atomic filesystem snapshot. Source or policy changes alter the inventory fingerprint.

## Profile observation schema

```json
{
  "schema_version": 1,
  "id": "my-project",
  "expectations": [{
    "source": "package.json",
    "selector": "/engines/node",
    "expected": ">=24",
    "confirmation": {
      "source": "AGENTS.md",
      "hash": "SHA256_OF_CURRENT_RULE_DOCUMENT",
      "authority": "owner-provided"
    }
  }],
  "roles": [{
    "id": "web-worker",
    "scope": ["packages/web/**"],
    "confirmation": {
      "source": "AGENTS.md",
      "hash": "SHA256_OF_CURRENT_RULE_DOCUMENT",
      "authority": "owner-provided"
    }
  }]
}
```

A matching expectation plus current hash-bound evidence is `confirmed` as an **observed claim**. A different observed value is `conflicting`; missing, stale or unsupported evidence is `unconfirmed`. The `authority` string remains a declaration: `owner_authority_verified`, `owner_identity_verified`, `operation_authorized` and `profile_applied` remain false. Role scopes show currently visible matching paths; confirmation does not turn a rule document or role claim into permission. Local owner trust/capability binding is a separate application contract.

Selectors support JSON pointers into static JSON/TOML/YAML, `$path`, `$hash` and `$symbol/NAME` (supported Tree-sitter names/kinds, never literal matches). Unsupported selectors and sensitive or overly large selected values return explicit unknown/error reasons. No semantic interpretation of legal, UX or business prose is performed.

## Document source reference schema

```json
{
  "schema_version": 1,
  "since_event": "event-1",
  "known_proposal_ids": [],
  "source_refs": [{
    "document": "README.md",
    "section": "Runtime",
    "source": "package.json",
    "selector": "/engines/node",
    "expected": ">=22",
    "rendered": "Requires Node >=22.",
    "template": "Requires Node {value}.",
    "source_hash": "SHA256_OF_REGISTERED_SOURCE_BASELINE",
    "document_hash": "SHA256_OF_REGISTERED_DOCUMENT_BASELINE",
    "decision_id": "DEC-runtime"
  }]
}
```

Audit distinguishes `changed_sources`, `affected_documents`, `verified_mismatch` and `review_needed`. A verified proposal requires a current document hash matching its explicit baseline, an unambiguous ATX heading section, one exact occurrence of the claimed rendered sentence, agreement between the old value and template, and a different observed scalar source value. Before/after snippets carry current source, document, registry and policy hashes. Non-scalar, dynamic, changed-but-still-matching or ambiguous claims need review rather than a fabricated patch.

`DOC-` proposal IDs are deterministic hashes of the actual correction and current evidence. Repeated audits return the same ID; changed evidence changes it. `known_proposal_ids` marks only a registry claim, not a verified existing task. Decision IDs are declared links, not automatic decision authority. `since_event` binds the explicit registry baseline; it does not claim replay of the control event history. Passing a different `--since` fails with `BASELINE_MISMATCH`.

The proposed task intent is reviewable metadata only. Audit does not edit the document, create a task, apply the profile, upgrade a dependency or publish anything. Any later task creation/application must revalidate current hashes, preserve permissions, and deduplicate against real task records.

Manifest references: [npm package.json](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/), [Cargo manifest](https://doc.rust-lang.org/cargo/reference/manifest.html), [pnpm workspace settings](https://pnpm.io/settings), [GitHub Actions workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax). Format support here is the explicit static subset above, not a claim to implement those tools' evaluators.
