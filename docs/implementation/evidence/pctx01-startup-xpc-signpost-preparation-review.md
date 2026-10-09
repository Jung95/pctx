# Exact-owned-URL historical assessment preparation

PCTX01-G05-D05 remains **product_failure**, whole PCTX01 totals are 33/40 details and 0/10 gates.
No product fix, original test execution, service observation or administrator
request occurred in this preparation. Original acceptance remains 1000ms,
8×16 concurrency, native execution and all stream/group/cancellation contracts.

## Evidence and hypothesis

The installed XprotectService routine emits a normal `XprotectAssessment` begin
at 0x10000f14c, a `URL` event at 0x10000f260, and a normal end at 0x10000fb60.
All use the assessment object's signpost ID at field offset 0x60. The URL payload
comes from its URL field at offset 0x20. See
[static disassembly](pctx01-startup-xpc-phase-signpost-disassembly.txt).
These markers delimit the **whole assessment**, not YARA compiler time. The
prior stack evidence identified compiler preparation at service level but did
not bind it to a failed owned request. A unique exact fixture URL and service
PID/signpost ID may provide a contextual request association; it is not a public
kernel child-PID binding. No supported compiler-only successful interval marker
was identified in the inspected routine.

SIP is enabled on this host. Apple documents that DTrace cannot inspect protected
system processes even with administrator rights:
[Runtime Protections](https://developer.apple.com/library/archive/documentation/Security/Conceptual/System_Integrity_Protection_Guide/RuntimeProtections/RuntimeProtections.html).
No debugger attachment, injection, protection change or persistent privileged
service was attempted or prepared.

## Rejected transport control

The generated owned native helper successfully emitted signposts, but the new
bounded live listener retained **0 events / 180 bytes**. Its result is
`AssertionError`, not a pass. Earlier compile-only attempts failed before helper
launch due to source suffix and SDK selection; those were corrected for this
control. Helper PID45151 and listener PID45152 were reaped and are now absent;
private control files and raw input were removed. No operational service was
observed. See [control receipt](pctx01-startup-xpc-signpost-native-control.json).
The unapproved live service collector was deleted rather than retained as an
alternative runnable entry point. Existing historical owned-helper schema
control returned four events on this installed host; that does not establish
service URL visibility or stored-event retention.

## Prepared historical method

Build-only preparation succeeded in 11.75s. The isolated original test source is
byte-identical; instrumentation writes a mode0600 private exact-fixture mapping
after original cleanup. Prepared root:
`/private/tmp/pctx01-xpc-signpost-forward-li2i8byo` (owned UID502, mode0700).
No consumed marker or original test run exists. The shared target directory now
contains a diagnostic binary; rebuild from normal sources before qualification.

After one original test, validate driver root reap and absence of every known
recorded owned PID before any service query. Read stored signposts in two steps:
first exact owned synthetic URL equality, then only admitted service PID/ID
normal and error boundaries. At most two reads, one <=7s historical window,
shared 30s query budget, 128KiB and 256 events; stop without fallback if no URL
matches. URLs/messages are transient and never publicly exported. Public
intervals contain owned PID/path digest, service PID/ID, timestamps and outcome.

Missing/redacted URLs, duplicate IDs, ambiguous owned lifetimes, missing/error
boundaries, cap violations and cleanup uncertainty refuse attribution. Endpoint
clock offset envelope must be <=1ms; this does not prove uninterrupted clock
stability. An absent association is unknown, not zero assessment cost. Even a
valid interval cannot partition compiler/scan/other work or prove a product fix.

The parser's exact URL, exclusive lifetime, missing/redacted/source and multiple
ID refusal controls pass. Python sources compile. Review required fixing selector
ownership, checking the shared deadline before each spawn, and restoring the
owned cleanup gate. Independent final passive review approved the repaired method and verified pinned
files, original test identity and instrumentation hashes. Fresh human scope approval remains pending because this
reads URL signposts from the operational service rather than numeric stack frames.
[Concrete scope and pinned files](pctx01-startup-xpc-signpost-scope-plan.json).
No Actions minutes consumed; no other official task activated.
