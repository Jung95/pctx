# Query deadlines

The current development source starts one monotonic budget after argument validation and before project discovery. Supported finite queries default to 10 seconds: `find`, `query`, `extract`, `graph`, `outline`, `read`, `changes`, `status`, `doctor`, `repo`, `cache`, finite `board`, finite `activity`, `handoff show`, `output show/find/render`, existing `savings report/opportunities`, all `session` operations, `context get/ack`, `pack plan/create/inspect/verify`, and local policy/mailbox operations. Finite Work reads (`task list/show`, `task complete --dry-run`, `agent list/show`, `check list/show/plan`) and quota `report/plan/reconcile` also share this budget. Resource status, runner check/resource/helper status, trust plan, inert filter Validate/Apply/Explain, adapter Claude Doctor/Verify/ProtocolFixture, Schedule List/Plan/Inspect and Inventory Scan/Profile/Audit also receive the same query budget. `index`, `build` and `checkpoint` default to 120 seconds.

```sh
pctx --root /path/to/project --timeout-ms 3000 --format json find auth
pctx --root /path/to/project --timeout-ms 120000 build --task-file task.txt --seed src/auth.ts --budget-bytes 12000
```

Source-independent argument grammar is checked before discovery, stdin consumption, index refresh or response-file creation. Shared validators cover Find/Query, Read selector/line combinations, Outline scopes/freshness, Extract locations/selectors, Build representations/limits/task-input conflicts, Graph bounds/directions, Handoff names and Inventory limits. Read positional and named path aliases conflict explicitly. Source line upper bounds, missing symbols, artifact-derived locations and source permissions remain producer checks; a valid request is not proof that its sources exist or are authorized.

An explicit positive `--timeout-ms` replaces the request budget. Project loading, database locks, source reads, parsing, nested refresh and response preparation consume the same budget. Strict `find`, `query` and `outline` do not start a new 120-second indexing budget. Registered child execution has a separate execution timeout; this query option is rejected on unsupported routes and on `board --watch` or `activity --follow`.

Expiry without usable results returns `TIMEOUT`, exit 7. Search can return already authorized and hash-validated matches with exit 3, reason `timeout` and an unknown omitted count. A candidate scan cap is a separate partial reason. Expired preparation cannot publish an incomplete replacement generation or checkpoint. The two-second body-search performance target is separate from the execution deadline. The eleventh historical measurement observed p95 1739.805458ms on its immutable earlier binary; current-source performance and required hardware/cold-cache equivalence remain unqualified.

Task documents are limited to 1 MiB and valid UTF-8. Unix stdin polling shares the request deadline and restores its original flags; task-file FIFOs are rejected without waiting for a writer. Windows pipe and disk stdin are supported in source, while console stdin returns an explicit capability error. Native Windows verification remains pending. Stdin access requires exclusive ownership.

Local Git/ps observations use bounded stream collection and cancellation, including inherited pipe EOF after root exit. Git status declines repositories with configured clean/process filter drivers rather than executing them or changing conversion semantics. The preflight is not atomic against concurrent configuration edits; complete side-effect isolation remains unfinished.

Filesystem calls, console output and some native operations are cooperative: checks bound phases but do not guarantee interruption of an individual stalled system call. Full finite-command coverage and platform qualification remain tracked in [requirements](../implementation/requirements.md), PCTX68/AX20 and PCTX69/AX21. This is not a full release deadline guarantee.

After a successful output write, delivery accounting reuses the original project and budget. If measurement cannot be confirmed, the delivered outcome is preserved and stderr emits `OUTPUT_MEASUREMENT_UNRECORDED` with `measurement_recorded: "unknown"`. Failed writes never record delivery. This is not proof of provider receipt, token usage or cost.

Native macOS CLI regressions verify exact newline-inclusive byte accounting and a complete output write followed by accounting timeout when the reader stalls past the original budget. The latter demonstrates cooperative pipe behavior; it does not guarantee hard cancellation of a blocked stdout write.

Session/context library calls also receive a 10-second budget when their project has no outer deadline; an existing deadline is retained. Selection, source revalidation and receipt/session transaction commits check that same budget. Native macOS tests verify that an already expired request cannot attach, change an epoch, suspend/reconcile, emit context or acknowledge it; CLI writer contention verifies no session/receipt/event/capsule changes and no renewed five-second wait. Lock admission may report `INDEX_BUSY` immediately before expiry; actual expiry reports `TIMEOUT`. An individual SQLite commit or filesystem call remains cooperative, so this does not promise rollback of an already completed commit if response preparation later expires.

Pack library entry supplies the same 10-second default when no deadline exists. Selected Pack builds keep that outer budget. Inventory/planned-source expiry returns TIMEOUT rather than a stale-plan error. A controlled fixture expires after inventory admission and verifies that Create publishes no output or parent. Delivery controls are checked after source reconstruction and immediately before Unix publication; this remains cooperative across native calls.

Work/quota read phases recompute SQLite busy waits from the original remaining budget, interrupt long SQL and check row/parsing phases. `check plan` propagates that same deadline into registered auxiliary workspace discovery and fingerprint reads. Task writes, registered check execution and continuous watch/follow lifetimes retain separate contracts. Resource, runner/trust read plans, inert filters, adapter probes, schedules and inventory now have their own targeted local deadline evidence. Broader phase-race and native Linux/Windows qualification remain incomplete; they are not implied by any one command subset.

Handoff create/update reuse the bounded explicit regular-file reader, preserving the existing 1MiB input cap and original internal request clock before checkpoint creation. `--from-file -` names a literal file in this route. Create/update do not acquire query `--timeout-ms` support; Show remains the finite-query route. Explicit source encoding or file failures precede checkpoints, while source-dependent error output keeps the normal command transport.

Checkpoint Create checks its optional nonempty display name (at most 256 UTF-8 bytes, without detected secret patterns) and project-relative glob scopes before project access. Names may contain spaces; scope `.` covers all allowed files. Absolute, backslash, parent-traversal and malformed glob scopes return `INVALID_ARGUMENT` (exit 2) before checkpoint or response-file changes.

Context Get checks mode, rejects `--since` in full mode, and validates explicit
scopes before project/session storage access. Explicit scopes must be normalized
relative paths: empty, absolute, backslash, empty components, `.` and `..`
components return `INVALID_ARGUMENT` (exit 2). Context scopes differ from
Checkpoint glob scopes. Valid requests still require an authorized project
session. CLI capacity admission runs first; an expired library request retains
its original timeout before argument validation.

Repo Status checks `--workspace current` and comma-separated fields
`branch,dirty,head,counts` before project discovery. Empty or unknown entries,
whitespace and other workspaces return `INVALID_ARGUMENT` (exit 2) without writing
a response file. Duplicate valid fields remain accepted. A workspace without Git
returns unsupported coverage and exit 6; supported observations remain exit 0.
The broker's data, permission isolation and refresh ownership remain unchanged.
