#!/usr/bin/env python3
"""Isolated archive installation/removal proof; no global installation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate(archive, expected_binary):
    digest = sha(archive)
    assert archive.with_suffix(archive.suffix + ".sha256").read_text().split()[0] == digest
    with tempfile.TemporaryDirectory(prefix="pctx-archive-install-") as td:
        root = Path(td)
        with tarfile.open(archive, "r:gz") as tf:
            entries = tf.getmembers()
            tops = set()
            for entry in entries:
                path = Path(entry.name)
                assert not path.is_absolute() and ".." not in path.parts
                assert entry.isfile() or entry.isdir()
                assert not any(part.startswith("._") for part in path.parts)
                assert not any(part in {".git", ".toolchain", ".pctx", ".codex"} for part in path.parts)
                assert path.name != "PCTX-implementation-spec-v0.6.md"
                tops.add(path.parts[0])
            assert len(tops) == 1
            tf.extractall(root / "unpacked", filter="data")
        bundle = root / "unpacked" / tops.pop()
        executable = bundle / ("pctx.exe" if os.name == "nt" else "pctx")
        assert sha(executable) == sha(expected_binary)
        for name in ["LICENSE", "THIRD_PARTY_NOTICES.txt", "sbom.json",
                     "docs/cli/install.md", "docs/cli/formats.md",
                     "docs/PCTX-implementation-spec-v0.6-en.md"]:
            assert (bundle / name).is_file(), name
        for _, target in re.findall(r"\[([^]]+)\]\(([^)]+)\)", (bundle / "README.md").read_text()):
            if target.startswith("docs/"):
                assert (bundle / target).is_file(), target
        prefix = root / "prefix" / "bin"
        prefix.mkdir(parents=True)
        installed = prefix / executable.name
        shutil.copy2(executable, installed)
        project = root / "project"
        project.mkdir()
        (project / "auth.py").write_text("def auth():\n    return 1\n")
        home = root / "home"
        home.mkdir()
        env = dict(os.environ, PCTX_DATA_DIR=str(root / "data"),
                   PCTX_USER_CONFIG=str(root / "absent-config"), HOME=str(home),
                   USERPROFILE=str(home), PCTX_ACTOR="owner")
        outcomes = []
        for args in [["init"], ["index", "update"], ["find", "auth", "--kind", "symbol"],
                     ["inventory", "scan"], ["doctor"]]:
            result = subprocess.run([str(installed), "--root", str(project), "--format", "json"] + args,
                                    env=env, capture_output=True, timeout=30)
            assert result.returncode == 0, (args, result.stdout.decode(), result.stderr.decode())
            value = json.loads(result.stdout)
            assert value["status"] == "ok"
            if args[0] == "find":
                assert value["data"]["items"][0]["path"] == "auth.py" and value["generation_id"]
            outcomes.append({"arguments": args, "exit": result.returncode})
        before = {str(p.relative_to(project)): sha(p) for p in project.rglob("*") if p.is_file()}
        installed.unlink()
        assert not installed.exists()
        after = {str(p.relative_to(project)): sha(p) for p in project.rglob("*") if p.is_file()}
        assert before == after
        assert not (home / ".config").exists()
        return {"archive_sha256": digest, "entries": len(entries), "binary_sha256": sha(executable),
                "isolated_archive_install": True, "commands": outcomes,
                "explicit_binary_removal_preserves_project": True, "private_spec_excluded": True,
                "global_settings_unchanged": True, "forward_version_update_and_migration": "not_verified"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("archive", type=Path)
    parser.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(validate(args.archive, args.binary), indent=2))
