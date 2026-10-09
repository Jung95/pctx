# macOS26.7.1: fresh shebang children block in AppleSystemPolicy beyond a1000ms query deadline

Local draft; not submitted. Project PCTX, public source baseline99462daf0c360f84c56fee7e76eadaaecb21db37. Native arm64, macOS26.7.1 build25G241, SIP enabled in prior diagnostic evidence. Fixed installed Info.plist versions: XProtect data5366, XProtectRemediator163, XprotectFramework/service1.0, AppleSystemPolicy2.0.0. These are installed values, not claims of update freshness or causality.

Each of eight workers creates one fresh mode0700#!/bin/sh fixture, then plans16 direct executions of that same path via native OS spawning. Every query starts a1000ms deadline before lookup/spawn and requires PGID=PID, fixed stdout/stderr and bounded cancellation/reap. No prewarm, retry-to-pass, script/interpreter substitution or deadline extension is used.

One approved7s System Trace yielded original test exit101. Exact failed child threads49083/10147983,49084/10147984,49086/10147986 each had a scheduler Blocked interval1000.103125,1000.093209,1000.066625ms. Their context-switch-associated kernel stacks reach procNotifyExecComplete →evaluateScript →waitForEvaluation →ASPEvaluationManager::waitOnEvaluation →lck_mtx_sleep. Samples occur just after the long blocked interval ends, not repeated before-deadline samples; cancellation may interrupt the wait and does not prove assessment completion. Parent spawn rounds below1ms; no output/exit is observed before deadline. Full128 outcomes are not directly recorded. Collection overhead may perturb timing; diagnostic success is not a repair.

This associates actual failed children with the script-policy wait, not an internal service request/compilation duration. Raw trace was deleted after owned numeric reduction and known-PID cleanup; full descendant enumeration remains unproved. Service-name-only frames were omitted. No security setting, policy exception, service restart, update or persistent privileged tool was applied.

Requested guidance:
1. Is there a supported correction for this exact fresh-script AppleSystemPolicy wait on25G241, preserving direct execution/1000ms/8x16/PGID/streams and first-launch state?
2. What bounded privacy-preserving observable can bind the child evaluation to its service request and partition queue/compiler/scan time?
3. If a specific XProtect or OS update addresses it, please identify the component/build and applicability. Available27.0.1 is not currently an evidenced fix; the AppleScript main-queue workaround is a different path.

The1000ms limit is the application's acceptance contract, not a universal macOS launch-time guarantee. This is the original PCTX integration reproducer; no standalone OS-only reproduction is claimed. Attached evidence is numeric/synthetic only; no raw systemwide trace, sysdiagnose, private paths or other application data is included.
