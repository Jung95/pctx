# PCTX01-G05-D05: repairs preserving the original conditions

Baseline: 4ec3ab9. Review only; no product/test edits, launch, service query,
administrator authentication, security change or platform update.
PCTX01-G05-D05 remains product_failure; fixed33/40 details,0/10 whole gates.

The directly joined original-supervisor failure has913.425ms from native birth
to scan start,185.029ms scan duration, and a result98.881–98.883ms after its
original deadline. All eight scans span1.092233s. Parent validation takes63us
and spawn628us in this failed record. A faster parent cannot obtain output
before the executable's policy evaluation permits execution. The observation
establishes this owned failure mechanism, not every original failure's cause.

| Candidate | Preserved conditions and decision |
| --- | --- |
| Optimize executable lookup/canonicalization/spawn | Compatible in principle, but measured sub-ms cost cannot account for this98.88ms deficit. No causal patch justified. |
| Faster polling/event-driven output | Compatible in principle; no payload/root exit exists at expiry. Cannot shorten the policy scan. Do not convert a monitoring improvement into startup closure. |
| Serialize or stagger admission | Waiting must consume each already-started original deadline; cannot remove the serial scan workload. Starting budgets after admission violates the contract. |
| Raise caller QoS | No demonstrated low-priority cause or supported direct control of service throughput. Installed scan queue is already created with0x21/USER_INTERACTIVE, relative priority0; no experiment justified merely by generic scheduling advice. |
| Direct interpreter, exec through a wrapper, native replacement fixture | Changes the stipulated direct execution/fresh-script workload; not admissible. |
| Prewarming/reusing scans or fixtures, retry-to-pass | Changes first-launch state or hides failures; not admissible. |
| Sign/notarize/alter attributes or policy exclusions | Changes executable/security conditions and is not an evidenced product repair; not applied. |
| Host/software correction with security preserved | Potentially compatible, but no matching supported fix/version is established. Do not install/restart/update policy services or claim improvement from unrelated updates. |

Queue QoS proof uses existing static evidence only: address0x100057f1c loads
w1=0x21 before serial builder0x100015344; builder preserves that class and
calls dispatch_queue_attr_make_with_qos_class with relative priority0.
Local SDK sys/qos.h maps0x21 toQOS_CLASS_USER_INTERACTIVE. This is the queue's
configured class, not a live scheduling measurement or proof against starvation.
Apple describes QoS as resource/scheduling prioritization, not an execution
latency guarantee. [Apple QoS documentation](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/EnergyGuide-iOS/PrioritizeWorkWithQoS.html).
Apple's launch-constraint documentation describes OS checks at exec/spawn;
it does not identify a supported caller switch to parallelize this scan queue.
[Apple launch constraints](https://developer.apple.com/documentation/security/defining-launch-environment-and-library-constraints).

Independent review /root/work_control finds no supported causal PCTX patch.
This bounded review does not prove that no solution exists. Next investigation
within this task is the operation dominating policy scan duration and whether
an applicable platform correction exists. Existing finite service-read/sample
scopes are consumed; any new operational-service collection requires a concrete
fresh scope. Static installed-code review and owned-process diagnosis remain
possible without changing security. No unchanged test rerun is justified now.
No vendor message is sent without explicit authorization.

A candidate needs a concrete causal mechanism and baseline/candidate evidence,
then all four byte-identical original tests: original pre-operation1000ms,
fresh fixtures,8x16,native exec,cwd/environment,PGID ownership,stdout/stderr,
overflow and cancel/reap, with every failure retained. Prove no budget renewal,
prewarm/retry, fixture substitution or security exemption. Required platform
acceptance remains separate and unverified. A diagnostic PASS is not closure.
