# Resume handoff

Full goal: implement the mandatory specification 0.6 and concrete follow-ups, with verified CLI, installation artifacts, English public documentation, honest measurements, commit/push. Do not close at this development milestone.

Workspace /Users/dev/PCTX, main, origin https://github.com/Jung95/pctx.git. Inspect `git log -1` and `origin/main` for the first verified development commit. Preserve the original source specification locally. Implementation scope does not authorize operational deployment, global hooks, OS schedules, paid model calls or user issue edits.

## Runtime

Use project-local tools; the direct CommandLineTools path avoids the system Git/SDK shim. Keep one heavy build/test active:

```sh
export CARGO_HOME=/Users/dev/PCTX/.toolchain/cargo
export RUSTUP_HOME=/Users/dev/PCTX/.toolchain/rustup
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export PATH="$CARGO_HOME/bin:/Library/Developer/CommandLineTools/usr/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
cargo test --locked --offline
```

Tests use isolated temporary project roots and PCTX_DATA_DIR. Real local platform evidence is macOS arm64 only.

## Current integration

Published second development commit: `40c6309c7631a2a5d6482852aae1327132feca8b`. Third development commit af1181c8d4f83ffb3f6e45c1c7c39706dd8b3649 is pushed and equals origin/main at this observation. New subsequent changes below are uncommitted and unverified. All three agents handed back their slices; no agent owns an active build. Parent owns shared Cargo/CLI/reader/CI integration. No managed worktree is used.

Pending third integration includes candidate-first metadata search, bounded exact-pattern policy compilation cache (8 entries, not an RSS guarantee), Unix root/ancestor no-follow opening, renderer/CLI byte-budget enforcement, Linux cfg-only environment mutation correction, and native Windows Job Object/SID/process-identity foundations. Windows foundation is not wired into the supervisor yet; macOS cannot verify its native compilation/runtime. A Windows CI step runs its seven native tests before full static checks. Complete suspended launch/guardian/reconciliation remains mandatory.

The root-security regressions cover leaf, root and ancestor symlink replacement, including empty metadata searches. Cache fixtures cover separate projects/current source/invalid globs and concurrent eviction. Search fixtures cover Boolean complements, aliases, regexes, stale candidate sources, exclusion changes and text/body validation. Format fixtures cover reversible fields, safe fences, control characters and actual CLI Markdown reads/unsupported-format rejection before writes. JSON budget errors retain available durable execution reread handles; a request too small to hold a complete error envelope cannot be promised bounded valid JSON.

## Measurements and platform evidence

Normative M/30 baseline **completed** (session 98971 terminal, exit 0). Reports are `evidence/evaluation-m30-baseline.json` and `.md`, original observations `/private/tmp/pctx-second-normative-m30`. Actual corpus: 10,000 files/209,715,200 bytes, 6,000 supported sources, 18,000 declared symbols, 4,000 document sections; 30 repetitions. Initial index 16.63s; incremental p95 2.82s; metadata search p95 1.26s; matched symbol p95 2.64s; body p95 2.66s; strict context p95 3.96s. Metadata/symbol/body missed their time targets. All 60 labeled searches exited partial, so Recall@20/MRR are unknown. Strict context was deterministic and within 12,000 bytes. Baseline failures prompted current search/policy changes; a fresh release M/30 observation is required. Model tokens/cost/quota/task outcome and hardware equivalence remain unknown.

Second smoke: zero mandatory diagnostic misses, zero observed presentation false passes, net 462,221/770,084 bytes saved (60.02%) including rereads; negative individual savings retained. This 100-file/51,200-byte/one-repeat smoke is not normative certification. Latest archive belongs to the second source revision, SHA256 18ececb2670a97693f4f251fb286624ddf76630b45d699765a5627965fb5ba28, verified in `evidence/second-archive-validation.log`; it does not contain pending third changes or prove future-version migration.

Actual second GitHub CI run 37713006538 is terminal: macOS passed; Linux failed schedule environment mutability; Windows failed 24 static/cfg diagnostics. See `evidence/second-github-ci-status.json`, `second-linux-ci-failure.log`, `second-windows-ci-failure.log`. Windows foundation was untracked and absent from that run. Do not treat any previous CI as proof of current source. Historical first CI failures remain separately retained.

## Next actions

1. Third local gate completed: formatting, all-target Clippy with warnings denied, 209 actual macOS tests and release build passed. Logs: third-full-macos-tests.log, third-macos-static.log, third-release-build.log. New M/30 observation COMPLETED: session39868 terminal exit0, directory /private/tmp/pctx-third-normative-m30, reports evidence/evaluation-m30-third.json/.md. Initial15.65s, incremental p95 2.01s, metadata347ms (miss), matched symbol364ms (met), body2164ms (miss), strict build2622ms (met). Inspect search_metrics/errors before accuracy claims. No agent owns active edits.
2. Build the release binary and run unchanged M/30 harness into a new directory. Observe exact live handle; never replace a job merely because an observation timed out. Inspect errors/coverage/denominators and retain negative/unknown results.
3. Commit/push verified development changes and inspect actual native Windows/Linux/macOS CI. Fix rather than hide warnings. Complete the Windows supervisor using the containment foundations and independent official-contract review.
4. Continue remote workflow, tokenizer/adaptive context, remaining follow-up contracts and full requirement audit. Requirement status is not promoted from a subset of passing tests. Full goal remains active.

No operational deployment, global hook, OS schedule registration, paid model call or user issue mutation was performed. Test children are isolated temporary fixtures. Native boot/PID tests require the authorized native execution context: restricted sysctl denial is not platform evidence. Preserve the Korean source specification and every existing user change.

## Subsequent review and pending corrections

Independent search review found invalid project/merged-user glob programs can be swallowed as successful empty results, and strict find/query/outline discard incomplete refresh coverage. Parent has verified fixes in src/reader.rs/project.rs/storage.rs/search.rs/main.rs and new tests/cli_contract.rs/tests/search.rs. These are NOT covered by the 209-test gate, the pushed af1181c commit or live M/30 binary. Fourth native gate completed: formatting of changed parent files, all-target Clippy and all212 macOS tests passed; evidence fourth-full-macos-tests.log/fourth-macos-static.log/fourth-targeted-tests.log. M/30 is terminal. Windows agent edits remain outside this platform proof. Policy compilation validates effective configuration and empty-input APIs; strict lookups retain available results with refresh reasons, partial status, exit3 and unknown omission count. Actual CLI fixtures passed for invalid project and merged-user globs, ordinary intentional exclusion, and partial strict find/query/outline retaining available auth evidence.

Root binding has a further mandatory gap: replacing the root with a different *normal directory* after Project::open can reuse old policy/workspace identity. Symlink protection does not prove root-instance identity. Implement a loaded-project identity bound to the opened root and test regular-directory replacement; no completed security claim covers this gap. Search has a hardcoded cooperative 2s limit; spec's configurable general timeout remains unfinished.

Agent requirements owns active edits ONLY src/windows_process.rs/tests/windows_process.rs: explicit native suspended launch/job-list/handle-list pipes/argv/env/cwd admission. Parent must add requested Win32_System_Pipes feature when handed back, then actually compile/test on Windows. Benchmark39868 is now terminal; parent owns the single heavy build queue. Agent work_control owns ONLY requirements.md bounded PCTX01–16/AC01–20 evidence audit; agent search completed read-only review. Preserve all changes. CI run37715893709 is terminal on af1181c: Linux/macOS success, Windows7 primitive tests passed then24 shared static failures. Evidence third-github-ci-status.json/third-windows-ci.log.

Third CI updated: Linux and macOS actual jobs completed successfully on af1181c; Windows still in progress at the last observation. This does not verify subsequent uncommitted corrections.

## Native Windows admission slice handed back

Requirements agent completed src/windows_process.rs/tests/windows_process.rs; parent added only Win32_System_Pipes to existing windows-sys features. The module now has explicit absolute executable/CRT argv, UTF-16 inputs, bounded explicit env/cwd, restricted inherited pipe handles/EOF stdin, atomic JOB_LIST assignment with CREATE_SUSPENDED, caller-obligated unsafe resume after durable guardian ACK, unsigned signaled exit (including259), rejection and containment accounting. Thirteen native fixtures exist; they have NOT run on Windows yet. Previous seven native CI passes do not cover these edits. Parent format/all-target host static checks passed; host excludes Windows, so this is not native compile proof. Windows Unicode environment *names*, full guardian/lease/capture cancellation and end-to-end process reconciliation remain mandatory. Files are stable, no agent actively editing or building.

Fourth development commit1981648 is pushed (inspect exact SHA). Policy/strict fixes and212-test proof are included. M/30 observation was the earlier immutable af1181c binary. No heavy local job is live at this handoff; sessions39868/98027/42718 are terminal. Revalidate any newly dispatched CI job separately.
