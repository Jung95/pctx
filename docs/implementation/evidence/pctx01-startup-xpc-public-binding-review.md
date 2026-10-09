# Public binding candidate review

PCTX01-G05-D05 remains **product_failure**. Whole PCTX01 totals remain33/40
details and0/10 gates. Completion still requires all four original macOS startup
failures resolved with original1s/8×16/native execution/group/streams conditions.
Static evidence does not close any condition. No original test, service query,
administrator authentication or product/platform modification occurred here.

## Pinned installed-file evidence

XprotectService SHA256 is
`d8cf6a1ac365a6c81ab695a6ec62ed914df54926497e8ea1b43315265ac7ba8c`,
arm64 UUID`57DD6284-09FD-351F-A87A-F60ECFB75F70`, unchanged from the prior capture.
The mapper resolves320 selector references and211 known ObjC methods. Pointer
reconstruction is specific to this installed image. Linear ADRP/ADD tracking,
register invalidation and nearest preceding IMP identify candidates; manual
call-site inspection establishes the conclusions below. Nearest IMP is not a
complete function-boundary proof, especially inside exported C functions.
[Mapping](pctx01-startup-xpc-public-binding-map.json),
[selected instructions](pctx01-startup-xpc-public-binding-selected-disassembly.txt).

| Candidate | Actual inspected use | Admission decision |
| --- | --- | --- |
| `samplingUUID` / `_sampling_uuid` | YARARule parses rule metadata `uuid`, initializes NSUUID at0x100001fa8 and stores it at object offset0x30. Getter0x100002348 and coder paths preserve that rule property. | Rule identifier; no assessment request binding established. |
| `sha256hash`, `xpProcessSha256`, `xpResponsibleSha256` | XProtectEventReporter `reportBehavioralDetection:` collects process hashes at0x100009ca0/0x100009d48, builds a dictionary and forwards it to feature-gated `com.apple.XProtectBridgeService` at0x10000a088. | Behavioral reporting data; no demonstrated normal successful-assessment log or phase marker. |
| `cdhash`, `sha256` in `reportGKAssessmentData:` | An assessment dictionary maps fields at0x10000972c/0x10000973c and goes to `AnalyticsSendEvent("com.apple.XProtect.assessmentResult2", ...)` at0x100009b64. | Internal analytics sink, not a demonstrated public request-ID/signpost join. |
| `XPUserInfoYARAScanErrorFileSHA256` | `YARAScanner scanURL:` calls `yr_scanner_scan_file` at0x100012ee0. A zero return branches at0x100012ee4 past hash construction. The nonzero error path loads the hash key0x100012fa0, hashes the file0x100012fb0, and constructs NSError userInfo. | Error-path diagnostic; cannot justify a successful-scan hash query. |

## GK metric caller and sink

The exported C function`_XPSendAssessmentMetric` begins0x1000049e8; the candidate
mapper's preceding `.cxx_destruct` label at its internal addresses is not the
function identity. XPAssessment `_sendReplyDictionary` gates metrics on result
<=2, computes NSDate `timeIntervalSinceDate:` using assessment offset0x68 at
0x1000106a0, and calls the metric function0x1000106e8. The metric dispatches event
type1 at0x100005210; `reportEvent:withData:` selects `reportGKAssessmentData:` at
0x10000a374, which ends in the analytics call above. This is a whole-assessment
measurement path, not a compiler-phase interval. The computed floating argument
alone does not prove that an exported event contains a duration.

The conditional `ess_notify_xp_malware_detected` call0x100009b3c is separate from
the analytics sink. Apple's public
[malware-detection event](https://developer.apple.com/documentation/endpointsecurity/es_event_xp_malware_detected_t)
contains detection/incident fields; it is not a public per-compiler interval API.
An Endpoint Security client also requires an
[Apple-granted entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.endpoint-security.client?changes=l_8_1&language=objc).
No client installation, entitlement request or event subscription was attempted.

Neither inspected sink establishes a public normal-scan log/signpost, unique
request bridge, emission/retention/visibility, or compiler start/end. This is a
bounded negative result for the inspected candidates, not absence across every
possible platform path. Candidate names do not authorize a wider runtime read.

## Verification and remaining dependency

Independent passive review confirmed the rule-property, reporting sink, metric
caller and scan-error guards. Product/test/lock hashes unchanged; mapper syntax
and pinned UUID verified. No PCTX acceptance test was rerun, no Actions consumed.
Temporary static disassembly/ObjC working files were removed after review;
selected installed-file evidence is retained.

Request-specific compiler attribution now needs a verified public timestamped
observable or vendor-supported instrumentation/correction. Current permissions
do not include private-log activation, other process observation or security
policy changes. No executable runtime diagnostic is prepared from these names.
A minimal factual vendor draft is prepared locally; submission/account actions
are separate from its preparation. D05 remains active and unresolved; do not
start a different official task because this observable is unavailable.
