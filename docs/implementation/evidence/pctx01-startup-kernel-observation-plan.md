# PCTX01-G05-D05: proposed privileged observation

Status: authorization pending; no privileged sampling executed. PCTX01 remains
implementing, G05-D05 product_failure, details33/40 and whole gates0/10.

The completed owned-child diagnostic and independent review are in
[pctx01-startup-thread-snapshot-verification.json](pctx01-startup-thread-snapshot-verification.json).
The ordinary-user capability check returned77:
`spindump must be run as root when sampling the live system`.

The proposed action is one bounded read-only trace of one newly created inert
PCTX diagnostic child, with its PID, parent, uid and native start time validated.
Its parent retains the unreaped identity through the observation. The child uses
the original1000ms clock and is cancelled on expiry; diagnostic observation and
cleanup do not extend or qualify that product execution budget.

The sampling command, after the actual owned PID is known, is:

```text
sudo /usr/sbin/spindump <validated-owned-child-pid> 1 5 -onlyTarget -noBinary -noIPC -timelimit 2 -o /private/tmp/pctx01-owned-child-kernel.txt
```

Only that isolated child is sampled; do not use the default whole-system capture,
-noTarget, -sampleWithoutTarget, another user's process, debugger attachment,
security-policy/entitlement changes or a root-owned replacement workload. Keep
pre-expiry samples and exact identity/time correspondence separate from observer
perturbation and cancellation. Report unavailable stack/wait cause honestly. A
later within-budget attempt alone cannot close the original4 startup failures.

Administrator capability must be supplied through the host's normal authorization
flow; never request or store a password in chat or evidence. Authorization for this
trace does not authorize main push, Actions, other processes or OS policy changes.
