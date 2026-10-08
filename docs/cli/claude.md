# Claude Code adapter

The adapter uses official command hook JSON and PCTX's session application API. It does not read Claude's transcripts, change native task/team directories, run a model, grant host permissions, or wrap arbitrary shell commands. `doctor` distinguishes installed CLI identity, documented events, offline fixture validation, and unknown live account support.

```sh
pctx adapter claude doctor
pctx adapter claude plan --agent AGENT_ID
pctx adapter claude install --plan PLAN_HASH --expect-hash PLAN_HASH
pctx adapter claude verify
pctx adapter claude protocol-fixture --from-file hook.json
pctx adapter claude event --agent AGENT_ID --from-file hook.json --idempotency-key receipt-1
```

Plan creation saves only hashes, identities, and PCTX additions in the private control directory. It reports the exact project-local `.claude/settings.local.json` additions. Installation is an explicit configuration action; it checks the plan hash and current settings hash, preserves other hooks, status line and permissions, and rejects symlink settings. No global Claude settings are changed. Current configuration must be quiescent while installing: the expected hash detects existing edits but external editors do not honor PCTX's private lock. Plan again after any edit. The bootstrap profile installs SessionStart, PreCompact, SessionEnd and the inert PreToolUse gate. PostCompact is documented but excluded because support in the local installed runtime has not been verified. Run `verify` and a controlled session after installation; configuration presence is not live transport evidence.

Installed command hooks use `--hook` and receive stdin. Successful native transport returns `{}` with no PCTX envelope or additional context. Explicit file imports return the regular PCTX metadata envelope. Unknown schemas return an adapter error, never an approval or a fabricated blocking decision. A hook cannot prove current runtime identity or delivery: event authority is recorded as an explicit local import. This bridge is advisory and is not a protected runner admission boundary.

SessionStart attaches a fresh PCTX session for startup/resume/clear. PreCompact changes the independent context epoch; a subsequent compact SessionStart reuses that boundary. An explicit receipt key makes retries idempotent; reusing a key with changed bytes fails. A process interrupted after recording an intent leaves a reconciliation-required receipt rather than repeating a session mutation. SessionEnd suspends the PCTX session. Permission and task events record observations only; they cannot grant permission or complete work. Printing stdout never acknowledges context. Obtain and explicitly acknowledge a full PCTX context packet after an epoch change. No transcript path, tool input, assistant message, compact summary, credential, or quota estimate is stored. Native rule-loading evidence and current account quota remain unknown.

Execute programs explicitly through `pctx run -- PROGRAM ARG...`; the shared runner preserves argv and child status. The adapter does not insert quiet/head flags, deduplicate executions, rerun commands for presentation, or apply a second output compressor. `--no-output-filter` is owned by the main PCTX output layer. Quota imports use the separately documented structured `quota ingest` API; status-line byte counts are not provider token/quota facts.

Managed scheduler execution receipts, OS registration and keep-awake integration are not supplied by this protocol bridge. Offline fixtures do not establish those capabilities. Live Claude compatibility, hook latency and account usage must be verified separately before claiming production integration.

Protocol references (checked 2026-10-08): [official hooks reference](https://code.claude.com/docs/en/hooks), [official CLI reference](https://code.claude.com/docs/en/cli-reference). The local development environment has no installed `claude` CLI; no account settings or model calls were used during implementation.

## Removing managed hooks

```sh
pctx adapter claude verify
pctx adapter claude uninstall --plan PLAN_HASH --expect-config-hash CURRENT_SETTINGS_HASH
```

The installation receipt records only hook groups newly added by that plan. An identical group that existed before installation is not owned. Uninstall removes exact owned groups, preserves subsequent user settings and all other hooks, and fails before publication if an owned group has been edited. It never removes global configuration. An interrupted installation requires reconciliation unless the complete current settings match its prepared publication hash. Adapter migrations are transactional (namespace schema version 2); restore must invalidate bindings and event receipts so a restored receipt cannot acknowledge a new epoch.

## Inert tool admission

The optional PreToolUse command hook does not execute, rewrite or reroute tools. Known read tools and a small exact set of read-only shell forms emit no decision. Dangerous direct commands return `hookSpecificOutput.permissionDecision: "deny"`; heavy, opaque or unknown commands return `"ask"`. This leaves independent host permissions in force. An untrusted `runner_receipt` field cannot grant admission. A successful observed process result is not a prospective runner admission token. Unknown shell forms, compound commands and scripts need explicit host review and the registered PCTX runner workflow; this bridge does not claim strong authorization over arbitrary shell execution. See [official PreToolUse decision schema](https://code.claude.com/docs/en/hooks#pretooluse-decision-control).

Pure PreToolUse checks parse bounded JSON without opening a DB or inventorying sources. PreCompact stores a small capsule of session epoch, current policy hash and explicit resume requirements; file inventory and full context selection are deferred to `context get`. Latency remains unmeasured. Owner imports and fixture validation do not establish trusted native transport. Non-owner runtime ownership requires the separate application session/run/capability contract; a native session string or prose does not confer it.

## Explicit status-line usage import

```sh
pctx adapter claude statusline --task-id TASK_ID --from-file statusline.json --pool POOL \
  --session PCTX_SESSION --epoch CURRENT_EPOCH --counter-epoch COUNTER_EPOCH \
  --observed-at 2026-10-08T12:00:00Z \
  --window-start 2026-10-08T11:00:00Z --window-end 2026-10-08T13:00:00Z \
  --idempotency-key status-receipt-1
```

The explicit import validates native session identity against the registered PCTX session, workspace and current epoch, requires an explicit task ID, then delegates usage persistence and counter/baseline validation to `quota ingest`. Missing values stay unknown. The offline schema accepts Claude Code versions 2.1.211 through 2.1.284; other versions disable this collector until checked. The current [official status-line documentation](https://code.claude.com/docs/en/statusline#available-data) describes `cost.total_cost_usd` as a client-side estimate and `context_window.total_input_tokens` / `total_output_tokens` as current-context counts. Accordingly, the importer records cumulative USD **estimates** through shared quota and returns context percentage **snapshots** as metadata, and imports no cumulative token usage. It never represents this estimate as billed cost or subscription quota. Provider observations use the explicit structured `quota ingest` API; neither transport asserts authenticated native collection. The status line itself is preserved and never automatically replaced.

When `claude` is absent, doctor reports installed hook support as false, while documented capability and fixture capability remain separate. PostCompact is never added by the bootstrap profile on documentation evidence alone. Actual managed scheduler execution receipts and keep-awake remain separate integration work.
