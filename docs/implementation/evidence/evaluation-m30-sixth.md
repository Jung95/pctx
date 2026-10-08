# PCTX Offline Evaluation

Status: `measured_not_complete_product_validation`

This report measures local CLI behavior and emitted bytes. Provider tokens, billed cost, subscription quota, and model task success remain unknown.

| Metric | Observed | Target | Result |
| --- | --- | --- | --- |
| initial_index | 16128.972792 | 90000 | met |
| incremental_20_files | 2459.95875 | 3000 | met |
| metadata_search | 584.492083 | 250 | miss |
| matched_symbol_search | 584.761292 | 700 | met |
| body_scan | 2589.320167 | 2000 | miss |
| strict_build | 3527.412875 | 15000 | met |
| cli_peak_memory | 38830080 | 536870912 | met |
| active_db_and_wal | 163594240 | 419430400 | met |
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

## Error and accuracy qualification

The sixth immutable binary produced partial body-scan results in all30 warm repetitions (exit3, scan_limit), so these timings do not prove complete body-search performance. Unsupported-language labeled queries were measured0/10; their Recall/MRR remain unknown. Exact20, alias10 and partial-name10 labels measured Recall@20/MRR1, with no wrong hashes; no-answer10 queries have no answer-bearing Recall denominator. Context30/30 remained deterministic and within12000bytes. Corpus10000files/200MiB/30repetitions, manifest810c9a60fa4090965a64e0dbf7caf166e0b7a595a4c522aa44c59e1503f0a66d. See immutable JSON and samples for detailed outcomes. Later candidate-first snapshot and keeper source changes are not measured by this binary.
