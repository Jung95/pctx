# CLI output formats

The renderer in `src/render.rs` provides `Compact`, `Json`, and `Markdown`. The CLI entry point applies it and checks final serialized budgets, including the newline. Process-level fixtures verify Markdown source reads, secret masking, unsupported-format rejection before initialization, and budget failure.

`compact` and `json` produce one minified standard JSON envelope followed by exactly one newline. They preserve the same fields and values. Success, partial results, errors and empty results all remain single documents; no progress text or diagnostics are mixed into the document. JSON control escapes are reversible, and Unicode text remains valid UTF-8. Terminal-sensitive C1 and bidirectional formatting characters are escaped without changing their parsed values.

Transport exceptions are explicit: `adapter claude event --hook` emits native hook output on success, while board/activity NDJSON emits event records. `pack create --output` writes the source artifact and leaves its response on stdout; streaming routes reject output-file options. Root and nested help state standard envelope schema 1.0 and each command's read/write/execute effects, including conditional flags. `--no-color` applies before help parsing, and parser diagnostics redact secrets and escape unsafe control characters.

`markdown` is supported for `build`, `outline`, `read`, and `handoff` (including create, update and show). Unsupported commands reject this format through common argument admission before response-file writes, project access, watch/follow dispatch or child launch. The refusal is one JSON envelope on stdout even when `--output` was supplied. NDJSON streaming is a separate CLI contract and is not implemented by this renderer.

Source-independent argument rejection uses one JSON error envelope on stdout even when compact or Markdown was requested. This includes parsed semantic options, invalid finite-query timeout/watch combinations and insufficient error-envelope capacity; rejected requests never write `--output`. Capacity validation remains exit 2 and precedes semantic validation. Parser failures use masked stderr unless JSON was explicitly requested. Failed delivery of a plain parser diagnostic returns I/O exit 7; ordinary delivered parser errors retain exit 2. Unsupported NDJSON uses its existing error-event stderr contract. These pre-execution transports are distinct from source-dependent command errors rendered after admission.

These examples use the supported format names:

```sh
pctx --format compact read src/main.rs --lines 1:20
pctx --format json outline src/main.rs
pctx --format markdown read src/main.rs --lines 1:20
pctx --format markdown build --task 'Inspect the CLI' --seed src/main.rs
pctx --format markdown handoff show session-note --validate
```

Markdown displays request status, validation, coverage, warnings and errors separately from source evidence. It includes available relative paths, source hashes, ranges, representation, parser status, freshness, omission indicators and follow-up references. Byte ranges keep their reported zero-based, exclusive-end semantics; line ranges keep their reported one-based inclusive semantics. Missing fields are not invented. An unsupported parser with no returned symbols is not represented as a successful structural analysis.

Source and handoff bodies are literal fenced content. The fence delimiter is longer than every backtick and tilde run in the content, so embedded Markdown cannot close it. Terminal control characters and invisible direction formatting appear as explicit escapes in the readable display. Already masked text remains masked; the renderer does not reread source, resolve references, execute examples or perform a second redaction pass.

A complete escaped JSON envelope appendix preserves every supplied field and exact source string, including fields that have no prose presentation. Parsing the appendix reconstructs the original envelope. Reported failed checks, partial content, truncation and errors remain visible; rendering never establishes test success, task completion or source freshness.

The CLI checks the byte budget **after** rendering the final document, including its newline. Markdown source display plus the full JSON appendix can be larger than JSON output. If the document exceeds the budget, the CLI emits a complete JSON `BUDGET_TOO_SMALL` error (exit 8). Execution artifacts retain their reread handle and child outcome in this fallback, and delivered bytes are recorded. A budget smaller than the complete error envelope cannot contain a valid error document; this failure is not represented as successful bounded content. Source strings and JSON documents are never cut silently to fit.


## Minimum valid output budgets

A byte budget smaller than one complete newline-terminated JSON error envelope is an invalid argument (`INVALID_ARGUMENT`, exit 2). It is rejected before project loading, index refresh, check/lease mutation, context emission or child execution. The rejection is one JSON document on stdout, including when Markdown or `--output` was requested; invalid arguments never create the requested output file. This necessary rejection document can exceed the invalid requested limit.

The guard derives a command-specific conservative bound from the real serializer, envelope keys, fixed error message and `data.minimum_budget_bytes`. It reserves a full nanosecond RFC3339 timestamp and includes the final newline. The returned `minimum_budget_bytes` is authoritative if a future schema changes these sizes; this is not an arbitrary global execution threshold.

| Command | Current minimum valid stdout capacity | Separate execution/selection limit |
| --- | --- | --- |
| `build` | 515 bytes | Required task/rules must fit the actual selected document. |
| `context get` | 517 bytes | Required full task/rules/references must fit before recording an emission. |
| `extract` | 517 bytes | Actual selected source and provenance must fit. |
| `run` | 513 bytes | Existing supervision metadata admission requires at least 3,000 bytes. |
| `runner check-run` | 516 bytes | Existing registered evidence admission requires at least 8,192 bytes. |
| `check run` (positional key or `--key`) | 515 bytes | Same registered check backend and 8,192-byte admission. |
| `runner helper-request` | 516 bytes | Helper evidence uses its registered execution backend and configured admission. |
| `read` | 514 bytes for the error envelope | Uses a fixed 65,536-byte output bound; there is no `--budget-bytes` argument. |

A budget equal to the minimum passes this argument guard. It can still produce `BUDGET_TOO_SMALL` (exit 8) when required context or execution metadata cannot fit. For example, a mandatory rule exceeding a valid 2,000-byte build budget fails with exit 8; no partial required-rule success is fabricated. These are different errors from an invalid document capacity.

`output show`, `output find`, and `output render` do not declare a caller-selected stdout byte-budget flag. `pack plan --budget-bytes` governs the produced pack artifact capacity, including its own archive minimum and part limits; it is not this stdout document budget. No new flag is silently invented for these commands.

If an already executed command's presentation exceeds its valid budget, the JSON fallback preserves its durable output reread handle and observed child outcome. A wrapper budget failure does not establish child success or discard a saved failure artifact. Final document size always includes its newline.

## Indexed metadata search coverage

Path, symbol and document search return the pinned index generation used for the query. Every matching candidate receives current physical authorization and the selected freshness validation before ranking and LIMIT. Nonmatching metadata is not opened; scanned_files is therefore null and coverage.physical_non_candidates_checked is false. These fields avoid disclosing historical counts of deleted or inaccessible indexed files. Text/all lookup keeps the physically filtered snapshot and reports that check separately. Candidate-universe freshness and strict refresh incompleteness retain their existing coverage meanings.

Build selection measures the actual chosen JSON or Markdown renderer, including terminal escapes, source fences, metadata and the final newline. Verified excerpts carry original byte/line ranges; multiline signature boundaries come from grammar body fields, with explicit outline fallback when unavailable. Optional representations can downgrade through signature, outline, reference and omission; required task/rules and applicable decisions remain intact. The used-byte counter must stabilize within three passes, otherwise optional selection is reduced and measured again. Omission details may become an explicit count/reason summary when their own overhead cannot fit. No exact tokenizer is currently registered; token-budget requests return CAPABILITY_UNAVAILABLE.

ContextGet and Build now share the adaptive selection engine. ContextGet measures its actual full/delta JSON packet, including terminal escapes, plan/receipt metadata, source invalidations, byte counters and newline. It restricts optional code candidates to the requested scope (or task scope), resolves directory scopes to files, and preserves applicable mandatory rules/decisions and task metadata. A packet can contain full spans, verified signatures, outlines or references; `selection_plan` reports omissions explicitly. A required packet that cannot fit returns BUDGET_TOO_SMALL before any emission/ack/event is recorded.

Serializer `adaptive-context-v6` binds the chosen representation, original source hash/ranges, masked delivered-body hash, parser set and omission plan. The receipt ledger stores these descriptors, not source bodies or task text. Existing v1/v2/v3/v4/v5 records remain stored but cannot authorize a v6 delta; obtain and explicitly acknowledge a new full packet. Printing a packet does not acknowledge it.

A delta requires an acknowledged baseline in the same session, epoch, task, policy, scope and serializer. Selection measures only the bodies actually added/changed in that delta. If a previously acknowledged signature is upgraded to a full span, the unreceived body is sent explicitly. An omitted source is absent from the received selection; subsequent delivery is `added`. A previously selected source that remains present but is omitted from the new plan appears in `invalidated` with `no_longer_selected`, while actual disappearance appears in `removed`. Budget omissions cannot masquerade as source deletion or acknowledged bodies.

Index refresh/selection/measurement hold no receipt writer transaction. Before publication, the receipt transaction rechecks session epoch/status, baseline authorization/selection, selected source hashes, task revisions and final capacity under the original deadline. Duplicate emission/ack reuses its existing receipt without another event. Duplicate ack preserves the originally recorded provenance and reports `receipt_reused`; acknowledgement proves receipt, not understanding.

The application selection interface `context::select_with_measurement` accepts a consumer that measures the complete rendered packet. Build uses its actual JSON/Markdown renderer; ContextGet uses the scope-restricted variant of the same engine and its full/delta receipt renderer. Selection checks the original deadline after consumer measurement, propagates delivery failures and revalidates selected sources. Native regressions cover escaped transport overhead, controlled source replacement during delivery preparation, original expiry, exact capacity, preserved mandatory content, omitted-source delivery and representation-sensitive explicit acknowledgement. Full §43 ranking/span selection, all invalidation variants and supported-platform qualification remain tracked separately.

## Decision and memory validity

Git cleanliness does not establish unchanged project guidance. Required rules, decision claims and scoped retrospective instructions carry current source hashes, and the document snapshot fingerprint appears in `source_versions.project_documents_hash` and the receipt selection plan. Selection revalidates that snapshot before output, including supersession dependencies omitted from the packet.

Only an `accepted` decision without an effective replacement is delivered as a current mandatory claim. Proposed decisions remain proposal references. Superseded, deprecated, rejected, cancelled and completed decisions are historical references, with `current_guidance: false`, source ID/hash, scope, supersession links and validity basis. An accepted predecessor with an effective replacement is also historical even when its own bytes remain unchanged. These references omit the instruction body and grant no permission or verified implementation authority. Explicitly seeding a retired decision does not turn its body into current guidance.

`superseded_by` reports declared links; `effective_superseded_by` excludes proposed/rejected replacements. A remaining terminal replacement link prevents implicit predecessor reactivation when the replacement is cancelled or completed. This classification uses current document records; durable lineage after link-file deletion/restoration is not yet qualified.

An acknowledged current decision becoming historical emits `invalidated` with `decision_no_longer_current`, along with its updated reference if selected. Actual decision/instruction deletion emits a removal tombstone. Changes to legacy scoped instructions are delivered as project data, not new authority. Role/topic delivery restrictions, handoff/task-memory variants and the full invalidation matrix remain separately tracked.

Observed retirement relationships persist as metadata in the existing append-only control event ledger, keyed by project and workspace. Removing a replacement document, clearing disposable caches or starting a new session does not reactivate its predecessor. Observations contain source IDs, relative replacement path/hash, declared status and policy fingerprint, never document bodies or authority. Queries may persist a validated observation even if later packet delivery fails; this is distinct from context emission and acknowledgement. Control backup/restore preserves history while retaining its existing authority revocation and workspace-remapping requirements.

An accepted predecessor may explicitly declare `reinstates: replacement-id` (or an array of IDs). Every ID must name a retirement observed in this workspace; unknown IDs, self references and declarations on non-accepted documents fail. A still-present accepted replacement continues to retire the predecessor. Restoration retains `observed_superseded_by` as provenance and grants no permission. History outside its bound current policy conservatively prevents reactivation, including explicit reinstatement without revealing hidden replacement IDs; `retirement_history_outside_current_policy` identifies that uncertainty. This is a minimal explicit source-claim restoration contract, not proof of current implementation behavior. Corrupt or future-version observations fail instead of promoting guidance. History reads/writes are bounded and follow the original request deadline.

Decision history SQL now transfers one validated TEXT row at a time, using a4096-byte BLOB-length sentinel before Rust payload materialization; Unicode character counts cannot weaken the byte limit. Non-TEXT or oversized observations fail with a bounded DB_ERROR and no new relationship publication. The8MiB aggregate serialized-history cap remains a metadata bound, not a native SQLite/RSS guarantee.

## Consumer-bound delivery controls

`context get --session SESSION --topic TOPIC` binds the registered consumer and exact declared topic before inventory or ranking. `build --session SESSION --topic TOPIC` uses the same read-only control helper; a `--role` ranking hint cannot override registered roles. Agent callers need a registered consumer session; unbound local owner inspection remains subject to wildcard and explicitly selected role/topic controls. These checks grant no host or operation permission.

A paused consumer/registered role returns ROLE_PAUSED; an applicable exact topic silence returns TOPIC_SILENCED. Active applicable silence with no topic returns DELIVERY_TOPIC_REQUIRED. Errors contain no source path/body or controlled topic label, and suppressed requests create no context emission or acknowledgement. Incomplete or over-limit controls fail closed. The original query deadline applies throughout.

The opaque delivery fingerprint binds recipient, topic and relevant registered-role controls into receipt selection metadata and baseline compatibility. Durable role-policy event sequence prevents same-second pause/resume from restoring an old baseline. Existing events identify role only, so a policy mutation conservatively invalidates other topics/recipients sharing that role. Controls are checked again before Build output and inside the ContextGet receipt writer transaction. Resume permits a fresh full packet; it never restores earlier acknowledgement eligibility. Serializer6/selector5 preserve older records while requiring a fresh full acknowledged baseline.

This implements declared packet-topic barriers. Per-item source-topic attribution, owner exception priority, handoff/task/approval memory integration, agent-bound Pack consumer plumbing and supported-platform qualification remain required; a caller's topic declaration is not proof of source content classification.


Common `--root`, `--format`, `--output`, `--timeout-ms` and `--no-color` options accept one explicit occurrence across the selected command path. They may appear before the command group, between group and leaf, or after the leaf. Repeating a singular option is an argument error even when its values are identical; defaults are not explicit occurrences. Repeatable producer options such as `--scope` keep their own contracts, and arguments after the child delimiter `--` remain literal child arguments.

`pack create` requires exactly one `--output PATH` at any command depth. It names the source artifact, while the standard response stays on stdout. Missing or duplicate artifact paths are rejected before effects. Root `--version`/`-V` prints the compiled CLI package version as plain text with exit0 before project or response-file access; it remains distinct from JSON schema1.0. These local argument proofs do not qualify every downstream timeout phase or native Linux/Windows execution.


Failed primary delivery returns I/O exit7 for help/version, native hook output, stream-error diagnostics and response-file failure diagnostics. Native hook and NDJSON use the same reversible JSON control escaping without adding a standard envelope to their protocol objects. Compact stream columns escape hidden controls and show newline/tab as literal escapes, preserving row boundaries.

An accounting warning follows an already completed result write. Its delivery is best effort: a closed diagnostic stream cannot panic or change the established command outcome, and metering remains unconfirmed. The retained output and original child truth remain available; warning failure never reruns a command. Current pipe evidence is native Unix; Windows delivery remains unqualified.
