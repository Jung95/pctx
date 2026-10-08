# Resume handoff

Full active goal: implement all mandatory specification0.6 and concrete follow-ups, with actual CLI, automated verification/measurements, English public documents, installable artifacts and authorized commit/push. No operational deployments/account mutations/global hooks/OS schedule registration/paid models are authorized merely by the specification. Preserve all user changes and the original Korean specification (ignored locally; SHA2568914faad57432f524aec0a06aa90c43c18c23f8c19e9f54fe69d9ad185db6523).

## Workspace and tools

Workspace /Users/dev/PCTX, main, origin https://github.com/Jung95/pctx.git. Latest published commit before this integration:4650e5be05bdee0ec5387e0e30e95c8a514695a8. Inspect HEAD/origin rather than assuming they equal this historical revision. No managed worktree is used. GitHub Jung95 authentication was network-verified; restricted-network auth failure was misleading.

```sh
export RUSTUP_HOME=/Users/dev/PCTX/.toolchain/rustup
export CARGO_HOME=/Users/dev/PCTX/.toolchain/cargo
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export PATH="$CARGO_HOME/bin:/Library/Developer/CommandLineTools/usr/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
```

One heavy build/test/benchmark at a time. Native macOS boot/PID/guardian tests need the authorized native context; restricted sysctl denial is not a platform result.

## Current integration and ownership

Root/budget/FIFO integration passed221actual macOS tests, all-target Clippy(-Dwarnings), and42targeted broker/CLI/root/search tests. Release build passed. Evidence sixth-full-macos-tests.log/sixth-macos-static.log/sixth-targeted-tests.log/sixth-release-build.log. Intermediate fixture allocation warning is retained separately; fixed without suppression.

Mandatory Project.root_anchor holds original root AND ancestor handles, caching only immutable Unix dev/inode identities. Actual no-follow opened directories/config/source files are compared; ordinary-directory or ancestor replacement (even unchanged final root inode) cannot rebind authority. Normal source edits remain allowed. Config is bounded regular1MiB; Unix FIFO opens cannot block before regular-file rejection. Unix read removes duplicate pathname checks while keeping current policy+anchored checks before both opens; non-Unix keeps pathname checks. Windows root traversal remains path-based, with native reparse races/ReFS128-bit identity proof missing.

CLI minimum stdout capacity is derived from an actual complete newline JSON error envelope (513–517bytes by command). Smaller budgets fail INVALID_ARGUMENT2 before Project/output-file/index/lease/check/context/child effects. Valid capacities can still fail required-content/evidence with BUDGET_TOO_SMALL8. Detailed boundaries: docs/cli/formats.md. Pack capacity is separate. Full logical ledgers remain unchanged under rejected capacity fixtures.

Parent owns common Cargo/lib/CLI/CI/docs and integrated root files. Search/work_control agents handed back and are idle. Requirements agent actively owns ONLY NEW src/windows_guardian.rs/tests/windows_guardian.rs. These untracked files are NOT in the completed gate or linked application, must not be silently committed as verified. Requested future feature Win32_Storage_FileSystem has not yet been added; parent must review/register module, hidden fixed guardian entry and native CI fixture step after stable handback. Existing thirteen Windows process-admission primitives are separate.

## Exact live handle and measurements

Immutable sixth release M/30 is LIVE in shell session6734, directory /private/tmp/pctx-sixth-normative-m30, log /tmp/pctx-sixth-normative-m30.log. Revalidate the same handle before waiting or any heavy job; transient observation timeout is not termination. No other heavy local job is intentionally active. Do not rebuild/replace that binary during observation. On completion inspect errors/coverage/corpus manifest and retain misses/unknowns; then update evidence and requirements.

Historical third M/30 (immutable af1181c) completed:10,000files/209,715,200bytes/30reps, manifest810c9a60…; index15.65s, incremental p952.01s, metadata347ms(miss250), matched364ms(met700), body2164ms(miss2000), build2622ms(met15000), all warm errors0; all60labels measured, answer-bearing Recall@20/MRR1, no-answer10/10empty, no wrong hashes. Reports evaluation-m30-third.json/.md. Model tokens/cost/quota/outcomes/hardware equivalence/cold-cache control remain unknown. Baseline failures remain historical in evaluation-m30-baseline.json/.md. Latest archive is still second-source revision (18ececb… SHA256), not current source; regeneration and installation/update/removal/migration proof remain required.

## Actual CI and next actions

Fifth CI37716879921 on4650e5b is terminal: macOS passed; Windows13native suspended/identity/job tests passed then24shared static/cfg diagnostics failed; Linux static passed but broker five_actual_cli_processes_share_one_refresh failed without subprocess payload. Evidence fifth-github-ci-status.json/fifth-windows-ci.log/fifth-linux-ci-failure.log. Cause is unknown; do not infer500ms wait as the cause or relax same-revision/one-refresh assertions. New broker fixture diagnostics capture status/stdout/stderr for the next run. Earlier Linux green is historical. Upcoming CI adds Windows reader/CLI fixtures before static checks; that does not replace full Windows backend verification.

1. Commit/push only verified root/budget changes and evidence, preserving untracked guardian files; inspect exact new CI, especially Linux broker response and Windows core fixtures.
2. Observe live M/30 session6734 to terminal before new heavy work. Audit its performance/accuracy and regenerate install artifact only after stable source/release evidence.
3. Review native keeper handback. Integrate shared runner/lease/output capture, suspended admission/durable ACK before resume, native reconciliation and guardian-failure takeover blocking. Windows LockFileEx process-owned locks require the shared ledger to retain unknown live jobs; no TTL/missing-job/handle-close release inference. Native boot identity is unknown, not an invented UUID. Actual Windows tests are required.
4. Finish configurable general timeout, full performance target corrections, remote authenticated pagination/PR-head gates/outbox, tokenizer/adaptive context and remaining mandatory contracts. Keep requirement audit honest; no whole-goal completion claim from passing subsets.
