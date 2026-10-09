# PCTX01 reverse comparison input recovery

**Consumed and complete:** closed3PASS/1FAIL,open3PASS/1FAIL. Do not rerun the
command below. See [verification](evidence/pctx01-startup-host-reverse-recovery-comparison-verification.json).
The remaining procedure is historical.

The previous reverse run stopped after four closed-phase passes because the keyboard
was set to Korean at OPEN. Its results remain;the comparison was incomplete. This
new finite comparison uses the same frozen source/binary and fresh session. Both
phases must share one live runner environment,so the exited prior runner cannot
continue its missing phase. Each original test runs once per phase and every failure
is retained. No root/auth,app automation,security change,deadline extension or
interpreter substitution. This recovers an interrupted protocol,not a product repair.

In an external Terminal run:

```sh
/opt/homebrew/bin/python3 /Users/dev/PCTX/docs/implementation/evidence/pctx01-startup-host-reverse-recovery-run.py.txt --ready /Users/dev/PCTX/docs/implementation/evidence/pctx01-startup-host-reverse-recovery-ready.json --output-prefix pctx01-startup-host-reverse-recovery-comparison
```

1. Enter `EXTERNAL TERMINAL`.
2. Quit Codex normally with ⌘Q,keep Terminal open,and enter `CLOSED`.
3. When prompted after the first four tests,reopen Codex without sending a task or
   starting other development jobs. Enter `OPEN` in Terminal.
4. After the second four tests finish,tell this chat the recovery completed.

Wrong keyboard input now shows an explanation and asks again without launching a
test or exiting. Lowercase/extra outer spaces are accepted. `STOP` explicitly ends
it;end-of-input also ends it. Do not repeat a consumed session. Unexpected runner
loss invalidates cleanup assumptions. Host state remains human-declared;order/time,
cache/contention/reopen/helper confounds remain. No repair or acceptance closure
follows from one passing run. PCTX01 remains33/40details,0/10whole gates.
