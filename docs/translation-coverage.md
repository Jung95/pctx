# English Specification Translation Coverage

The public English edition is [PCTX-implementation-spec-v0.6-en.md](PCTX-implementation-spec-v0.6-en.md). It translates the owner-provided [Korean specification](PCTX-implementation-spec-v0.6.md), preserving that source unchanged. Release stages, mandatory obligations, optional extensions, exclusions, examples, proposed targets, and uncertainty statements retain their source meanings. This edition is a specification, not implementation or measured-performance evidence.

Source SHA-256: `8914faad57432f524aec0a06aa90c43c18c23f8c19e9f54fe69d9ad185db6523`.

## Structural Audit

The completed audit found:

- All 45 numbered sections, in identical order.
- All 141 third-level subsection headings, with matching counts in each section.
- All 50 tables and 613 table lines, with matching counts in each section.
- Every PCTX01–PCTX48 implementation row and AC01–AC82 acceptance row, in identical order.
- All 33 fenced examples, balanced and retaining the same language labels.
- Identical TOML, YAML, and Rust example bodies; valid JSON examples.
- Identical command-option sequences in every shell example.
- The same set of source links.

Human-readable strings in shell/JSON examples, diagram labels, layout placeholders, rule bodies, and terminal displays are translated. The three Korean glossary keys remain intentionally unchanged as input data demonstrating Korean-to-English aliasing; surrounding explanations are English. Markdown's rule example uses a longer outer fence without changing its contents' semantics.

The audit checks structure and preserved machine contracts. It cannot prove every translated sentence is semantically equivalent. Translation was performed section by section with direct source comparison; selected binding contracts received additional manual checks.

## Manual Contract Checks

| Source sections | Checked meaning |
| --- | --- |
| 1–7 | Portable knowledge versus environment/source replication; versioned release scope; explicit initialization; policy union, connector intersection, and executable trust |
| 8–10 | Independent evidence/freshness/coverage; actual-byte ranges; generation isolation; matched versus strict; non-atomic workspace observations and recovery |
| 11–17 | Lexical aliases versus semantics; ranking and bounded reads; serialized budgets and mandatory rules; acknowledgment; unsupported versus empty; metadata-only export and security lifetimes |
| 18–26 | Proposed performance targets versus measurements; task-level paired evaluation and noninferiority; all initial AC/work rows; compatibility and independent versions |
| 27 | Assignment/run/lease authority; transaction completion gates; latest valid check attempts; author claims versus runner observations; immutable submissions; restoration and remote/local-state separation |
| 28–33 | Shared-query failures/singleflight; epochs/tombstones; usage versus quota; capsule recovery; trusted owner provenance; pause/silence persistence; real child-process resource ownership |
| 34–38 | Head-SHA and deployment evidence; WIP reservations; no destructive action from unavailable sources; DST/misfires; managed Claude configuration; replaceable profile data; additional AC/work rows and full coordination costs |
| 39–45 | No imported advertising claims; unchanged argv/execution count/authority; masked original retrieval without rerun; bounded capture and filter protection; stale diagnostic handling; adaptive full/delta rules; explicit source packs, integrity versus freshness; negative/net savings and ablations |

## Remaining Limits

No separate bilingual reviewer has certified this edition. Structural parity and manual spot checks are evidence of coverage, not a full independent linguistic proof. Technical assertions and external links are preserved as owner-provided specification material; this translation did not independently verify dependency releases, installed provider capabilities, real account connections, or performance claims. Source ambiguities remain source ambiguities rather than being silently resolved through translation.

No build or runtime test was required or run for this documentation-only translation. The parent integration task handles publication links and requirement status updates.
