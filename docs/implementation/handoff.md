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

## Current ownership and pending integration

First verified development commit is `ad6ab48f1987d3590a937be1eabe2041270e7b6d`, pushed to origin/main. Current changes are uncommitted and not a new verified milestone. Preserve every changed/new file. GitHub account Jung95 was rechecked against the network successfully; a restricted-network auth probe falsely reported invalid credentials, so no credential replacement was necessary. The previous goal turn yielded actual auth evidence (progress).

Main owns all integrated modules after stable handbacks. No agent currently edits files. Requirements agent completed a read-only Windows backend design using official Microsoft sources; a native Job Object/guardian implementation remains mandatory. Current new files include parser/inventory modules and CLI docs. No managed worktrees exist.

## Exact next actions

Current integration passed formatting, all-target Clippy with warnings denied, and all 190 macOS tests. Evidence: `evidence/second-macos-static.log`, `evidence/second-full-macos-tests.log`. Restricted tests disclosed missing native boot identity rather than fake support; authorized native tests proved guardian ownership/attachment, parent-death survival, isolated inherited-FD retention, schedule claim races and restoration. Historical restricted/native intermediate failures remain in separate history logs, including a fixture querying the workspace rather than control database; the corrected fixture passes.

Parent integrated v2 schedule backup/restore whitelist and scrubbed portable install bindings/private metadata; running attempts restore as interrupted_unknown, installations unknown_restored. Registered Git NUL parsers preserve filename newlines and separators, with binary fallback still partial for ordinary commands. Parser presentation omissions and complete nested check output budgets/delivery are integrated. Canonical check/resource/job routes call existing application services. Check-alias-specific lease/trust regression still needs expansion.

Release build and second smoke completed. `evidence/evaluation-second-smoke.json`: zero mandatory diagnostic misses, zero observed presentation false passes, net 462,221 bytes saved (60.02%) including rereads, negative savings retained for small cases, supported-parser bundle/model outcome unknown. Nonnormative corpus 100 files/51,200 bytes/one repetition. Local regenerated archive includes public docs; `evidence/second-archive-validation.log` proves checksum/safe entries/private-source exclusion/README guides and isolated init/index/find/inventory. Package SHA256 18ececb2670a97693f4f251fb286624ddf76630b45d699765a5627965fb5ba28. Latest archive does not prove future-version migration or external installation.

Normative M/30 observation is LIVE in shell session 98971, output `/private/tmp/pctx-second-normative-m30`, log `/tmp/pctx-second-normative-m30.log`. It began after all heavy jobs completed. Revalidate the exact live session with write_stdin before waiting; a transient timeout is not termination. Do not launch another build/test/benchmark while live. On completion inspect report/corpus denominators/errors/eligibility, preserve observations in evidence, update requirements/progress and commit/push. Do not claim normative performance success until the actual result is audited. Model/provider/cost usage remains unmeasured.

First actual cross-platform CI run https://github.com/Jung95/pctx/actions/runs/37710582060 failed: Windows static checks (21 cfg/unreachable/unused warnings), Linux static checks (2 warnings), macOS static checks passed but 3 guardian fixtures timed out before durable attachment. Failed logs are preserved in `evidence/first-github-ci-failures.log`. Native-platform tests have NOT passed CI. Independent audit notes fixture 8-second total admission bound cannot cover both allowed 5-second guardian handshakes plus repeated binary hashing; diagnosis is pending, not proof of a native lock bug. Keep durable guardian ACK and child-survival exclusion strict, add bounded phase/slot diagnostics, and rerun after integration. Windows concrete process backend remains mandatory work, not a permanently accepted unsupported stub.

Continue Windows native backend, remote workflow, tokenizer/adaptive context and remaining mandatory contracts after this verified milestone; the complete requirement audit remains unfinished. Goal stays active.

No PCTX product child job is intentionally active outside isolated fixtures. Authorized native process inventory found no cargo/rustc/guardian jobs at this integration check. Restricted process inventory alone was denied and did not establish that fact. Parent has not started a new heavy build while agents finish edits. Local first archive/install evidence and prior bounded release measurements remain historical in `evidence/archive-install-validation.log` and `evidence/evaluation-smoke-bounded.json`; do not claim latest source matches that artifact.
