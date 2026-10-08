# Query deadlines

The current development source starts one monotonic budget after argument validation and before project discovery. Supported finite queries default to 10 seconds: `find`, `query`, `extract`, `graph`, `outline`, `read`, `changes`, `status`, `doctor`, `repo`, `cache`, finite `board`, finite `activity`, `handoff show`, `output show/find/render`, and existing `savings report/opportunities`. `index`, `build` and `checkpoint` default to 120 seconds.

```sh
pctx --root /path/to/project --timeout-ms 3000 --format json find auth
pctx --root /path/to/project --timeout-ms 120000 build --task-file task.txt --seed src/auth.ts --budget-bytes 12000
```

An explicit positive `--timeout-ms` replaces the request budget. Project loading, database locks, source reads, parsing, nested refresh and response preparation consume the same budget. Strict `find`, `query` and `outline` do not start a new 120-second indexing budget. Registered child execution has a separate execution timeout; this query option is rejected on unsupported routes and on `board --watch` or `activity --follow`.

Expiry without usable results returns `TIMEOUT`, exit 7. Search can return already authorized and hash-validated matches with exit 3, reason `timeout` and an unknown omitted count. A candidate scan cap is a separate partial reason. Expired preparation cannot publish an incomplete replacement generation or checkpoint. The two-second body-search performance target is separate from the execution deadline and remains unmet in the latest historical measurement.

Task documents are limited to 1 MiB and valid UTF-8. Unix stdin polling shares the request deadline and restores its original flags; task-file FIFOs are rejected without waiting for a writer. Windows pipe and disk stdin are supported in source, while console stdin returns an explicit capability error. Native Windows verification remains pending. Stdin access requires exclusive ownership.

Local Git/ps observations use bounded stream collection and cancellation, including inherited pipe EOF after root exit. Git status declines repositories with configured clean/process filter drivers rather than executing them or changing conversion semantics. The preflight is not atomic against concurrent configuration edits; complete side-effect isolation remains unfinished.

Filesystem calls, console output and some native operations are cooperative: checks bound phases but do not guarantee interruption of an individual stalled system call. Full finite-command coverage and platform qualification remain tracked in [requirements](../implementation/requirements.md), PCTX68/AX20 and PCTX69/AX21. This is not a full release deadline guarantee.

After a successful output write, delivery accounting reuses the original project and budget. If measurement cannot be confirmed, the delivered outcome is preserved and stderr emits `OUTPUT_MEASUREMENT_UNRECORDED` with `measurement_recorded: "unknown"`. Failed writes never record delivery. This is not proof of provider receipt, token usage or cost.

Native macOS CLI regressions verify exact newline-inclusive byte accounting and a complete output write followed by accounting timeout when the reader stalls past the original budget. The latter demonstrates cooperative pipe behavior; it does not guarantee hard cancellation of a blocked stdout write.
