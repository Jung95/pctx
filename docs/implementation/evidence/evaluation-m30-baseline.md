# PCTX Offline Evaluation

Status: `measured_not_complete_product_validation`

This report measures local CLI behavior and emitted bytes. Provider tokens, billed cost, subscription quota, and model task success remain unknown.

| Metric | Observed | Target | Result |
| --- | --- | --- | --- |
| initial_index | 16633.338417 | 90000 | met |
| incremental_20_files | 2822.097167 | 3000 | met |
| metadata_search | 1255.832459 | 250 | miss |
| matched_symbol_search | 2644.441417 | 700 | miss |
| body_scan | 2656.194333 | 2000 | miss |
| strict_build | 3958.731958 | 15000 | met |
| cli_peak_memory | 38928384 | 536870912 | met |
| active_db_and_wal | 162615296 | 419430400 | met |
| exact_identifier_file_recall_at_20 | None | unknown | unknown |
| AC15_context_determinism | True | unknown | met |
| context_selection_serialization_only | None | unknown | unknown |
| wrapper_additional_latency | None | unknown | unknown |
| masked_1mib_postprocessing | None | unknown | unknown |
| model_input_token_reduction | None | unknown | unknown |
| model_task_success_noninferiority | None | unknown | unknown |
| new_session_recovery_token_reduction | None | unknown | unknown |
| idle_model_wakes | None | unknown | unknown |

Observed targets are not release certification: corpus/repetition eligibility and hardware equivalence are separately reported. See report.json and samples.json for errors and coverage.
