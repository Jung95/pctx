# Reproducible Evaluation

PCTX evaluation separates local correctness, emitted bytes, CPU/disk/memory costs, actual model tokens, provider cache usage, API-cost estimates, subscription quotas, and task success. A reduction in local bytes is not evidence of lower billing or quota consumption. The normative reference is [specification Sections 18–19, 38, and 45](PCTX-implementation-spec-v0.6-en.md). The owner-provided Korean specification remains authoritative.

The offline harness invokes an **existing compiled PCTX executable** with its real CLI, isolated project root, user-private temporary data directory, and synthetic inputs. It neither builds PCTX nor launches models, reads credentials/private conversations, registers host schedules, publishes artifacts, or calls paid providers. It does explicitly trust exact local `cat` fixture invocations inside the disposable benchmark environment. Functional workloads run after corpus performance measurements, so their extra files do not silently inflate performance denominators.

## Corpus and Reference Conditions

| Profile | Policy-allowed generated text files | Generated source bytes | Meaning |
| --- | --- | --- | --- |
| S | 1,000 | 20 MiB | Functional/cold-start observations |
| M | 10,000 | 200 MiB | Mandatory performance corpus |
| L | 100,000 | 2 GiB | Scalability observation; not mandatory MVP acceptance |

The specification does not prescribe a 20,000-file normative corpus. Reference hardware is eight CPU cores, 16 GiB RAM, local SSD, and offline operation. Compare operating systems separately. At least 30 measured warm repetitions follow a preparation invocation. A different host can provide useful observations but cannot certify reference-hardware equivalence without separate evidence.

`generate-corpus.py` uses seed 1048 by default. Exactly 60% of files use supported Python, JavaScript, JSX, TypeScript, and TSX, distributed evenly; 20% are Markdown and 20% unsupported-extension UTF-8 text. Every supported-language file declares one function, one class, and one method. Each Markdown file has two headings. The generator records counts, source hashes, symbol distributions, line statistics, maximum file size, exact bytes, and a generation-manifest digest. Padding consists of valid comments or document/text lines; bodies contain no real secrets or dependencies.

Files receive floor(total bytes/file count), with the remaining bytes assigned deterministically to the first files. S/M files average approximately 20 KiB, below the default 1 MiB reader limit. A seed changes padding and labeled sample selection reproducibly. A corpus with overridden count/size is explicitly nonnormative. Labels stay outside the scanned project; PCTX-generated configuration/glossary are separately identifiable runtime metadata.

The 60 labeled queries comprise 20 exact identifiers, 10 partial names, 10 Korean aliases, 10 absent identifiers, and 10 unsupported-language text markers. Expected files and exact function names are generated from known declarations rather than inferred from returned success messages. Exact-identifier Recall@20 targets 100%; category-specific Recall@20 and MRR are reported separately. No-answer queries have empty-result checks rather than undefined recall converted to zero or one.

## Running the Harness

Use the already-built binary selected for evaluation. The following smoke run is small and **cannot satisfy normative performance acceptance**:

```bash
python3 scripts/benchmark.py \
  --cli ./target/release/pctx \
  --config tests/evaluation/default.json \
  --smoke \
  --output /private/tmp/pctx-evaluation-smoke
```

A reference-shaped run uses the default M corpus and 30 measured repetitions:

```bash
python3 scripts/benchmark.py \
  --cli ./target/release/pctx \
  --config tests/evaluation/default.json \
  --output /private/tmp/pctx-evaluation-m
```

Run only one heavy build/test/benchmark at a time. The evidence output directory must not already exist. Temporary inputs are created with the `.pctxbench-` prefix beneath canonical `/private/tmp` on macOS, or the platform temporary directory elsewhere, and removed afterward. `--keep-corpus` retains them for investigation; no private credentials are copied. The report records retention without exposing an absolute source path by default.

`--config` is a **harness JSON configuration**, not a PCTX CLI flag. Supported keys are schema_version, profile, seed, repetitions, context_budget_bytes, extract_budget_bytes, timeout_seconds, output_fixtures, session_workloads, local_git_broker, cold_cache_control, and reference_environment. Unknown keys are rejected. There are no arbitrary command, shell, network, or model-launch hooks. `--profile S|M|L` and `--repetitions N` override that configuration. Smoke forces 100 files, 51,200 total bytes, and one measured repetition, visibly marking the corpus nonnormative.

The corpus can also be generated without invoking PCTX:

```bash
python3 scripts/generate-corpus.py \
  --destination /private/tmp/.pctxbench-manual/project \
  --labels /private/tmp/.pctxbench-manual/corpus-labels.json \
  --profile S --seed 1048
```

Use a new destination. Custom `--files`/`--total-bytes` never retain a normative label. The script writes no operational project or user configuration.

## Measurements and Correctness

| Workload | Actual observation | Specification target or contract |
| --- | --- | --- |
| Initial index | One fresh index invocation, hashing/parsing included | M maximum 90 seconds |
| Incremental index | Twenty actual same-size comment changes before each invocation; complete enumeration included | M p95 3 seconds |
| Metadata search | Literal symbol search, freshness off, limit 20, warm preparation | M p95 250 ms |
| Matched search | Same query with current-file verification | M p95 700 ms |
| Body scan | Absent literal forces the complete corpus scan | M p95 2 seconds |
| Strict build | Actual build with task/seed, final JSON envelope and byte budget | M p95 15 seconds; AC15 body determinism |
| Labeled searches | Expected-path recall/rank, result hashes, empty results | Section 19 accuracy; unsupported distinct from empty |
| Symbol read | Returned exact symbol ID/hash/range/text compared with real file bytes | Prevent stale/wrong locations |
| Extract | Expected declaration content, hash, and complete emitted-byte count | Extract correctness/budget |
| Stale symbol | Real source change followed by use of original versioned ID | Must refuse stale code |
| Shared query | Five actual CLI processes, before/after broker refresh counter, source revisions/scopes | Local part of AC39; remote issue component unobserved |
| Session packets | Five explicit sessions, full/ack/unchanged delta, compact boundary then rejected baseline | Local receipt/epoch correctness; recovery tokens unobserved |
| Output and reread | Actual trusted child capture, full records, diagnostics, repeated reread after removing input | Stable artifacts without source rerun; emitted net savings |
| CLI memory | Per-invocation Unix wait4 maximum RSS | 512 MiB reference target; direct process only |
| Index/WAL | Private index plus corresponding WAL size after explicit temporary-project index GC, before later functional workloads | 400 MiB reference target; retention policy not independently certified |

Each timing group reports sample count, nearest-rank p50/p95, maximum, errors, and observed peak RSS. Initial indexing and warm groups remain separate. New processes/new databases **do not establish cold OS caches**: corpus generation itself warms pages. Default cold_cache_control is unknown; externally_controlled requires the operator to supply a genuinely controlled runner. The harness never evicts system caches or claims it did. Actual hardware memory, storage type, filesystem, and reference equivalence remain unknown if not observed.

On macOS, wait4 reports RSS bytes; on Linux it reports KiB, converted to bytes. On platforms without wait4, memory is unknown, not zero. Memory observations exclude the Python harness and independently surviving descendants. Overall child-tree peak memory requires another validated observer. The script drains stdout/stderr concurrently and bounds retained sample output; oversized or incomplete captures cannot become valid JSON samples.

Context-only selection/serialization (800 ms), incremental hashing-only decomposition, wrapper additional latency (50 ms), and isolated masked 1 MiB postprocessing (100 ms) require instrumentation beyond these complete CLI timings. They are reported unknown rather than estimated by subtraction from unrelated workloads.

Determinism compares complete `data` bodies, preserving source revisions, representation, omissions, and budget fields; only the envelope is excluded. Source locations are checked against original file hashes/ranges, not a success string. The harness does not prove every declared symbol was parsed correctly or measure semantic recall beyond its labeled synthetic queries. Public-project snapshots and broader fixtures remain separate necessary evaluation inputs.

## Output Savings and Failure Fixtures

`output-cases.json` covers tiny outputs, verbose progress, multiple failure/warning diagnostics, malformed report text, Unicode/CR progress, overlimit records, binary input, and a real nonzero `cat --invalid-option` exit with no input file (the fixture retains its historical `nonzero_missing_input` ID). Child exit/capture state comes from the actual runner. Words such as “completed” or “error” inside fixtures are presentation inputs, not proof tests ran.

Reports distinguish input fixture bytes, masked-uncompressed baseline bytes, initial compact emission bytes, subsequent retrieval emission bytes, total emissions, and net savings. Whole serialized stdout envelopes count as delivered bytes. Two repeated original reads and diagnostic retrieval costs are included. Negative savings remain negative. A rerender or output not observed by the harness is not added as model receipt. The harness records emitted bytes, not acknowledged or provider-received tokens.

Required diagnostic IDs are searched in retained records; repeated records are compared directly. Removing the source fixture before the repeated read provides a concrete reread check, but artifact identity alone is not independent process-supervisor proof. Native process-count security regressions remain in the runner/output integration suites.

The verbose-bundle aggregate compares masked baseline to **all** observed emissions. Its Section 45 normative status stays unknown when parsers are unsupported or expected cases are missing. A byte reduction alone cannot establish AC82: model diagnosis, code-fix success, rework, additional conversation, and provider costs remain unmeasured. False presentation passes and missing diagnostic IDs are reported separately, never hidden by compression percentages.

No tokenizer is fabricated. Tokenizer tokens, provider usage/cost, subscription quota, first-correct-action recovery tokens, and model wake counts are null/unknown without appropriate observers. Redaction savings are not counted as compression savings; already-compacted inputs cannot acquire invented original baselines.

## Evidence and Interpretation

A run produces:

- `mission.json`: configuration and future evaluation protocol, with checksum seal; integrity is not owner authorization.
- `corpus-labels.json`: exact generated file/hash/query labels and declared distributions.
- `samples.json`: bounded actual CLI observations, argument descriptors, JSON, exit/capture states, timings, byte counts, and direct-process RSS.
- `report.json`: machine-readable observed metrics, met/miss/unknown target states, corpus/repetition eligibility, and explicit limits.
- `report.md`: a short human table derived from those measurements.

“Met” means the observed value meets a numerical target on that host; it does not certify the reference environment or full product. “Miss” means an observed target or contract failed. “Unknown” means there is no sufficient observation. Smoke results, fewer than 30 repetitions, non-M corpora, failed CLI samples, and unconfirmed hardware cannot establish normative performance acceptance. Script exit zero means the measurement run finished, not that all product requirements passed; inspect report status/metrics. Harness infrastructure failure exits nonzero and retains its partial evidence.

No benchmark results are prefilled in this documentation. Generation and harness code require parent integration and actual runs before claiming measured performance. Python syntax and JSON fixture validation alone do not validate CLI behavior.

## Future Model Evaluation Protocol

[The live mission](../tests/evaluation/live-mission.json) describes at least 10 bug fixes, 10 small feature changes, and 10 review/impact tasks, at least three repetitions per arm, alternated order, reset state, fixed models/policies, warm/cold conditions, and output/extract/adaptive/all ablations. It includes all exploration, coordination, retrieval, retry, recovery, cache, and preparation costs. The task—not repeated runs—is the experimental unit. Analyze paired task-level bootstrap intervals and the predeclared -5 percentage-point noninferiority margin at 95% confidence; insufficient samples remain judgment pending.

A sealed plan can be exported without invoking the binary or any model:

```bash
python3 scripts/benchmark.py \
  --cli ./target/release/pctx \
  --plan-only \
  --output /private/tmp/pctx-evaluation-plan
```

The harness has no live model-launch interface. A future separately authorized provider observer must supply attributed request IDs, cached/uncached input and output usage, versioned price tables/currency, task success checks, and account/window quota provenance. A checksum seals the plan against accidental mutation; it is not a signature, permission grant, or live connection result. Until those trials exist, the 20% input-token, 40% recovery-token, cost, noninferiority, and unchanged-idle-wake targets remain unverified.
