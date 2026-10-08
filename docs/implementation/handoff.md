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

Published second development commit: `40c6309c7631a2a5d6482852aae1327132feca8b`. Third development integration is being committed; inspect Git/origin to establish its exact revision rather than assuming the second revision contains it. All three agents handed back their slices; no agent owns an active build. Parent owns shared Cargo/CLI/reader/CI integration. No managed worktree is used.

Pending third integration includes candidate-first metadata search, bounded exact-pattern policy compilation cache (8 entries, not an RSS guarantee), Unix root/ancestor no-follow opening, renderer/CLI byte-budget enforcement, Linux cfg-only environment mutation correction, and native Windows Job Object/SID/process-identity foundations. Windows foundation is not wired into the supervisor yet; macOS cannot verify its native compilation/runtime. A Windows CI step runs its seven native tests before full static checks. Complete suspended launch/guardian/reconciliation remains mandatory.

The root-security regressions cover leaf, root and ancestor symlink replacement, including empty metadata searches. Cache fixtures cover separate projects/current source/invalid globs and concurrent eviction. Search fixtures cover Boolean complements, aliases, regexes, stale candidate sources, exclusion changes and text/body validation. Format fixtures cover reversible fields, safe fences, control characters and actual CLI Markdown reads/unsupported-format rejection before writes. JSON budget errors retain available durable execution reread handles; a request too small to hold a complete error envelope cannot be promised bounded valid JSON.

## Measurements and platform evidence

Normative M/30 baseline **completed** (session 98971 terminal, exit 0). Reports are `evidence/evaluation-m30-baseline.json` and `.md`, original observations `/private/tmp/pctx-second-normative-m30`. Actual corpus: 10,000 files/209,715,200 bytes, 6,000 supported sources, 18,000 declared symbols, 4,000 document sections; 30 repetitions. Initial index 16.63s; incremental p95 2.82s; metadata search p95 1.26s; matched symbol p95 2.64s; body p95 2.66s; strict context p95 3.96s. Metadata/symbol/body missed their time targets. All 60 labeled searches exited partial, so Recall@20/MRR are unknown. Strict context was deterministic and within 12,000 bytes. Baseline failures prompted current search/policy changes; a fresh release M/30 observation is required. Model tokens/cost/quota/task outcome and hardware equivalence remain unknown.

Second smoke: zero mandatory diagnostic misses, zero observed presentation false passes, net 462,221/770,084 bytes saved (60.02%) including rereads; negative individual savings retained. This 100-file/51,200-byte/one-repeat smoke is not normative certification. Latest archive belongs to the second source revision, SHA256 18ececb2670a97693f4f251fb286624ddf76630b45d699765a5627965fb5ba28, verified in `evidence/second-archive-validation.log`; it does not contain pending third changes or prove future-version migration.

Actual second GitHub CI run 37713006538 is terminal: macOS passed; Linux failed schedule environment mutability; Windows failed 24 static/cfg diagnostics. See `evidence/second-github-ci-status.json`, `second-linux-ci-failure.log`, `second-windows-ci-failure.log`. Windows foundation was untracked and absent from that run. Do not treat any previous CI as proof of current source. Historical first CI failures remain separately retained.

## Next actions

1. Third local gate completed: formatting, all-target Clippy with warnings denied, 209 actual macOS tests and release build passed. Logs: third-full-macos-tests.log, third-macos-static.log, third-release-build.log. New M/30 observation is LIVE in shell session 39868, directory /private/tmp/pctx-third-normative-m30, log /tmp/pctx-third-normative-m30.log. Revalidate that exact handle before waiting or starting heavy work. No agent owns active edits.
2. Build the release binary and run unchanged M/30 harness into a new directory. Observe exact live handle; never replace a job merely because an observation timed out. Inspect errors/coverage/denominators and retain negative/unknown results.
3. Commit/push verified development changes and inspect actual native Windows/Linux/macOS CI. Fix rather than hide warnings. Complete the Windows supervisor using the containment foundations and independent official-contract review.
4. Continue remote workflow, tokenizer/adaptive context, remaining follow-up contracts and full requirement audit. Requirement status is not promoted from a subset of passing tests. Full goal remains active.

No operational deployment, global hook, OS schedule registration, paid model call or user issue mutation was performed. Test children are isolated temporary fixtures. Native boot/PID tests require the authorized native execution context: restricted sysctl denial is not platform evidence. Preserve the Korean source specification and every existing user change.
