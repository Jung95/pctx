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
- Actual first GitHub CI ran on Windows, Linux and macOS and failed (run 37710582060). Windows/Linux stopped at static checks; macOS stopped at three guardian fixture admission timeouts. No passing platform CI, provider model usage, paid cost evaluation or live product connector result is established. Byte measurements are not model-token or quota savings.

## Environment and publication

macOS arm64, 10 CPUs, 16 GiB; local ignored Rust 1.99.0. SQLite bundled with runtime minimum version check. Git branch main, origin https://github.com/Jung95/pctx.git. GitHub network authentication confirms active Jung95. This gate prepares the first verified development commit; inspect `git log` and `origin/main` for publication status. Public documentation is English; the original Korean input is preserved and excluded; the faithful English edition and translation coverage record are present (PCTX67).

## Next actions

Pack corrections, scoped documents, quota ledger, local schedule registry and offline Claude adapter are integrated. Registered runner has 19/19 isolated tests passing using native macOS OS identity, including canonical bridge exclusion, child survival after parent death and negative observation after guardian death. Watch/NDJSON regressions pass (4 watch + 5 CLI tests); quota restore regression passes (7 quota + 8 work tests). The final macOS gate passed formatting, all-target static checks with warnings denied, and all 148 tests; see `evidence/full-macos-validation.log`. The local macOS arm64 archive is generated with notices/SBOM for 131 locked dependencies. Hash/extraction/version, isolated exploration, atomic same-build reinstallation and removal preserving user data passed; see `evidence/archive-install-validation.log`. Future-version migration is not established by reinstalling the same build. Commit and push the verified development milestone, keeping incomplete requirements explicit. Continue registered checks/resource supervision, scheduling, adapters, remaining coordination/search/contracts and reproducible measurement toward the full goal.

## Latest measurement observation

`evidence/evaluation-smoke-debug.json` is an actual debug-binary run on 100 files / 51,200 bytes, one repetition. It is not normative performance qualification. Exact identifier file Recall@20 was 1.0; model tokens/cost and task success remain unknown. Verbose capture timed out and lost required diagnostic coverage; rereads incurred negative net byte savings, retained in the report. Investigation found fixed masking regexes compiled per record; these now use a shared lazy compiled pattern set. New release measurements are required before any performance claim.

Release smoke observation: `evidence/evaluation-smoke-release.json` records the optimized binary on the same nonnormative 100-file / single-repeat corpus. Verbose children now exit normally, but record limits still cause lost diagnostics and negative net byte savings. Session workload full packets did not produce baselines; investigation remains open. These failures remain visible and prevent acceptance promotion.

Corrected session fixture: `evidence/evaluation-smoke-bounded.json` declares a one-symbol task scope and unchanged 12,000-byte budget. Five actual sessions produced 1,992-byte full packets and 1,106-byte unchanged delta packets; old baseline after compact was rejected. Five simultaneous Git queries observed one refresh. These are local byte/correctness observations, not token/billing/quota evidence. Verbose output still missed one required diagnostic and incurred -78,161 net saved bytes (-34.08%); this remains a required correction.

## Pending second integration

First verified development commit `ad6ab48` is pushed. Current uncommitted extensions include retained late diagnostic tails, registered parsers, static inventory/profile/drift proposals, canonical check aliases, shared scheduled mailbox, managed schedule execution and portable schedule restore scrubbing. Parser/inventory modules were handed back; schedule and CI/privacy repairs are still owned by agents. Newest edits and regression fixtures await compilation/testing; the first-commit gate does not verify them. Detailed ownership and exact next actions are in handoff.md.

Second integration gate: formatting and all-target static checks passed; all 190 actual macOS tests passed (`evidence/second-macos-static.log`, `evidence/second-full-macos-tests.log`). This includes 15 parser, 8 inventory, 11 operations, 10 output, 20 runner, 17 schedule and 9 work-control tests, plus remaining full regression suites. Native schedule registration and active keep-awake effects were deliberately not exercised. New release smoke and normative M/30 observations are pending; cross-platform CI remains failed until a new actual run proves otherwise.

Second release smoke (`evidence/evaluation-second-smoke.json`) retained all mandatory diagnostics (zero misses) and observed zero presentation false passes. Net emitted-byte reduction after rereads was 462,221 / 770,084 bytes (60.02%); tiny/malformed/Unicode/large-record cases still had negative individual savings. Supported-parser bundle and model outcome remain unknown; this 100-file/51,200-byte/one-repeat smoke is not normative certification. The regenerated local archive includes public guides; checksum, safe entries, private-input exclusion, README guide links and isolated init/index/find/inventory passed (`evidence/second-archive-validation.log`).

Normative M/30 free observation started after all builds/tests/package checks, using 10,000 files/200 MiB and unchanged workload budgets. It completed; the retained M/30 baseline misses metadata/symbol/body targets and cannot establish search accuracy because all labeled searches returned partial. See evaluation-m30-baseline.json/.md. No model/provider calls are authorized or made.

Publication: commit `40c6309` is pushed. Actual second GitHub CI run 37713006538: macOS succeeded, Linux failed E0596 in Linux-only schedule environment mutation (current uncommitted cfg shadow fix awaits new CI), Windows failed 24 static/cfg diagnostics; its native tests did not run. See `evidence/second-github-ci-status.json` and `second-linux-ci-failure.log`. Active next extensions are Windows native containment foundation, metadata candidate-first search and actual output format rendering; their code is not covered by the published gate.

## Third integration in progress

Candidate-first metadata search and compiled policy programs address measured baseline overhead without reusing source or permission decisions. Independent review found a root/ancestor symlink escape; Unix root opening now walks no-follow descriptors from the filesystem root, allowing only macOS fixed system aliases with exact observed targets. Root/ancestor/leaf and concurrent cache-eviction regressions are included. The new renderer is wired into the actual CLI with final-byte accounting and complete JSON budget errors. Windows SID/private Job Object/native FILETIME identity foundations remain separate from the unfinished supervisor backend; actual Windows CI is required. Full macOS/static validation and a new unchanged M/30 observation are the next gates.

Third local gate passed: formatting, all-target Clippy with warnings denied and 209 macOS tests. Logs: third-macos-static.log / third-full-macos-tests.log; intermediate Markdown error-format fixture failure remains in third-intermediate-format-failure.log. Release build passed. Windows foundation is excluded on this host and not verified by the 209-test result. A new immutable release binary M/30 observation is starting; no measured improvement is claimed yet.

Third M/30 immutable release observation completed on af1181c with the same declared corpus hash and 30 repetitions. Initial index15.65s, incremental p952.01s, metadata347ms (miss250), matched symbol364ms (met700), body2164ms (miss2000), strict build2622ms (met15000). All warm samples succeeded, all60 labeled queries measured, answer-bearing categories Recall@20/MRR1.0, zero wrong hashes. No-answer accuracy has its separate empty-result checks. Reports evaluation-m30-third.json/.md retain model/cost/quota/task-outcome/hardware unknowns. These results do not verify subsequent source corrections.

Actual third CI on af1181c: Linux and macOS full jobs passed; Windows seven native identity/Job Object foundation fixtures passed, then shared-module static checks failed24 diagnostics. Full Windows supervisor remains incomplete. Evidence third-github-ci-status.json / third-windows-ci.log.

Independent review found invalid merged security globs could become successful empty results, strict lookup could hide partial refresh, and regular-directory root replacement was not identity-bound. Parent implemented policy compilation validation at config/empty-input query boundaries and carried refresh coverage/reasons through strict find/query/outline; static and targeted CLI/search/cache tests passed. Whole native gate is running. Root-instance binding and configurable general timeout remain mandatory unresolved behavior. New Windows suspended admission code is being developed separately and is unverified.

Fourth native gate completed with212 macOS tests and all-target Clippy passed (fourth-full-macos-tests.log / fourth-macos-static.log). This verifies policy/strict correction fixtures including ordinary excluded-path behavior and invalid merged-user programs. It does not complete regular-directory root binding, configurable timeout, Windows launch integration or remaining full-goal contracts.

## Sixth integration gate

Mandatory immutable root/ancestor identity binding and anchored configuration/source reopen checks are integrated. New regressions reject ordinary-directory replacement (including unchanged root inode under a replaced ancestor), deleted-root recreation, and FIFO source/config admission; normal and atomic source edits remain valid. Minimum JSON-error capacity is rejected before effects, with exact command-specific boundaries and unchanged index/control/lease/check/context ledgers in actual CLI fixtures.

All221 macOS tests passed and all-target Clippy with warnings denied passed;42 targeted broker/CLI/root/search tests passed. Evidence sixth-full-macos-tests.log/sixth-macos-static.log/sixth-targeted-tests.log; intermediate single fixture allocation warning retained separately, corrected without suppression. This gate does not verify Windows pathname races or suspended/guardian application integration.

Actual fifth CI4650e5b: macOS passed, Windows suspended-admission13 native tests passed then24 shared static failures; Linux static passed but five-process broker fixture failed without its subprocess error payload. New fixture diagnostics retain success/same-revision/one-refresh assertions; cause remains unknown pending the next actual Linux result. Fifth native logs/status retained. Next immutable release M/30 observation is required after root/budget changes; no improvement is presumed. Native Windows keeper module is under development and unlinked/unverified.

## Seventh integration in progress

Commit96c79a6 was confirmed pushed. Sixth CI37719044534 completed: macOS full gate passed, Windows13 native containment fixtures passed but10 CLI fixtures failed during init with IO_ERROR; Linux broker singleflight waiter returned refresh_pending/unavailable after its500ms cutoff. Exact native logs and statuses are retained in sixth-github-ci-status.json/sixth-linux-ci.log/sixth-windows-ci.log. Current corrections use a shared10s broker deadline, Windows flushed-file/same-directory MoveFileExW publication instead of Unix read-only directory fsync, and create-only race conflict classification. Native Windows confirmation remains pending.

Sixth immutable M/30 finished. Metadata584ms(miss250), matched585ms(met700), body2589ms(miss2000 and30/30 partial errors), strict build3527ms(met15000), initial16.13s and incremental2.46s. Unsupported labels0/10 measured; no complete accuracy claim. Reports evaluation-m30-sixth.json/.md and samples retain these outcomes. Candidate-first DB search now avoids opening nonmatching metadata rows while authorizing/hash-checking every match before LIMIT; unopened aggregate counts are suppressed. Windows keeper is registered as a fixed private pipe entry, but lease/capture/cancellation integration and native tests are not yet complete.

Seventh native macOS full gate passed230 tests and all-target Clippy with warnings denied;51 targeted broker/search/publication/CLI/root cases passed. Evidence seventh-full-macos-tests.log/seventh-macos-static.log/seventh-targeted-tests.log. Intermediate create-only race classification and missing fixture directory failures are retained separately and corrected. Windows keeper implementation has independent static review of owned-worker lifetimes, cancellation semantics and release proof; actual native execution is still pending. Platform cfg changes retain Unix behavior and unfinished Windows capability errors without blanket lint allowances.

Seventh local release archive generation and isolated validation passed: checksum,93 safe entries, no AppleDouble/private-spec input, binary equality, included license/notices/inventory/public README guides, installed init/index/find/inventory/doctor and explicit binary removal preserving project. Evidence seventh-package.log/seventh-archive-validation.json. First extraction failure from macOS sidecars is retained; COPYFILE_DISABLE and explicit verifier rejection corrected it. Forward-version update/migration remains unverified.
