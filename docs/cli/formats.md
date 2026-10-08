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
