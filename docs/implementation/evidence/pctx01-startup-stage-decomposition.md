# PCTX01-G05-D05 — small-stage code decomposition

Baseline: 13b7ed8, 2026-10-09. This is a read-only source and retained-evidence
review. No test, trace, service query, administrator authentication, platform
change or external submission was performed. D05 remains product_failure;
PCTX01 remains 33/40 details and 0/10 whole gates.

## Scope and completion

The user requested diagnosis in smaller code parts. These parts are investigation
boundaries within D05, not additional official tasks. Completion still requires
a justified correction and all four original startup contracts, plus required
platform verification. Preserve the original 1000ms shared deadline, 8 workers
with 16 planned calls each, direct OS execution, cwd/environment, PGID=PID,
combined stream bound, exit/group proof and cancellation/reap requirements.

## Actual source boundaries

| Part | Source boundary | Existing finding | Limit / next discriminating fact |
| --- | --- | --- | --- |
| A. Fixture creation | `tests/project_deadline.rs:145–153,567–598` | Each worker writes one mode0700 shebang script before its loop; that path is reused for 16 planned queries. | Creation is before that query's deadline. Concurrent filesystem/policy effects remain possible; the script is not a Mach-O fixture. |
| B. Parent admission | `src/query_process.rs:283–465,545–627` | Canonical cwd, executable/argument admission and environment/config assembly precede spawn. An earlier failed request recorded validation63us. | This timing belongs to an earlier run, not all requests. Git-status recursive preflight is absent from the failing rev-parse path. |
| C. OS spawn and group setup | `src/query_process.rs:628–651` | Standard process_group(0), then Command::spawn. The latest three failures report spawn_ms0, rounded to milliseconds; an earlier request measured628us. | Spawn returning is not proof of the child's first user instruction. No demonstrated one-second parent-spawn stall. |
| D. Child script-policy wait | Exact-child retained trace | Three exact failed children have approximately1000.07–1000.10ms Blocked intervals and associated AppleSystemPolicy evaluateScript/waitForEvaluation/waitOnEvaluation callchains. | Samples are just after those intervals end; cancellation may terminate the wait. Internal service request and assessment completion are not proved. |
| E. Policy request admission / queue residence | Earlier owned-hash policy timing; installed EvaluationManager serial workQueue | Earlier PID35071 scan starts913.425ms after birth; the scan takes185.029ms. Eight serial scans span1.092233s. | Birth-to-scan is not pure queue time: request preparation, delivery and scheduling may contribute. These numbers are not the latest three children's timeline. |
| F. Signature / policy scanner | Installed PolicyScanner performScan and SecStaticCodeCheckValidityWithErrors path | PolicyScanner call at0x10005b3ac; inspected implementation includes signature/policy helpers. | Static branches do not prove which helpers processed an owned script, nor their duration. |
| G. XProtect request / overlap | XPScanner performXPScan at0x10006b978; beginAnalysis call0x10006baac | The installed policy method can launch XP before synchronous PolicyScanner, or force a later XP launch, and join via semaphore0x10005b408. | F and G can overlap. Their elapsed times must not simply be added. No same-request inner-stage timing is retained. |
| H1. Rule compilation input | XprotectService call0x10000c9ec, yr_compiler_add_string | Earlier identity-admitted service sample mapped to this call. | Service activity, not an owned-request duration or proof that compilation caused this timeout. Rule loading/input cost remains unpartitioned. |
| H2. Compiled-rule assembly | XprotectService call0x10000cce4, yr_compiler_get_rules | Earlier identity-admitted service sample mapped to this call. | Inclusive sample weights32/24 are not milliseconds or CPU percentages; no request association. |
| H3. File rule scan | YARAScanner scanURL, yr_scanner_scan_file at0x100012ee0 | Installed successful branch skips the error-only file-SHA diagnostic. | The call exists; its owned-request execution and time are not measured. An error-only hash cannot bind successful scan stages. |
| I. Completion / reply | XP result callback; semaphore signal0x10005c020; XPAssessment _sendReplyDictionary | Static code joins XP completion before normal GK scan-complete. Reply metrics/signposts cover whole assessment. | No verified public compiler-start/end bridge. A rule samplingUUID is not an assessment request ID. |
| J. Script work / parent collection | Fixture kill/printf; `src/query_process.rs:653–761` | Collector drains nonblocking pipes, observes root exit, proves group emptiness, reaps and checks original deadline. Latest failures have no stream bytes/EOF/root exit. | No evidence that stream draining caused the child's kernel wait. Missing output alone does not establish every user instruction was absent. |
| K. Failure cleanup | `src/query_process.rs:763–785` | Timeout is returned; retained identity permits group/root SIGKILL and bounded reap observation. | Cleanup follows failure; its grace is not a renewed successful-query budget. Cleanup acceptance remains mandatory. |

Environment detail: fake_git sets env_clear on its template command, but this
concurrent test reconstructs only program/args. The product Unix output path
itself clears the inherited environment and installs explicitly collected entries
plus its Git safety settings: GIT_CONFIG_NOSYSTEM=1, GIT_CONFIG_GLOBAL=/dev/null,
GIT_CONFIG_COUNT=0, GIT_OPTIONAL_LOCKS=0 and GIT_PAGER=cat. Thus this is a controlled
five-variable environment, not an empty environment or a copy of template settings.

## Compiler input versus target-file input: inspected small part

Retained installed disassembly permits a further split without running code:

- At0x10000c8b0 the path creates a compiler. Its successful branch initializes
  a SHA256 context at0x10000c988, obtains UTF8String, hashes strlen+1 bytes at
  0x10000c9cc, then passes that string to yr_compiler_add_string at0x10000c9ec.
  This is compiler string input; the instructions do not establish that it is
  the launched shell script or that its digest is a public assessment identifier.
- A separate enumerated-input path reads a supplied path into NSString using
  initWithContentsOfFile:encoding:error: at0x10000cb38, then obtains UTF8String
  and hashes strlen+1 at
  0x10000cb88, then calls yr_compiler_add_string at0x10000cba8. The surrounding
  failure literal describes supplied YARA rules. Enumeration advances at
  0x10000ccc4 and branches back at0x10000cccc while objects remain. These are
  multiple possible rule inputs, not a proved per-query compilation count.
- After enumeration,0x10000cce4 calls yr_compiler_get_rules. This separates
  compiler input ingestion from compiled-rule extraction; neither duration nor
  cross-assessment reuse is established by this selected range.
- Separately, scanURL obtains the argument's fileSystemRepresentation at
  0x100012ed4 and passes it to yr_scanner_scan_file at0x100012ee0. That is the
  file-scan input boundary, distinct from the compiler UTF8String boundary.

These distinctions give three precise investigation targets rather than one
undifferentiated “XProtect is slow” claim. They do not prove which rule/input
was expensive, memory pressure, repeated compilation per launch, or a cache bug.
No rule contents or runtime inputs were collected. Evidence:
[compiler selected instructions](pctx01-startup-bundle-stack-xpc-selected-disassembly.txt),
[file-scan selected instructions](pctx01-startup-xpc-public-binding-selected-disassembly.txt).

## Narrowest unresolved boundary

The demonstrated stall is D, while its service-side cause is unpartitioned across
E–I. PCTX B/C/J changes are not currently justified as a remedy for that stall.
This does not declare those modules universally defect-free.

The next local investigation unit is **H1/H2 versus H3**: rule preparation and
compilation versus file scanning. Read-only installed call-site analysis can
further identify inputs, conditional reuse and surrounding calls without new
authentication. It cannot assign runtime cost to an owned request. A defensible
runtime attribution would need all of the following, before another capture:

1. A verified bridge from the exact generated file/evaluation to one service
   request, not a display name, rule UUID or matching sample weight.
2. Timestamped boundaries for that request's admission, compiler input/assembly,
   file scan and reply, with policy/XP overlap represented separately.
3. Exact service identity and bounded collection, preserving the original test
   conditions and recording cancellation/censoring rather than treating it as
   assessment completion.

Current retained public-observable review has not established that bridge.
Do not rerun a generic stack trace, activate private logging, substitute an
interpreter or modify platform binaries to manufacture it. All finite collection
approvals already consumed remain consumed. Any new runtime scope must be
prepared concretely and separately authorized. Vendor submission stays local
and unsubmitted; it is not a prerequisite for this static decomposition.

## Evidence and verification

- [Exact failed-child association](pctx01-startup-child-trace-review.md)
- [Earlier request timing](pctx01-startup-policy-forward-joined.json)
- [Policy / XP overlap and branches](pctx01-startup-scan-cost-review.md)
- [Installed call map](pctx01-startup-scan-cost-static.json)
- [Identity-admitted service call mappings](pctx01-startup-bundle-stack-operation-findings.json)
- [Public request-binding limits and file-scan call](pctx01-startup-xpc-public-binding-review.md)

Actual source was re-read for admission, spawn, drain, exit/group proof and
cleanup, together with the concurrent fixture. No product/test source changed;
no acceptance condition closed. Documentation links and source pins are checked
before integration. Passive independent /root/work_control review checked source
boundaries and retained call sites; its environment correction was applied.
Compiler/file-input distinctions passed independent review. Links resolve,
git diff --check passes, and source/test/lock SHA256 pins match the previous
executed baseline. No runtime test was needed for this documentation-only change.
