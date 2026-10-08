# Source topic delivery controls

A request's --topic describes the packet; it does not replace source classifications. Assign topics in .pctx/config.toml:

```toml
[[policy.source_topics]]
scope = ["src/public/**"]
topics = []

[[policy.source_topics]]
scope = ["src/legal/**", ".pctx/handoffs/legal.md"]
topics = ["legal-review"]
```

An empty list explicitly classifies matching sources as public. All overlapping project/user assignments contribute their union. A broad public assignment cannot remove a narrower restriction. Each of at most256 assignments has1..32 normalized relative glob scopes and0..32 topic labels, each up to256 UTF8 bytes. Malformed/sensitive/control-character labels or scopes fail with a bounded configuration error.

Managed Rule/Decision front matter may add topics. These declarations cannot remove owner-assigned labels or grant a reporting exception. Topic labels are authored metadata; PCTX does not infer them from source text. Under applicable silences, a legacy source without any explicit classification returns SOURCE_TOPIC_REQUIRED even when the packet declares an unrelated public topic. Existing roles, registered consumers and work capabilities still apply.

Build and ContextGet check applicable mandatory rules, current decisions and task source scopes. A mandatory conflict returns DELIVERY_POLICY_CONFLICT/exit7 without the source path/body/topic. Optional code/document candidates are removed before candidate limits and representation selection; a generic delivery_policy omission has an unknown count, no hidden path/title or retrieval reference, and marks selection incomplete. Strict completeness requests fail instead of claiming complete selection. Context graph expansions and input seed/scope provenance expose opaque fingerprints rather than potentially hidden edges or paths.

Build --handoff checks the handoff's classification before reading its task body and revalidates its source hash. Pack plan/create apply classification in metadata, signatures, selected and full modes, including managed document front matter and referenced task scopes. A public packet or broad public assignment cannot bypass a declared restricted decision.

Current project configuration is read through captured root authority, limited to1MiB and rechecked for identity/metadata changes. Effective user/project policy is compared with the captured policy, and original configuration presence is remembered for final checks. Missing configuration in an initialized registry is a concurrency error. Changes or deletion during selection reject delivery; individual native filesystem calls remain cooperative. Existing user-configuration input limits are a separate unfinished contract.

Serializer adaptive-context-v7 and selector adaptive-v6 preserve older stored data but require a compatible fresh full packet and explicit acknowledgement. Classification is not author authentication, receipt confirmation, permission restoration or a host-access grant. Original-policy-owner exceptions, explicit conflict priority, broader memory variants and platform/recovery qualification remain required.

The current task guard covers task-id source scope references and managed handoffs. Task-file/stdin/raw input provenance and broader approval/task-memory classification still require separate qualification; this source-context slice does not establish complete memory delivery enforcement.
