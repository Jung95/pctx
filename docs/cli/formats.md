# CLI output formats

The renderer in `src/render.rs` provides `Compact`, `Json`, and `Markdown`. The CLI entry point applies it and checks final serialized budgets, including the newline. Process-level fixtures verify Markdown source reads, secret masking, unsupported-format rejection before initialization, and budget failure.

`compact` and `json` produce one minified standard JSON envelope followed by exactly one newline. They preserve the same fields and values. Success, partial results, errors and empty results all remain single documents; no progress text or diagnostics are mixed into the document. JSON control escapes are reversible, and Unicode text remains valid UTF-8. Terminal-sensitive C1 and bidirectional formatting characters are escaped without changing their parsed values.

`markdown` is supported for `build`, `outline`, `read`, and `handoff` (including create, update and show). Unsupported commands must reject this format before execution, so a formatting error cannot occur after a write or child launch. NDJSON streaming is a separate CLI contract and is not implemented by this renderer.

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

Serializer `adaptive-context-v3` binds the chosen representation, original source hash/ranges, masked delivered-body hash, parser set and omission plan. The receipt ledger stores these descriptors, not source bodies or task text. Existing v1/v2 records remain stored but cannot authorize a v3 delta; obtain and explicitly acknowledge a new full packet. Printing a packet does not acknowledge it.

A delta requires an acknowledged baseline in the same session, epoch, task, policy, scope and serializer. Selection measures only the bodies actually added/changed in that delta. If a previously acknowledged signature is upgraded to a full span, the unreceived body is sent explicitly. An omitted source is absent from the received selection; subsequent delivery is `added`. A previously selected source that remains present but is omitted from the new plan appears in `invalidated` with `no_longer_selected`, while actual disappearance appears in `removed`. Budget omissions cannot masquerade as source deletion or acknowledged bodies.

Index refresh/selection/measurement hold no receipt writer transaction. Before publication, the receipt transaction rechecks session epoch/status, baseline authorization/selection, selected source hashes, task revisions and final capacity under the original deadline. Duplicate emission/ack reuses its existing receipt without another event. Duplicate ack preserves the originally recorded provenance and reports `receipt_reused`; acknowledgement proves receipt, not understanding.

The application selection interface `context::select_with_measurement` accepts a consumer that measures the complete rendered packet. Build uses its actual JSON/Markdown renderer; ContextGet uses the scope-restricted variant of the same engine and its full/delta receipt renderer. Selection checks the original deadline after consumer measurement, propagates delivery failures and revalidates selected sources. Native regressions cover escaped transport overhead, controlled source replacement during delivery preparation, original expiry, exact capacity, preserved mandatory content, omitted-source delivery and representation-sensitive explicit acknowledgement. Full §43 ranking/span selection, all invalidation variants and supported-platform qualification remain tracked separately.
