#!/bin/sh
set -eu
# Produces a local development archive; publishing is a separate action.
version=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')
system=$(uname -s | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)
name="pctx-$version-$system-$arch"
cargo build --release --locked
python3 scripts/notices.py
mkdir -p dist
stage=$(mktemp -d "${TMPDIR:-/tmp}/pctx-package.XXXXXX")
trap 'rm -rf "$stage"' EXIT HUP INT TERM
mkdir "$stage/$name"
cp target/release/pctx LICENSE README.md "$stage/$name/"
cp dist/THIRD_PARTY_NOTICES.txt "$stage/$name/"
cp dist/sbom.json "$stage/$name/"
# Include the public guides referenced by README; preserve the private input locally.
mkdir "$stage/$name/docs"
cp docs/PCTX-implementation-spec-v0.6-en.md docs/translation-coverage.md docs/evaluation.md "$stage/$name/docs/"
cp -R docs/cli docs/implementation "$stage/$name/docs/"
"$stage/$name/pctx" --version
# macOS tar must not add AppleDouble resource-fork sidecars to portable archives.
COPYFILE_DISABLE=1 tar -czf "dist/$name.tar.gz" -C "$stage" "$name"
python3 - "$name" <<'PY'
import hashlib,pathlib,sys
p=pathlib.Path('dist')/(sys.argv[1]+'.tar.gz')
p.with_suffix(p.suffix+'.sha256').write_text(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+p.name+'\n')
PY
