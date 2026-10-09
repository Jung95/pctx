# PCTX01-G05-D05 — supported correction applicability, 2026-10-09

Only D05 is active/product_failure; PCTX01 remains33/40 details,0/10 gates. This review uses exact-child evidence and public Apple sources, plus installed component versions and a bounded read-only update listing. No new product test, trace, log/service observation, download, installation, setting change or vendor submission.

## Applicability decisions

| Candidate | Evidence and decision |
| --- | --- |
| AppleScript main-thread initialization /26.3 change | Apple DTS thread810258 diagnoses in-process NSAppleScript/Xojo main-queue initialization. The suggested trivial-script initialization is a workaround, and later asks users to reproduce on26.3. PCTX is a child shebang-script kernel evaluateScript wait; current26.7.1 already exceeds26.3. Different mechanism; priming also violates the frozen first-launch conditions. |
| General XProtect data updates | Apple says signatures and background protection components update independently of OS updates. Installed XProtect data5366/remediator163 were read from fixed Info.plists; these numbers are not proof that they are latest or causal. Reviewed sources do not identify a matching latency correction or caller API to control the wait. No background/forced update requested. |
| macOS27.0.1 | Read-only softwareupdate list returns27.0.1 build26A5434,17929043KiB,requires restart. Apple's27.0.1 public summary says bug fixes; enterprise notes mention a Platform SSO fix. Neither identifies this script-policy latency correction. Availability is not applicability; no installation or fix claim. |
| Stay on26.x patch | Apple's current version table lists26.7.1 for Tahoe26, matching installed26.7.1/25G241. The reviewed security bulletin identifies a CoreGraphics fix, not this wait. No newer26.x product appeared in this bounded macOS product listing. This is not an exhaustive catalog or proof no future fix exists. |
| syspolicy_check distribution/signing guidance | Apple's trusted-execution guide checks application distribution/signature problems. It does not demonstrate a latency correction for fresh direct shebang execution. Do not replace performance evidence with a distribution-check result or prime the original fixture. |
| Parent spawn/polling/QoS or fixture replacement | Existing preserved-condition review plus latest child evidence provide no causal PCTX patch: rounded sub-ms spawn, no bytes/exit at expiry, approximately1s child policy block. Budget renewal, warmup, lower concurrency, interpreter/helper substitution, signing/attribute exemptions and bypasses are excluded. |

Sources (reviewed2026-10-09): [Apple DTS AppleScript diagnosis](https://developer.apple.com/forums/thread/810258), [background updates](https://support.apple.com/en-us/101591), [malware protection](https://support.apple.com/guide/security/protecting-against-malware-sec469d47bd8/web), [current OS versions](https://support.apple.com/en-us/109033), [27.0.1 summary](https://support.apple.com/en-us/127257), [enterprise notes](https://support.apple.com/en-us/148830), [26.7.1 security content](https://support.apple.com/en-us/149228), [trusted execution guidance](https://developer.apple.com/forums/thread/706442).

No applicable correction was established within these sources. This bounded search does not prove there is no solution. The exact wait mechanism is supported; inner policy-service request/compiler/memory cause and a correction remain unknown. Required acceptance is all four original contracts and supported-platform proofs; no conditions are weakened and no task is closed.

## Concrete vendor payload

`pctx01-startup-platform-report.zip` contains four UTF-8 files: report.md, observations.json, reproduction.md, fixture-source.rs.txt. Data is limited to already retained owned PID/TID/imageUUID/offset/state evidence, OS/component versions, synthetic source, and public repository links. It excludes raw trace/logs/sysdiagnose, other-app frames/names/paths, username/home paths, serial numbers and credentials. Contents and hashes are in the payload manifest; local preparation does not submit anything.

Proposed recipient: Apple Feedback Assistant web at https://feedbackassistant.apple.com/ for one macOS script-startup performance report, exact four-file payload only. Apple's Feedback Assistant documentation says the native app automatically attaches sysdiagnose, while the web supports manual ZIP upload; use the web for this minimized scope. Login/form availability is not yet tested. Additional mandatory diagnostics would require explicit separate scope; do not launch the native app or silently add diagnostics.

Next within D05: explicit human authorization to send this concrete report, then inspect Apple's supported correction or diagnostic guidance. No unsupported OS change or generic trace retry is justified. The development/commit approval does not explicitly authorize contacting an external vendor.
