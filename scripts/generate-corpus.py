#!/usr/bin/env python3
"""Generate deterministic, secret-free PCTX evaluation inputs; never run tools."""
import argparse
import hashlib
import json
import random
from pathlib import Path

PROFILES = {"S": (1000, 20 * 1024**2), "M": (10000, 200 * 1024**2),
            "L": (100000, 2 * 1024**3)}
LANGUAGES = ("py", "js", "jsx", "ts", "tsx")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def source_header(index, extension):
    fn, cls, method = (f"benchFunction{index:06d}", f"BenchClass{index:06d}",
                       f"benchMethod{index:06d}")
    if extension == "py":
        text = f"def {fn}(value):\n    return value + {index}\n\nclass {cls}:\n    def {method}(self):\n        return {index}\n\n"
    else:
        text = f"export function {fn}(value) {{ return value + {index}; }}\nexport class {cls} {{ {method}() {{ return {index}; }} }}\n"
    return text.encode(), [fn, cls, method]


def pad(header, target, prefix, rng):
    """Exactly target bytes, valid comments/text, max 80 ASCII bytes per line."""
    out = bytearray(header)
    while len(out) < target:
        remaining = target - len(out)
        if remaining < len(prefix) + 1:
            out.extend(b" " * (remaining - 1) + b"\n")
            break
        width = min(80, remaining)
        body = bytes(rng.choice(b"abcdefghijklmnopqrstuvwxyz0123456789 ")
                     for _ in range(width - len(prefix) - 1))
        out.extend(prefix + body + b"\n")
    return bytes(out)


def generate(destination, profile="S", seed=1048, files=None, total_bytes=None):
    count, size = PROFILES[profile]
    count = files if files is not None else count
    size = total_bytes if total_bytes is not None else size
    if count < 100 or size // count < 512 or (size + count - 1) // count > 1024**2:
        raise ValueError("Require >=100 files and 512..1048576 mean bytes/file")
    destination = Path(destination).resolve()
    destination.mkdir(parents=True, exist_ok=False)
    language_count = count * 60 // 100
    document_count = count * 20 // 100
    rng = random.Random(seed)
    records, aliases = [], {}
    line_count, max_line_bytes = 0, 0
    quotient, remainder = divmod(size, count)
    for i in range(count):
        target = quotient + (i < remainder)
        if i < language_count:
            ext = LANGUAGES[i % 5]
            path = f"src/group{i // 100:04d}/module{i:06d}.{ext}"
            header, symbols = source_header(i, ext)
            prefix = b"# " if ext == "py" else b"// "
            category = "supported_language"
        elif i < language_count + document_count:
            ext, symbols = "md", []
            path = f"docs/topic{i:06d}.md"
            header = f"# Benchmark Topic {i:06d}\n\n## Evidence {i:06d}\n\n".encode()
            prefix, category = b"", "document"
        else:
            ext, symbols = "xyz", []
            path = f"legacy/note{i:06d}.xyz"
            header = f"UnsupportedTextMarker{i:06d}\n".encode()
            prefix, category = b"", "other_text"
        data = pad(header, target, prefix, rng)
        line_count += data.count(b"\n")
        max_line_bytes = max(max_line_bytes, *(len(line) + 1 for line in header.splitlines()), min(80, target - len(header)))
        filename = destination / path
        filename.parent.mkdir(parents=True, exist_ok=True)
        filename.write_bytes(data)
        records.append({"path": path, "bytes": len(data), "sha256": digest(data),
                        "category": category, "extension": ext,
                        "symbols": symbols, "header_bytes": len(header), "declaration_lines": 7 if ext == "py" else 2})
    sampled = random.Random(seed + 1).sample(range(language_count), 40)
    queries = []
    for q, index in enumerate(sampled):
        name = f"benchFunction{index:06d}"
        category = "exact_identifier" if q < 20 else "partial_name" if q < 30 else "korean_alias"
        term = name if q < 20 else name[5:] if q < 30 else f"평가별칭{q:02d}"
        if category == "korean_alias":
            aliases[term] = [name]
        queries.append({"id": f"Q{len(queries)+1:02d}", "category": category,
                        "query": term, "kind": "symbol", "expected_paths": [records[index]["path"]],
                        "expected_symbol": name})
    for q in range(10):
        queries.append({"id": f"Q{len(queries)+1:02d}", "category": "no_answer",
                        "query": f"NoSuchBenchmarkIdentifier{q:06d}", "kind": "symbol", "expected_paths": []})
    for index in range(language_count + document_count, language_count + document_count + 10):
        queries.append({"id": f"Q{len(queries)+1:02d}", "category": "unsupported_language",
                        "query": f"UnsupportedTextMarker{index:06d}", "kind": "text",
                        "expected_paths": [records[index]["path"]]})
    observed = {category: sum(r["category"] == category for r in records)
                for category in ("supported_language", "document", "other_text")}
    label = {"schema_version": 1, "seed": seed, "profile": profile,
             "normative_shape": files is None and total_bytes is None,
             "file_count": count, "total_bytes": sum(r["bytes"] for r in records),
             "categories": observed, "language_variants": LANGUAGES,
             "declared_symbols": language_count * 3,
             "symbol_distribution": {"function": language_count, "class": language_count,
                                     "method": language_count},
             "document_sections": document_count * 2,
             "max_line_bytes": max_line_bytes, "mean_line_bytes": size / line_count, "line_count": line_count, "max_file_bytes": max(r["bytes"] for r in records),
             "mean_file_bytes": size / count, "files": records, "queries": queries,
             "aliases": aliases, "generation_manifest_hash": digest(json.dumps(records, sort_keys=True).encode())}
    if label["total_bytes"] != size:
        raise AssertionError("Corpus byte accounting mismatch")
    return label


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--labels", type=Path, required=True)
    parser.add_argument("--profile", choices=PROFILES, default="S")
    parser.add_argument("--seed", type=int, default=1048)
    parser.add_argument("--files", type=int)
    parser.add_argument("--total-bytes", type=int)
    args = parser.parse_args()
    labels = generate(args.destination, args.profile, args.seed, args.files, args.total_bytes)
    args.labels.parent.mkdir(parents=True, exist_ok=True)
    with args.labels.open("x", encoding="utf-8") as output:
        json.dump(labels, output, ensure_ascii=False, sort_keys=True, indent=2)
        output.write("\n")
    print(json.dumps({k: labels[k] for k in ("profile", "file_count", "total_bytes", "normative_shape")}))


if __name__ == "__main__":
    main()
