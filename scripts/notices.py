#!/usr/bin/env python3
"""Generate dependency notices and a CycloneDX inventory from the locked graph."""
import json
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], cwd=root))
packages = sorted((p for p in metadata["packages"] if p["source"]), key=lambda p: (p["name"], p["version"]))
notices = ["PCTX dependency notices\nGenerated from Cargo.lock and local dependency source archives.\n"]
components = []
for package in packages:
    base = pathlib.Path(package["manifest_path"]).parent
    license_files = sorted(p for p in base.iterdir() if p.is_file() and p.name.lower().startswith(("license", "licence", "copying", "copyright", "unlicense")))
    notices.append(f"\n{'=' * 72}\n{package['name']} {package['version']}\nLicense expression: {package['license'] or 'not declared'}\nRepository: {package['repository'] or 'not declared'}\n")
    if not license_files:
        notices.append("No top-level license file in this source archive; see package license declaration and repository.\n")
    for path in license_files:
        notices.append(f"\n--- {path.name} ---\n" + path.read_text(errors="replace") + "\n")
    component = {"type": "library", "name": package["name"], "version": package["version"], "purl": f"pkg:cargo/{package['name']}@{package['version']}", "licenses": [{"expression": package["license"]}] if package["license"] else []}
    checksum_file = base / ".cargo-checksum.json"
    checksum = json.loads(checksum_file.read_text()).get("package") if checksum_file.exists() else None
    if checksum:
        component["hashes"] = [{"alg": "SHA-256", "content": checksum}]
    components.append(component)
# Native bundled components may not have individual Cargo package identities.
notices.append("\nBundled SQLite is in the public domain. The source notice is retained by libsqlite3-sys. Tree-sitter grammar source is distributed under each grammar crate's declared license above.\n")
destination = root / "dist"
destination.mkdir(exist_ok=True)
(destination / "THIRD_PARTY_NOTICES.txt").write_text("".join(notices))
(destination / "sbom.json").write_text(json.dumps({"bomFormat": "CycloneDX", "specVersion": "1.5", "version": 1, "metadata": {"component": {"type": "application", "name": "pctx", "version": next(p["version"] for p in metadata["packages"] if p["name"] == "pctx" and not p["source"])}}, "components": components}, indent=2) + "\n")
print(f"Wrote notices and dependency inventory for {len(packages)} locked Cargo packages; inventory includes target-specific dependencies.")
