# PCTX Offline Evaluation

Status: `measured_not_complete_product_validation`

This report measures local CLI behavior and emitted bytes. Provider tokens, billed cost, subscription quota, and model task success remain unknown.

| Metric | Observed | Target | Result |
| --- | --- | --- | --- |
| initial_index | 16303.404042 | 90000 | met |
| incremental_20_files | 2443.682083 | 3000 | met |
| metadata_search | 27.89125 | 250 | met |
| matched_symbol_search | 28.187084 | 700 | met |
| body_scan | 2581.989542 | 2000 | miss |
| strict_build | 3503.003333 | 15000 | met |
| cli_peak_memory | 38797312 | 536870912 | met |
| active_db_and_wal | 163426304 | 419430400 | met |
| exact_identifier_file_recall_at_20 | 1.0 | unknown | met |
| AC15_context_determinism | True | unknown | met |
| context_selection_serialization_only | None | unknown | unknown |
| wrapper_additional_latency | None | unknown | unknown |
| masked_1mib_postprocessing | None | unknown | unknown |
| model_input_token_reduction | None | unknown | unknown |
| model_task_success_noninferiority | None | unknown | unknown |
| new_session_recovery_token_reduction | None | unknown | unknown |
| idle_model_wakes | None | unknown | unknown |

Observed targets are not release certification: corpus/repetition eligibility and hardware equivalence are separately reported. See report.json and samples.json for errors and coverage.

## Scope qualification

This immutable binary matches seventh release SHA256b76af8e21ef0e1df24a9dff2527e7784f2d5bd3d813e6e986cd2393a8c810ee0 and source9c8b694. All30 metadata/matched samples succeeded. Metadata p95 improved from584.492083ms in the sixth observation to27.89125ms on the same declared corpus/repetitions; physical candidate authorization/hash checks remain mandatory. Body30/30 samples still exit3 partial with scan_limit, so2581.989542ms is not complete-body-scan latency. Unsupported-language labels0/10 measured; no complete accuracy claim. Exact20/alias10/partial10 labels measured Recall@20/MRR1 with zero wrong hashes; no-answer10 uses its own empty-answer denominator. Context30/30 deterministic within12000B. Current hardware equivalence/cold caches/model tokens/cost/quota/task outcomes remain unknown. Later tool-environment gate changes are not part of this binary.
