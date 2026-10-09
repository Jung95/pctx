# PCTX01-G05-D05 — exact-child script-policy wait, 2026-10-09

## Finding

The approved once-only7s all-process trace completed (recorder0, original test101). Each of the three failed query roots49083/49084/49086 has a distinct trace-local process numeric identity and a single matching TID. The detailed tables now specify target-pidALL. Original test diagnostics report elapsed1000ms, spawn_ms0 (millisecond-rounded), no stdout/stderr bytes, no observed root exit,164 iterations. The harness reports0passed/1failed/13filtered and1.01s test-body; planned128 outcomes are not all directly recorded.

| Failed PID | TID | Long Blocked interval, ms | Policy-wait callchain samples |
| --- | --- | ---: | ---: |
| 49083 | 10147983 | 1000.103125 | 1 |
| 49084 | 10147984 | 1000.093209 | 1 |
| 49086 | 10147986 | 1000.066625 | 1 |

Each context-switch-associated stack contains `lck_mtx_sleep`, `ASPEvaluationManager::waitOnEvaluation`, `AppleSystemPolicy::waitForEvaluation`, `AppleSystemPolicy::evaluateScript`, and `AppleSystemPolicy::procNotifyExecComplete`. This directly associates each failed child with the script-policy evaluation wait path. The stacks are recorded just after the long Blocked intervals end, not repeated pre-deadline samples proving an unchanged stack throughout. Running labels and inclusive weights are not CPU duration. Blocked intervals end near cancellation and threads terminate shortly afterward; this does not prove policy assessment completed.

Thirty owned frames retain only image UUID/offset and five allowlisted policy/wait symbols; twelve owned sample rows and thirty scheduler-state intervals remain. No absolute kernel addresses, user paths or unrelated process frames are retained. The kernel UUID is94769496-091D-3DF1-84AB-5CAD0DB6A5D3; AppleSystemPolicy UUID8474AAC8-5327-3045-A969-9B20DBF588BD. Exact service path/birth admission was not completed before raw deletion, so display-name-selected service rows were removed after independent review; no current service/compiler attribution is claimed.

## Code and wording correction

`tests/project_deadline.rs:149` `fake_git` writes a fresh mode0700 `#!/bin/sh` script. It is executed via the native OS spawn path, but is not a Mach-O native executable fixture. Earlier shorthand “native fixture” was misleading; the original script bytes and all test conditions remain unchanged. Each worker creates its script once before the16-iteration loop, then reuses that path. The capture's `evaluateScript` path agrees with the actual fixture code.

`src/query_process.rs:639` preserves PGID=PID through standard process_group setup, then calls Command::spawn; the bounded collector checks the shared deadline. The spawn observation is below one millisecond after rounding, while the owned child is blocked in the platform policy path for approximately the whole deadline. No Rust launch/pipe change is demonstrated to remove this wait while preserving the original executable/PGID/environment/deadline.

Apple's public [XNU task_wait_to_return source](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/osfmk/kern/task.c) calls `mac_proc_notify_exec_complete` before return to user execution. This supports the execution-stage interpretation. The public main branch is contextual source, not established as the installed kernel's exact source revision or the proprietary AppleSystemPolicy implementation.

## Preserved-condition correction review

Increasing/resetting the deadline, warming fixtures, reducing8x16 concurrency, replacing direct execution with an interpreter/helper, or ignoring failing startup checks violates the agreed reproduction. Disabling/bypassing policy also lies outside approval. This capture identifies a concrete platform wait; it does not identify the assessment request ID, an internal compiler bottleneck, memory contribution or a vendor-supported correction. No product patch or successful qualification follows. A supported platform correction must address script assessment latency without changing these conditions; none is yet verified. A minimal local vendor draft was updated with the exact-owned evidence; nothing was submitted.

## Collection, controls and cleanup

Matching unique notification preceded one test launch. Its1750ns coordinator timestamp gap is not child startup duration. TOC/detail exports succeeded; os-log exports contain0rows. Recorder/root/wrapper terminal receipts are explicit and listener cancellation returned0. Eleven known recorder/wrapper/root/failed-child/exporter PIDs were absent;414231462logical bytes of private raw/exports were removed. Successful child/full descendant enumeration remains unproved; external Instruments caches were not inspected/deleted. Product/test/lock hashes match executed pins.

PCTX01-G05-D05 remains product_failure; PCTX01 stays33/40 details,0/10 gates. This diagnostic scope is consumed; no further recording, task switch or Actions run occurred. Next only within D05: inspect a supported platform correction or public source guidance for this exact script-policy wait, then repair and verify all four original startup contracts and required platforms. Do not repeat a generic stack capture or claim an internal compiler cause from earlier service-only evidence.

Evidence: [numeric result](pctx01-startup-child-trace-numeric.json), [run receipt](pctx01-startup-child-trace-result.json), [export receipt](pctx01-startup-child-trace-export.json), [executed scope](pctx01-startup-child-trace-executed-plan.json), [cleanup](pctx01-startup-child-trace-cleanup.json), [reducer](pctx01-startup-child-trace-reduce.py.txt).
