# Implementation progress

Baseline: specification 0.6, all 45 sections read; 2026-10-08. The repository began with the owner's source specification only. Original input remains unchanged locally. Full required scope, including concrete follow-ups, remains active.

## Integrated behavior

- Project/workspace bindings, safe descriptor-based source reads, metadata-only SQLite generations, five Tree-sitter grammar variants, Boolean/literal search, extraction, bounded byte context, checkpoints and explicit handoffs.
- Separate task control database, transactional claim/revision gates, structured check evidence, historical completion validity, explicit session epochs and context acknowledgement/delta.
- Exact argv trust, manual non-heavy execution, masked bounded output records, reread without rerun, and observed byte delivery accounting.
- Static import graph, safe local Git status broker with cross-process singleflight, declarative inert filters, immutable source pack plans, local approval/pause/silence and recipient-bound mailbox.
- Staged index rebuild with prior database quarantine; control backup/restore revokes run leases, session acknowledgements and decision grants while preserving durable pauses.

These are partial specification implementations. Code or a passing subset does not imply an entire acceptance requirement is verified. Remaining mandatory contracts are tracked in requirements.md.

## Observed verification

- Current macOS arm64 integration evidence: current-integration-tests.log (output, session, search, work control); first-integration-tests.log (broker, CLI, multi-process concurrency, core security, graph; it retains the earlier output fixture failure).
- extensions-tests.log: filters, operations and pack initial suites passed. A later independent pack review found policy visibility, pinned read, aggregate bound and freshness gaps; fixes and new regression evidence are required before promotion.
- rebuild-tests.log: ten core/security tests passed, including corruption quarantine/checkpoint preservation and refusal to replace a newer schema. Independent review subsequently found a two-rename crash window; publication was changed to preserve the original by hard link and atomically replace the active path. Rerun pending.
- The output fixture failure used raw '%' as a printf format. It now passes data through '%s'; the secret masking assertion remains intact.
- No real Linux/Windows execution, provider model usage, paid cost evaluation, platform CI or live product connector result is established. Byte measurements are not model-token or quota savings.

## Environment and publication

macOS arm64, 10 CPUs, 16 GiB; local ignored Rust 1.99.0. SQLite bundled with runtime minimum version check. Git branch main, origin https://github.com/Jung95/pctx.git. GitHub network authentication confirms active Jung95. This gate prepares the first verified development commit; inspect `git log` and `origin/main` for publication status. Public documentation is English; the original Korean input is preserved and excluded; the faithful English edition and translation coverage record are present (PCTX67).

## Next actions

Pack corrections, scoped documents, quota ledger, local schedule registry and offline Claude adapter are integrated. Registered runner has 19/19 isolated tests passing using native macOS OS identity, including canonical bridge exclusion, child survival after parent death and negative observation after guardian death. Watch/NDJSON regressions pass (4 watch + 5 CLI tests); quota restore regression passes (7 quota + 8 work tests). The final macOS gate passed formatting, all-target static checks with warnings denied, and all 148 tests; see `evidence/full-macos-validation.log`. The local macOS arm64 archive is generated with notices/SBOM for 131 locked dependencies. Hash/extraction/version, isolated exploration, atomic same-build reinstallation and removal preserving user data passed; see `evidence/archive-install-validation.log`. Future-version migration is not established by reinstalling the same build. Commit and push the verified development milestone, keeping incomplete requirements explicit. Continue registered checks/resource supervision, scheduling, adapters, remaining coordination/search/contracts and reproducible measurement toward the full goal.

## Latest measurement observation

`evidence/evaluation-smoke-debug.json` is an actual debug-binary run on 100 files / 51,200 bytes, one repetition. It is not normative performance qualification. Exact identifier file Recall@20 was 1.0; model tokens/cost and task success remain unknown. Verbose capture timed out and lost required diagnostic coverage; rereads incurred negative net byte savings, retained in the report. Investigation found fixed masking regexes compiled per record; these now use a shared lazy compiled pattern set. New release measurements are required before any performance claim.

Release smoke observation: `evidence/evaluation-smoke-release.json` records the optimized binary on the same nonnormative 100-file / single-repeat corpus. Verbose children now exit normally, but record limits still cause lost diagnostics and negative net byte savings. Session workload full packets did not produce baselines; investigation remains open. These failures remain visible and prevent acceptance promotion.

Corrected session fixture: `evidence/evaluation-smoke-bounded.json` declares a one-symbol task scope and unchanged 12,000-byte budget. Five actual sessions produced 1,992-byte full packets and 1,106-byte unchanged delta packets; old baseline after compact was rejected. Five simultaneous Git queries observed one refresh. These are local byte/correctness observations, not token/billing/quota evidence. Verbose output still missed one required diagnostic and incurred -78,161 net saved bytes (-34.08%); this remains a required correction.
