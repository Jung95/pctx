# Historical exact-owned-URL result and diagnostic privacy control

PCTX01-G05-D05 remains **product_failure**, whole PCTX01 totals are 33/40 details and 0/10 gates.
The approved scope was consumed once. No platform/product repair was implemented.

## Original-condition failure

One byte-identical original 1000ms/8×16 test exited101. Recorded outcomes:
98 total,96OUTPUT,2TIMEOUT(PID45676/45677);30 planned outcomes censored by the
failure. Test window2026-10-09T20:00:37.844719–20:00:43.892494UTC.
The Rust harness reports1.05s for the test body; the enclosing driver wall window
was6.047775s. Those measure different intervals and must not be substituted for
a failed call's1000ms budget. The two failed calls had spawn durations0.911/0.850ms and total entry-to-terminal
999.933/999.930ms. Maximum adjacent observation gaps6.280/6.340ms and all retained
traces had no dropped points. These do not establish an internal XP duration or
rule out every system pressure mechanism. Endpoint offset envelope was2000ns.
[Original outcomes](pctx01-startup-xpc-signpost-forward-result.json).

## Stored-log association

Exact-owned-URL query1 exited0 and returned2bytes (`[]`),0events. Query2 correctly
never ran. There is **no request association and no assessment interval**; this
is not absent XP activity or zero XP cost. No additional read or original-test
retry occurred. Possible false negatives include default-private object payload,
service bookmark/path rendering, emission/retention, ordinary-user visibility,
and publication timing. Parser guards could not cause this result because zero
rows arrived before parsing. [Query receipt](pctx01-startup-xpc-signpost-result.json).

## New own-only privacy/representation control

A distinct generated helper emitted four bounded historical signposts with an
NSURL containing a synthetic space. Its absoluteString and description both
matched Path.as_uri. The **default `%@` payload was `<private>`**, while the
**`%{public}@` payload matched the expected URL**.4394bytes read; no actual service
or original product test observed in this control. This directly tests a missing
prerequisite in the earlier public-C-string schema control, not a product fix.
[Source and receipt](pctx01-startup-xpc-signpost-url-control.json).

Installed service static code uses `%@` for its URL, guarded by
_os_signpost_enabled. The constructor creates subsystem`com.apple.xprotect`,
category`signposts` via_os_log_create at0x100011fc0:
[constructor](pctx01-startup-xpc-signpost-log-constructor.txt).
Apple documents default redaction of dynamic strings/objects:
[Generating Log Messages](https://developer.apple.com/documentation/os/generating-log-messages-from-your-code?changes=latest_5).
Taken together, this makes URL privacy a supported diagnostic obstacle. It does
**not demonstrate the service's actual runtime privacy/emission state**, establish
why the exact query was empty, or establish the cause of either startup timeout.
No private-log activation, logging configuration or protection change was made.

## Cleanup, review and next action

Original root/driver/log query reaped;101 known recorded owned PIDs absent before
private root removal. Own control compiler/helper/query reaped;3 known control
PIDs absent. Raw service messages were never retained; private mapping/source
removed. Product source, original test and lock hashes unchanged. Normal-source
build-only restoration succeeded8.67s; no extra original test run and no
qualification claim. [Cleanup](pctx01-startup-xpc-signpost-cleanup.json),
[normal build](pctx01-startup-xpc-signpost-normal-build.json).

Next within D05: statically assess an already-public file hash/request identifier
or supported vendor diagnostic that can bind the failed owned fixture without
private-log activation. Do not repeat the consumed exact-URL strategy or widen
service reads automatically. Whole-assessment markers would still not partition
compiler-only work. No Actions minutes or other official PCTX task activated.

Static follow-up found candidate names`samplingUUID`,`sha256hash`, and
`xpProcessSha256` in the installed universal binary. Names alone do not prove
public logging, request linkage, successful-path emission or API support.
[Candidate offsets](pctx01-startup-xpc-public-binding-candidates.json) are a
starting point for call-site review, not authorization to inspect more runtime data.

Independent final passive review found no material evidence or completion-claim
blocker. It confirmed the own-control/privacy distinction, cleanup receipts and
build-only label. D05 remains unresolved; counts above are whole PCTX01 totals.
