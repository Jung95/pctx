# PCTX01 complete visible command admission register

Baseline d0dbaef; specification §14/§38; PCTX01-G03 only. Actual native help
traversal observed172 paths including140 visible leaves without project/data
creation. The binary hash and every help hash are in evidence/pctx01-admission-registry.json.
Hidden guardian is a separate internal route; library-only contracts are recorded
below. A help traversal proves enumeration, not semantic completion. This register
replaces incremental discovery as the next-action source; the prior A01–A11 list
is a historical bounded qualification, not an exhaustive G03 claim.

Public-command denominator:140 leaves, stable C001–C140 at this revision. This is
an audit denominator, not completed acceptance gates. Official PCTX01 remains
closed0/10, denominator unchanged. No independent business features added.
Source review classified every leaf by producer family and separated raw-input,
authority/state and pure grammar. R01–R12 below freeze currently identified
residual groups; R01–R12 now have local evidence (12/12 groups). None is whole
required-platform qualified. Native Linux/Windows and broader common gates remain.

| Residual ID | Existing pure contract / exact source boundary | Status and scope |
| --- | --- | --- |
| R01 | operations Owner Queue limit1..1000 (1223), Inbox limit1..1000/nonnegative cursor (1402) | Local native60 matrix/six direct expiry/bounds/alias/non-owner proof, related48PASS; pctx01-operations-admission-verification.json; required platforms open |
| R02 | operations Message Send exactly one recipient/label (1557), Resolve evidence label (1366) | Local native60 matrix/six direct expiry/bounds/alias/non-owner proof, related48PASS; pctx01-operations-admission-verification.json; required platforms open |
| R03 | downstream policy_controls Role label/optional topic/recipient and recipient-needs-topic (38/291), bounded reason4096 (49); raw Operations recipient alias is state-dependent | Local native60 matrix/six direct expiry/bounds/alias/non-owner proof, related48PASS; pctx01-operations-admission-verification.json; required platforms open |
| R04 | work Agent Report stage/summary4096/percentage100 (1487) | Local native36 matrix/six direct expiry/accepted report stages and replay, related51PASS; pctx01-report-admission-verification.json; lease/order remain stateful; platforms open |
| R05 | session Suspend/Boundary reason labels (339/354); Context Ack provenance (691) already Clap-protected | Local native36 matrix/six direct expiry/accepted1024-byte reasons, related51PASS; pctx01-report-admission-verification.json; Ack provenance validator/direct refusal only; platforms open |
| R06 | quota Ingest idempotency-key label (414) | Local native36 matrix/six direct expiry/actual key256 ingestion+replay/non-owner proof, related51PASS; pctx01-report-admission-verification.json; input schemas/windows remain producer-owned; platforms open |
| R07 | inventory Profile/Audit/optional Scan profile reader lexical paths (reader94; inventory361/1046); supplied-expiry priority scan553 | Local native60 CLI/six-entry direct path+expiry/actual accepted and policy/baseline proofs, related40PASS; pctx01-inventory-admission-verification.json; path5 preserved; platforms open |
| R08 | output binding argv nonempty/count256/bytes65536 (525), Run modes already Clap-protected | Local native24 CLI/direct invalid+expiry/isolated nonowner and actual TrustPlan boundaries, related34 outerPASS; pctx01-execution-admission-verification.json; binding/authority/state remain producer-owned; platforms open |
| R09 | filter Activate private identifier (727), Explain argv1..256 (1244), non-UTF8 explicit paths (relative helper208) | Local native32 CLI/nine direct expiry/actual ID+path+absolute preview/Explain256 and state-denial proofs, related18PASS; pctx01-filter-admission-verification.json; Apply ID-or-path preserved, absolute opacity deferred; platforms open |
| R10 | adapter Plan/Event agent, Uninstall plan, Statusline task/pool/session/counter/key labels; Event explicit key is input-dependent | Local native36 CLI/nine direct expiry/isolated nonowner/bounds/actual PreToolUse key exemption and receipt replay, related26PASS; pctx01-adapter-admission-verification.json; Event key validates after parse/PreToolUse branch and before receipt lock/DB; platforms open |
| R11 | schedule Add key, Update/Remove/Recover labels/reason, List/Reconcile/Tick namespace, RFC3339 --at, RunLoop intervals/ticks/TTL/conditional purpose | Local native92 refusal combinations/23 direct+expiry/nonowner/bounds/Add256 replay and no-tick namespace exception, related28PASS; pctx01-schedule-admission-verification.json; RunLoop namespace conditional, Inspect/Uninstall unrestricted lookup keys, future-time/ownership remain producer-owned; platforms open |
| R12 | runner job/helper IDs and HelperRequest mode/scope count/nonsecret/nonlocal explicit scope (job_path/helper_read_path/helper_request) | Local44 refusal combinations/direct expiry/boundaries/non-owner/queued policy proofs,44 relatedPASS+intent1PASS+timeout/control21PASS;3 loss failures/exact restore;staticPASS; no whole platform closure |

Read-only independent audit /root/work_control inspected all Operations/Work/
Session/Context/Quota/Schedule/Inventory routes. Parent inspected main plus
adapter/filter/runner/output/storage/search/extract/context/graph/broker dispatch
and validators. Existing source is authoritative; source references above are
base-revision locations. This is static discovery evidence; tests do not yet
qualify open rows. Work Check Run missing-key check is already Clap protected
and before module DB access but remains outside its shared validator; record it
under R04 as a library-only residual. Raw input schema/count/content, task/check/
criterion existence, leases, registered IDs, workspace/current policy, owner,
CAS/revision, approvals, installation capability and source-scope policy require
actual input/state and remain in their producers. No fabricated pure label rules
for currently unrestricted lookup keys.

| Stable leaf ID | Visible route | Producer | Admission audit |
| --- | --- | --- | --- |
| C001 | `activity` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C002 | `adapter claude doctor` | src/adapter.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C003 | `adapter claude event` | src/adapter.rs | R10 |
| C004 | `adapter claude install` | src/adapter.rs | G03-C004 local admission proof: exact existing provided hash length/hex/equality before discovery/lock, pctx01-install-admission-verification.json; stored bytes/identity stateful; required platforms/full matrix open |
| C005 | `adapter claude plan` | src/adapter.rs | R10 |
| C006 | `adapter claude protocol-fixture` | src/adapter.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C007 | `adapter claude statusline` | src/adapter.rs | R10 |
| C008 | `adapter claude uninstall` | src/adapter.rs | R10 |
| C009 | `adapter claude verify` | src/adapter.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C010 | `agent heartbeat` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C011 | `agent list` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C012 | `agent register` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C013 | `agent report` | src/work.rs | R04 |
| C014 | `agent show` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C015 | `board` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C016 | `build` | src/context.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C017 | `cache stats` | src/broker.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C018 | `changes` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C019 | `check begin` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C020 | `check list` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C021 | `check plan` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C022 | `check record` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C023 | `check run` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C024 | `check show` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C025 | `checkpoint create` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C026 | `checkpoint delete` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C027 | `checkpoint list` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C028 | `context ack` | src/session.rs | R05 |
| C029 | `context get` | src/session.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C030 | `control backup` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C031 | `control restore` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C032 | `decision record` | src/operations.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C033 | `decision request` | src/operations.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C034 | `decision show` | src/operations.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C035 | `doctor` | src/main.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C036 | `extract` | src/extract.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C037 | `filter activate` | src/filters.rs | R09 |
| C038 | `filter apply` | src/filters.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C039 | `filter explain` | src/filters.rs | R09 |
| C040 | `filter test` | src/filters.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C041 | `filter validate` | src/filters.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C042 | `find` | src/search.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C043 | `handoff create` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C044 | `handoff show` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C045 | `handoff update` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C046 | `impact` | src/graph.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C047 | `inbox read` | src/operations.rs | R01 |
| C048 | `index gc` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C049 | `index rebuild` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C050 | `index update` | src/storage.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C051 | `init` | src/project.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C052 | `inventory audit` | src/inventory.rs | R07 |
| C053 | `inventory profile` | src/inventory.rs | R07 |
| C054 | `inventory scan` | src/inventory.rs | R07 |
| C055 | `job cancel` | src/runner.rs | R12 |
| C056 | `message ack` | src/operations.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C057 | `message resolve` | src/operations.rs | R02 |
| C058 | `message send` | src/operations.rs | R02 |
| C059 | `outline` | src/search.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C060 | `output find` | src/output.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C061 | `output render` | src/output.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C062 | `output show` | src/output.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C063 | `owner queue` | src/operations.rs | R01 |
| C064 | `pack create` | src/pack.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C065 | `pack inspect` | src/pack.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C066 | `pack plan` | src/pack.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C067 | `pack verify` | src/pack.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C068 | `policy attest-owner` | src/operations.rs / policy_controls.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C069 | `policy evaluate` | src/operations.rs / policy_controls.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C070 | `policy exception-record` | src/operations.rs / policy_controls.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C071 | `policy exception-revoke` | src/operations.rs / policy_controls.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C072 | `policy release` | src/operations.rs / policy_controls.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C073 | `policy report-evaluate` | src/operations.rs / policy_controls.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C074 | `policy report-fingerprint` | src/operations.rs / policy_controls.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C075 | `query` | src/search.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C076 | `quota ingest` | src/quota.rs | R06 |
| C077 | `quota plan` | src/quota.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C078 | `quota reconcile` | src/quota.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C079 | `quota release` | src/quota.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C080 | `quota report` | src/quota.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C081 | `quota reserve` | src/quota.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C082 | `read` | src/search.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C083 | `refs` | src/graph.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C084 | `repo status` | src/broker.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C085 | `resource status` | src/runner.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C086 | `role list` | src/operations.rs / policy_controls.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C087 | `role pause` | src/operations.rs / policy_controls.rs | R03 |
| C088 | `role resume` | src/operations.rs / policy_controls.rs | R03 |
| C089 | `run` | src/output.rs | R08 |
| C090 | `runner check-plan` | src/runner.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C091 | `runner check-run` | src/runner.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C092 | `runner helper-cancel` | src/runner.rs | R12 |
| C093 | `runner helper-release` | src/runner.rs | R12 |
| C094 | `runner helper-request` | src/runner.rs | R12 |
| C095 | `runner helper-status` | src/runner.rs | R12 |
| C096 | `runner job-cancel` | src/runner.rs | R12 |
| C097 | `runner resource-status` | src/runner.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C098 | `runner trust` | src/runner.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C099 | `savings opportunities` | src/output.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C100 | `savings report` | src/output.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C101 | `schedule add` | src/schedule.rs | R11 |
| C102 | `schedule inspect` | src/schedule.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C103 | `schedule install` | src/schedule.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C104 | `schedule list` | src/schedule.rs | R11 |
| C105 | `schedule pause` | src/schedule.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C106 | `schedule plan` | src/schedule.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C107 | `schedule reconcile` | src/schedule.rs | R11 |
| C108 | `schedule recover` | src/schedule.rs | R11 |
| C109 | `schedule remove` | src/schedule.rs | R11 |
| C110 | `schedule resume` | src/schedule.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C111 | `schedule run-loop` | src/schedule.rs | R11 |
| C112 | `schedule tick` | src/schedule.rs | R11 |
| C113 | `schedule uninstall` | src/schedule.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C114 | `schedule update` | src/schedule.rs | R11 |
| C115 | `session attach` | src/session.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C116 | `session boundary` | src/session.rs | R05 |
| C117 | `session reconcile` | src/session.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C118 | `session suspend` | src/session.rs | R05 |
| C119 | `status` | src/main.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C120 | `task assign` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C121 | `task block` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C122 | `task cancel` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C123 | `task claim` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C124 | `task complete` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C125 | `task create` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C126 | `task criterion accept` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C127 | `task edit` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C128 | `task list` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C129 | `task pause` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C130 | `task ready` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C131 | `task reassign` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C132 | `task reopen` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C133 | `task resume` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C134 | `task review` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C135 | `task show` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C136 | `task start` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C137 | `task submit` | src/work.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C138 | `trace` | src/graph.rs | Existing parser/shared admission or state/input-dependent checks; whole matrix open |
| C139 | `trust add` | src/output.rs | R08 |
| C140 | `trust plan` | src/output.rs | R08 |
