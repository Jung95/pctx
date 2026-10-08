#!/usr/bin/env python3
"""Run pinned-source native Linux checks offline in a disposable container.

Caller supplies a credential-free registry cache and a native Rust toolchain.
This proves only the actual Linux architecture tested, not other platforms.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def tree_hash(source):
    digest = hashlib.sha256()
    for path in sorted(source.rglob("*")):
        if path.is_symlink():
            content = os.readlink(path).encode()
            kind = b"link"
        elif path.is_file():
            content = path.read_bytes()
            kind = b"executable" if path.stat().st_mode & 0o111 else b"file"
        else:
            continue
        digest.update(path.relative_to(source).as_posix().encode() + b"\0")
        digest.update(kind + b"\0" + hashlib.sha256(content).digest())
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True, help="Immutable sha256 image ID")
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--toolchain", required=True, type=Path)
    parser.add_argument("--cargo-home", required=True, type=Path)
    parser.add_argument("--target", required=True, type=Path)
    parser.add_argument("--record", required=True, type=Path)
    parser.add_argument("--phase", choices=["clippy", "test", "proc-test"], required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"sha256:[a-f0-9]{64}", args.image):
        parser.error("--image must be an immutable sha256 ID")
    if not re.fullmatch(r"[a-f0-9]{40}", args.revision):
        parser.error("--revision must be the full archived Git commit")
    paths = [args.source, args.toolchain, args.cargo_home, args.target]
    paths = [path.resolve(strict=True) for path in paths]
    if any(not path.is_dir() or "," in str(path) for path in paths):
        parser.error("Mount paths must be directories without commas")
    source, toolchain, cargo_home, target = paths
    if any((cargo_home / name).exists() for name in ["credentials", "credentials.toml", "config", "config.toml"]):
        parser.error("Use a dedicated Cargo cache without credentials or global config")
    phase = {
        "clippy": "cargo clippy --offline --locked --all-targets -- -D warnings",
        "test": "cargo test --offline --locked --all-targets --no-fail-fast",
        "proc-test": "cargo test --offline --locked --lib query_process::linux_group_tests -- --nocapture",
    }[args.phase]
    name = f"pctx-linux-{os.getpid()}"
    command = [
        "docker", "run", "--rm", "--init", "--name", name, "--network", "none", "--read-only",
        "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
        "--user", f"{os.getuid()}:{os.getgid()}",
        "--tmpfs", "/tmp:rw,nosuid,nodev,exec,size=1g", "--entrypoint", "/bin/sh",
        "--workdir", "/workspace",
    ]
    for path, destination, readonly in [
        (source, "/workspace", True), (toolchain, "/opt/pctx-rust", True),
        (cargo_home, "/cargo", False), (target, "/target", False),
    ]:
        mount = f"type=bind,src={path},dst={destination}"
        command += ["--mount", mount + (",readonly" if readonly else "")]
    for value in [
        "CARGO_HOME=/cargo", "CARGO_TARGET_DIR=/target", "CARGO_BUILD_JOBS=2",
        "HOME=/tmp/pctx-home",
        "PATH=/opt/pctx-rust/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    ]:
        command += ["--env", value]
    command += [args.image, "-c", "mkdir -p /tmp/pctx-home && exec " + phase]
    record = {
        "source_revision": args.revision, "source_tree_sha256": tree_hash(source),
        "image": args.image, "phase": args.phase, "container": name,
        "command": command, "status": "running", "build_jobs": 2,
        "test_threads": "default; no test concurrency override",
    }
    args.record.write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps({"container": name, "phase": args.phase, "source_tree_sha256": record["source_tree_sha256"]}), flush=True)
    try:
        result = subprocess.run(command)
    except BaseException:
        subprocess.run(["docker", "stop", "--time", "2", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        record["status"] = "interrupted"
        args.record.write_text(json.dumps(record, indent=2) + "\n")
        raise
    record.update(status="terminal", exit=result.returncode)
    record["source_unchanged"] = tree_hash(source) == record["source_tree_sha256"]
    args.record.write_text(json.dumps(record, indent=2) + "\n")
    return result.returncode if record["source_unchanged"] else 1


if __name__ == "__main__":
    sys.exit(main())
