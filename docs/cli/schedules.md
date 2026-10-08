# Local schedules and managed bridges

Schedules are durable control records. Reconcile plans occurrences; `tick` actually performs a bounded local job and records its result. Neither command calls a model. Internal notices remain queued until a recipient acknowledges them; queue insertion does not imply external delivery or a runtime wakeup.

## An isolated runnable example

Use an installed `pctx` binary or replace `pctx` below with an absolute path to the built binary. These commands use a temporary project and a separate data directory. They stage a fixture bridge and never register a job with your operating system.

```sh
sandbox=$(mktemp -d)
mkdir "$sandbox/project"
export PCTX_DATA_DIR="$sandbox/data"
export PCTX_ACTOR=owner
cd "$sandbox/project"
pctx init
cat > digest.json <<'JSON'
{
  "schema_version": 1,
  "namespace": "demo",
  "id": "owner-digest",
  "timezone": "Europe/Berlin",
  "cadence": {"kind": "daily", "at": "09:00"},
  "valid_from": "2024-01-01T00:00:00Z",
  "job": "read_query",
  "bridge": "managed",
  "role": "assistant",
  "recipient": "owner",
  "topic": "daily-digest",
  "enabled": true,
  "misfire": "coalesce_latest"
}
JSON
pctx schedule add --from-file digest.json --idempotency-key demo-digest
pctx --format json schedule plan --namespace demo owner-digest \
  --provider fixture --staging-root .pctx/demo-bridge > plan-envelope.json
python3 - <<'PY'
import json
from pathlib import Path
plan = json.loads(Path('plan-envelope.json').read_text())['data']
Path('reviewed-plan.json').write_text(json.dumps(plan, indent=2))
Path('reviewed-hash.txt').write_text(plan['plan_hash'])
PY
# Review reviewed-plan.json before installing it.
pctx schedule install --from-file reviewed-plan.json \
  --expect-hash "$(cat reviewed-hash.txt)"
pctx schedule inspect --namespace demo owner-digest
pctx schedule tick --namespace demo
pctx schedule tick --namespace demo
pctx schedule run-loop --namespace demo --interval-seconds 1 \
  --max-ticks 1 --ttl-seconds 5
pctx schedule uninstall --namespace demo owner-digest \
  --expect-hash "$(cat reviewed-hash.txt)"
```

An empty board produces a successful read without a notice. A changed nonempty board can queue one compact notice containing counts and a result reference. Repeating the same logical occurrence does not execute it again. A later unchanged board does not queue another notice. Board result receipts retain a bounded summary and a full result hash; `pctx board` rereads the current source.

For an elapsed interval, replace cadence with:

```json
{"kind": "interval", "seconds": 7200, "anchor": "2024-01-01T00:00:00Z"}
```

Daily times use the stored IANA zone. A repeated wall-clock minute runs once per local date; a missing wall-clock time moves to the next valid minute within 180 minutes. Intervals count elapsed seconds. Missed occurrences coalesce to the latest one; missed dates are never recorded as synthetic successes. `reconcile --at RFC3339` is useful for inspecting logical plans. `tick --at` cannot execute a future occurrence.

## Execution and policy

The registry supports these concrete job contracts:

| Job | Actual execution |
| --- | --- |
| `read_query` | In-process `pctx board` application function; bounded result summary. |
| `queue_digest` | In-process owner queue, at most five actions; unobserved time estimates remain unknown. |
| `backup` | Sanitized SQLite control snapshot through the existing backup implementation, under `.pctx/scheduled-backups`, with an exact trusted local decision. |
| `external_action` | No external execution without an observed provider/transport capability. A failed attempt records the capability failure; it never claims publication or a model call. |

Backup authorization binds action kind `backup`, environment `local`, resource and sole scope `schedule/NAMESPACE/ID`, amount `0`, empty `argv`, and the definition's role, topic and recipient. The referenced decision must match the complete action, policy and expiration, with trusted owner provenance. A successful backup exposes an opaque artifact reference; it does not overwrite an earlier snapshot. Use the decision CLI documented in the operations guide to review the exact action before enabling that schedule.

Schedule revision, workspace identity, enabled state, explicit pause, role pause, topic silence, exact session epoch and expiring owner decision are checked before execution and again before result/notification finalization. A pause detected after an action has run suppresses notification and records the observed effect without claiming that it was undone. Claim transactions commit before application reads and backup calls. Result and local mailbox insertion share the final writer transaction.

Registry mutation, tick and managed runtime calls require the local owner boundary (`PCTX_ACTOR=owner`). The actor environment is local operator configuration, not a remote authentication mechanism. An agent's assertion of approval cannot create a trusted decision.

Paused schedules remain paused across reconcile and restore. Managed execution requires a reviewed binding for the current definition revision, runtime binary, workspace and policy. Editing, pausing or resuming a definition changes its revision; generate and review a fresh managed plan before resuming managed execution.

```sh
pctx schedule list --namespace demo
pctx schedule pause --namespace demo owner-digest \
  --expect-revision 1 --reason 'Quiet operating window'
pctx schedule resume --namespace demo owner-digest \
  --expect-revision 2 --reason 'Resume local checks'
```

A failed completed attempt requires `tick --retry-failed`. Running and interrupted attempts block takeover regardless of elapsed time. `schedule recover` is an explicit owner operation for an in-process attempt whose original process is observed absent; a live process, permission-denied observation, restored foreign workspace or unsupported platform fails closed. It records failure and unknown prior success, then permits an explicit retry. It does not fabricate a result or free a surviving child.

## Native registration

`plan --provider launchd` on macOS or `--provider systemd` on Linux generates exact project-private manifests and binds the native runtime executable. Planning does not write files or register a job. Install without `--apply-native` stages owned manifests only. A fixture manifest describes an executable tick invocation; it is not an OS job.

Native registration is a separate explicit operator action:

```sh
# Generate and review a provider-specific plan first.
pctx schedule install --from-file reviewed-native-plan.json \
  --expect-hash REVIEWED_HASH --apply-native
pctx schedule inspect --namespace demo owner-digest --observe-native
pctx schedule uninstall --namespace demo owner-digest \
  --expect-hash REVIEWED_HASH --apply-native
```

Do not run these commands as part of the isolated example. They affect the selected user's OS scheduler and require authorization for that real installation. The installer validates the complete reviewed plan again, preserves conflicting user edits, executes native commands through the shared bounded process supervisor, and records captured native receipts. Partial registration remains unknown. A manifest alone is never labeled registered. Removal is limited to the exact managed label and verified files; unknown native registration cannot be silently removed by deleting a manifest.

The macOS bridge uses a user-domain launchd job with `ProgramArguments`, `WorkingDirectory`, explicit environment and a 60-second `StartInterval`. The Linux bridge uses a user oneshot service and a monotonic timer polling after completion. PCTX retains timezone and occurrence policy; the native bridge merely invokes tick. These interfaces follow [Apple's launchd job documentation](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html) and the [systemd timer contract](https://github.com/systemd/systemd/blob/main/man/systemd.timer.xml).

Native provider execution requires the host to allow the exact command and local user-domain access. Tests exercise the isolated fixture protocol, not registration in a real login session. Claude native schedules are adapter capabilities with session lifetime and observed list/cancel requirements; this module does not assume that an installed Claude runtime supports them. See the [official Claude scheduled-task documentation](https://code.claude.com/docs/en/scheduled-tasks).

Portable backups discard executable bindings and private installation paths. Restored installation state is unknown and requires a new reviewed plan; no OS registration or executable trust is inherited.

## Explicit bounded keep-awake

`run-loop` is a finite local scheduler process bounded by `--max-ticks`, `--ttl-seconds`, and an interval. On macOS, `--keep-awake --purpose 'Local scheduled check'` requests an owned IOKit idle-system-sleep assertion only while eligible local work remains. Pause and silence checks run during the wait in one-second slices. Runtime exit releases the assertion, and failed host acquisition/release is reported honestly. It does not request display wake or override lid-close, low-battery or explicit sleep behavior. The assertion also has a native timeout that turns it off if an in-process action outlives the requested TTL. See [IOPMAssertionCreateWithDescription](https://developer.apple.com/documentation/iokit/1557078-iopmassertioncreatewithdescripti) and [IOPMAssertionRelease](https://developer.apple.com/documentation/iokit/1557090-iopmassertionrelease?changes=_3).

Keep-awake is opt-in; the ordinary loop has no power-management effect. Other platforms report unverified native capability for this optional assertion rather than pretending it was acquired. No heartbeat uses repeated model prompts, global hooks or paid calls.
