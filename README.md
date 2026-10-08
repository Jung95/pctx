# PCTX — Portable Project

PCTX is a Rust CLI for local project exploration, bounded context, and durable work coordination. It is under active implementation against specification 0.6. This development snapshot does **not** satisfy the full release acceptance criteria.

The local source of truth is `docs/PCTX-implementation-spec-v0.6.md`, supplied by the project owner. The [English specification](docs/PCTX-implementation-spec-v0.6-en.md) preserves the complete design; [translation coverage](docs/translation-coverage.md) records the audit. Public implementation documentation is in English. Requirements and unverified work are tracked in [requirements](docs/implementation/requirements.md), [progress](docs/implementation/progress.md), and [handoff](docs/implementation/handoff.md).

## Build

Install a Rust toolchain, then run:

```sh
cargo build --locked
cargo test --locked
cargo run -- --help
```

SQLite is bundled; no database server or model service is required. Current native macOS arm64 full validation retains four startup failures. Native Linux aarch64 full validation at an earlier fixed revision retains one schedule-concurrency failure. First-release macOS x86_64, Linux x86_64 and Windows x86_64 qualification remains pending; historical CI and scoped Windows checks do not qualify the current source. See [platform evidence](docs/implementation/handoff.md) for exact revisions and limits.

## Explore a project

```sh
pctx --root /path/to/project init
pctx --root /path/to/project index update
pctx --root /path/to/project find auth --kind symbol
pctx --root /path/to/project outline src/auth.ts
pctx --root /path/to/project read src/auth.ts --lines 1:40
pctx --root /path/to/project --format json build --task "Fix authentication" --seed src/auth.ts --budget-bytes 12000
pctx --root /path/to/project checkpoint create --name before-fix
pctx --root /path/to/project changes --since before-fix
```

`PCTX_DATA_DIR` selects an isolated local data directory. Project configuration resides in `.pctx/config.toml`. Workspace indexes and coordination records reside outside the source directory by default. An explicit output file is created without overwriting an existing file.

`find QUERY` is literal. `find --query '(login OR auth) AND NOT legacy'` enables the bounded Boolean grammar. Outlines use Tree-sitter for Python, JavaScript, JSX, TypeScript and TSX. Unsupported UTF-8 files remain searchable but have no claimed language structure. Symbol reads validate source hashes and reject stale IDs.

Supported finite queries share one request deadline, including project loading and nested index refresh. Use `--timeout-ms 3000` to set a positive budget in milliseconds. See [query deadlines](docs/cli/query-deadlines.md) for covered commands, partial results and platform limits.

## Coordinate local work

See [work example](docs/cli/work.md) for a complete JSON task/check workflow. Tasks can be assigned and claimed, reported, submitted, reviewed, and completed without GitHub. A report claiming success is distinct from validated check evidence. Completion accepts a fixed submission; later source changes make its current validity outdated.

Run `pctx status --capabilities` before relying on a feature. Implementation status is separate from complete acceptance verification. No measured model-token or subscription savings are claimed yet.

See [source topic controls](docs/cli/source-topics.md), [source packs](docs/cli/pack.md), [installation](docs/cli/install.md), [Claude adapter](docs/cli/claude.md), and [runner](docs/cli/runner.md), [filter authoring](docs/cli/filters.md), and [evaluation](docs/evaluation.md) for current contracts and verification limits. Registered execution, inert filters, local quota observations, static inventory and actual local schedule ticks are implemented as development features. Reviewed managed bridge staging is available; native OS registration and live provider verification remain unverified. See [schedules](docs/cli/schedules.md), [inventory](docs/cli/inventory.md) and [parsers](docs/cli/parsers.md).

For reconnectable work events, use `pctx --format ndjson activity --follow --since-seq 0`. `pctx board --watch` prints refreshed local state every two seconds; NDJSON board observations distinguish time/heartbeat changes from permanent event sequences.
