# Installation, update and removal

This is a development build. Full release qualification is pending. Local execution evidence is macOS arm64 only; other targets are not verified by compilation alone.

## Build and install from source

A Rust toolchain and platform C/C++ compiler are required to build the bundled SQLite and grammar dependencies. The resulting CLI does not require Rust, Python, Node, a database server, or a model runtime to execute.

```sh
cargo install --path . --locked --root "$HOME/.local"
"$HOME/.local/bin/pctx" --version
```

Add `$HOME/.local/bin` to your PATH using your own shell configuration. Installation does not change project files, hooks, provider settings or schedules.

Update by building a reviewed newer checkout and repeating the installation command. Keep the previous binary and a control backup until the new version's database compatibility has been checked. A newer database schema must never be downgraded automatically.

```sh
pctx control backup --output /path/to/new-backup.json
pctx doctor --format json
```

Remove the binary using `cargo uninstall --root "$HOME/.local" pctx` or remove the explicitly installed archive binary. User-created rules, tasks, handoffs, and backups remain. Review `pctx status --format json` for the local index/control locations before deleting any data manually. Index data is derived; control data contains durable work originals.

## Isolated development

Set `PCTX_DATA_DIR` to a fresh temporary directory and pass `--root` for a temporary project. No global hook or OS schedule is installed by the CLI examples.

The project's ignored `.toolchain` directory was prepared for development on the initial host. It is not a runtime dependency or a distributable artifact.
