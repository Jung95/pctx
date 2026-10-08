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
