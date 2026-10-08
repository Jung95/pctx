# Source packs and consumers

Plan an explicit project-relative scope before creating a pack:

```sh
pctx pack plan --task-id TASK-ID --scope src/auth --content selected --session SESSION-ID --topic authentication --budget-bytes 64000
pctx pack create --plan PACKPLAN-ID --expect-hash HASH-FROM-PLAN --output artifacts/auth-context
pctx pack inspect artifacts/auth-context
pctx pack verify artifacts/auth-context --against current
```

An authenticated agent needs its current run capability and an owned registered session. The shared context delivery controls apply to metadata, signatures, selected and full representations before loading sources. The registered consumer role is authoritative. A paused role or silenced declared topic prevents planning and creation. If relevant silences exist, an omitted topic is rejected. Local owner planning may omit a session and remains subject to owner/wildcard and selected-role controls. A declared packet topic does not classify individual source contents.

Plans bind the consumer session, epoch, workspace and opaque delivery-control fingerprint. Creation rechecks these controls at entry, after validating sources and immediately before Unix atomic publication. A policy change followed by resume, or a session boundary, requires a new plan. Existing private schema1 plans return PACK_PLAN_STALE and must be regenerated; existing public schema1 artifacts remain readable and verifiable. New manifests include only an opaque delivery_barrier, without exporting the consumer session or topic label. Integrity verification never authenticates the author, acknowledges context or grants authority.

For pack create, --output names the artifact. Its response is emitted on stdout; it does not write a second response file at the artifact path. A .json destination creates one JSON artifact; another destination creates a directory. Actual serialization, including the delivery fingerprint, is included in byte limits. Token measurement remains unavailable.

Pack CLI and library calls use one cooperative 10s deadline by default; positive CLI --timeout-ms overrides are established before discovery and are not renewed by nested selected-context builds. Native filesystem and publication calls remain cooperative. A concurrent control change between the final check and filesystem publication is not transactionally excluded. Windows pinned pack publication and concurrent final-pass change qualification remain outstanding.

Direct library source-validation timeouts currently map to PACK_PLAN_STALE; the CLI normalizes an exhausted request deadline to TIMEOUT. Correcting the library classification remains required; neither outcome publishes a pack.
