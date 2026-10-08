#!/usr/bin/env python3
"""Package insdc-rdf converter output into an immutable release.

Usage:
    package_rdf_release.py SPEC OUT_DIR --data-root DIR [--jobs N] [--level L]

Writes OUT_DIR/<id>/ (per-chunk gzip, RO-Crate 1.1 metadata, README, checksums),
OUT_DIR/<id>.index-entry.json and, if the spec asks for it, OUT_DIR/<id>.tar.gz.
Nothing is uploaded.
"""
import argparse
import concurrent.futures
import gzip
import hashlib
import json
import os
import shutil
import sys
import tarfile
from pathlib import Path

FORMATS = ("nt", "ttl", "jsonld")
BLOCK = 8 * 1024 * 1024
RO_CRATE_PROFILE = "https://w3id.org/ro/crate/1.1"

GZ_ENCODING = {
    "nt": ["application/n-triples", "application/gzip"],
    "ttl": ["text/turtle", "application/gzip"],
    "jsonld": ["application/ld+json", "application/gzip"],
}
EXT_ENCODING = {
    ".md": "text/markdown",
    ".json": "application/json",
    ".tsv": "text/tab-separated-values",
    ".sha256": "text/plain",
    ".shex": "text/shex",
    ".yaml": "application/yaml",
    ".svg": "image/svg+xml",
}


class PackageError(Exception):
    """A refusal or a mismatch that stops packaging."""


# ---------------------------------------------------------------- helpers


def sha256_file(path):
    h = hashlib.sha256()
    with open(str(path), "rb") as f:
        for block in iter(lambda: f.read(BLOCK), b""):
            h.update(block)
    return h.hexdigest()


def human_size(n):
    if n < 1024:
        return "%d B" % n
    size = float(n)
    for unit in ("KB", "MB", "GB", "TB"):
        size /= 1024
        # Pick the unit after rounding, so 1023.97 KB prints as 1.0 MB.
        if round(size, 1) < 1024 or unit == "TB":
            return "%.1f %s" % (size, unit)


def gzip_chunk(args):
    """Worker: gzip one chunk deterministically.

    Returns (source, fmt, file_name, gz_size, gz_sha256, nt_line_count).
    """
    src_path, dst_path, source, fmt, level = args
    lines = 0
    sha = hashlib.sha256()
    size = 0
    with open(src_path, "rb") as fin, open(dst_path, "wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw,
                           compresslevel=level, mtime=0) as gz:
            for block in iter(lambda: fin.read(BLOCK), b""):
                if fmt == "nt":
                    # A final line without "\n" would be undercounted;
                    # expected_triples catches that.
                    lines += block.count(b"\n")
                gz.write(block)
    with open(dst_path, "rb") as f:
        for block in iter(lambda: f.read(BLOCK), b""):
            sha.update(block)
            size += len(block)
    return source, fmt, os.path.basename(src_path), size, sha.hexdigest(), lines


def chunk_stem(name):
    return name.rsplit(".", 1)[0] if "." in name else name


def regular_files(directory):
    return sorted(p for p in Path(directory).rglob("*") if p.is_file())


def relpath(path, root):
    return path.relative_to(root).as_posix()


# ------------------------------------------------------------- validation


def validate(spec, out_dir, data_root, repo_root):
    rid = spec["release_id"]
    out = Path(out_dir)
    targets = [out / rid, out / (rid + ".partial")]
    if spec.get("tarball"):
        targets.append(out / (rid + ".tar.gz"))
        targets.append(out / (rid + ".tar.gz.partial"))
    for t in targets:
        if t.exists():
            raise PackageError("output already exists, refusing to overwrite: %s" % t)

    for s in spec["sources"]:
        name = s["name"]
        base = Path(data_root) / s["dir"]
        for f in ("manifest.json", "progress.json"):
            if not (base / f).is_file():
                raise PackageError("%s: missing %s" % (name, base / f))
        manifest = json.loads((base / "manifest.json").read_text(encoding="utf-8"))
        total = manifest["total_chunks"]
        stems = {}
        for fmt in FORMATS:
            d = base / fmt
            if not d.is_dir():
                raise PackageError("%s: %s is not a directory" % (name, d))
            files = [p for p in d.iterdir() if p.is_file()]
            if len(files) != total:
                raise PackageError(
                    "%s: %s holds %d files, manifest says %d chunks"
                    % (name, fmt, len(files), total))
            stems[fmt] = sorted(chunk_stem(p.name) for p in files)
        for fmt in FORMATS[1:]:
            if stems[fmt] != stems["nt"]:
                raise PackageError("%s: chunk names of %s differ from nt" % (name, fmt))
        if not (Path(repo_root) / s["schema_dir"]).is_dir():
            raise PackageError("%s: schema_dir %s not found under %s"
                               % (name, s["schema_dir"], repo_root))


# ------------------------------------------------------------- generation


def build_readme(spec, stats, tree_lines):
    rid = spec["release_id"]
    lic = spec["license"]
    base_url = spec["base_url"]
    bucket = spec["bucket"]
    out = ["# " + spec["name"], "",
           "Release `%s` · published %s · [%s](%s)"
           % (rid, spec["date_published"], lic["name"], lic["id"]), ""]
    for p in spec["readme"]["intro"]:
        out += [p, ""]

    out += ["## Contents", "",
            "| Source | Records | Triples | Chunks | N-Triples (gz) | Turtle (gz) | JSON-LD (gz) |",
            "|---|---:|---:|---:|---:|---:|---:|"]
    for s in spec["sources"]:
        st = stats[s["name"]]
        out.append("| %s | %s | %s | %s | %s | %s | %s |" % (
            s["title"], format(st["records"], ","), format(st["triples"], ","),
            format(st["chunks"], ","), human_size(st["bytes"]["nt"]),
            human_size(st["bytes"]["ttl"]), human_size(st["bytes"]["jsonld"])))
    total_triples = sum(st["triples"] for st in stats.values())
    total_records = sum(st["records"] for st in stats.values())
    total_chunks = sum(st["chunks"] for st in stats.values())
    out += ["", "Total: %s records, %s triples, %s chunks per format."
            % (format(total_records, ","), format(total_triples, ","),
               format(total_chunks, ",")),
            "", "Release layout:", "", "```"] + tree_lines + ["```", ""]

    first = spec["sources"][0]["name"]
    out += ["## Download", "",
            "The bucket is public. No AWS account is needed.", ""]
    if spec.get("tarball"):
        out += ["The whole release as one archive:", "", "```sh",
                "curl -O %s/releases/%s.tar.gz" % (base_url, rid),
                "tar xzf %s.tar.gz" % rid, "```", ""]
    out += ["N-Triples only:", "", "```sh",
            'aws s3 sync --no-sign-request s3://%s/releases/%s/ ./%s/ '
            '--exclude "*/ttl/*" --exclude "*/jsonld/*"' % (bucket, rid, rid),
            "```", "",
            "To fetch Turtle or JSON-LD instead, swap the excludes: exclude "
            "`*/nt/*` and `*/jsonld/*` for Turtle, or `*/nt/*` and `*/ttl/*` "
            "for JSON-LD.", "",
            "A single file:", "", "```sh",
            "curl -O %s/releases/%s/%s/nt/chunk_0000.nt.gz" % (base_url, rid, first),
            "```", ""]

    out += ["## Verify", "", "```sh", "cd %s" % rid,
            "sha256sum -c --ignore-missing provenance/checksums.sha256", "```", "",
            "`--ignore-missing` lets a partial download, such as one format only, "
            "be checked.", ""]

    out += ["## Load into a triplestore", "",
            "Every chunk is a complete RDF document, so chunks can be loaded one by "
            "one or streamed together. For example, with a loader that reads "
            "N-Triples on stdin:", "", "```sh",
            "zcat */nt/*.nt.gz | <loader reading N-Triples on stdin>", "```", "",
            "See `scripts/qlever_rebuild_index.sh` in the "
            "[insdc-rdf repository](%s) for the QLever recipe used to validate "
            "this release." % spec["software"]["repository"], ""]

    out += ["## Schema", "",
            "`schema/<source>/` holds the rdf-config model, the ShEx shape "
            "(`shape.shex`), the diagram (`schema.svg`) and example SPARQL "
            "(`sparql.yaml`).", ""]

    out += ["## Notes", ""] + ["- " + n for n in spec["readme"]["notes"]] + [""]

    out += ["## Provenance", "",
            "| Input | Last modified | Size (bytes) | MD5 |", "|---|---|---:|---|"]
    for i in spec["inputs"]:
        size = format(i["content_size"], ",") if "content_size" in i else "—"
        out.append("| [%s](%s) | %s | %s | %s |" % (
            i["name"], i["id"], i.get("date_modified", "—"), size,
            i.get("md5", "—")))
    sw = spec["software"]
    out += ["", "Converted with insdc-rdf `%s` ([`%s`](%s/commit/%s))"
            % (sw["version"], sw["commit"][:7], sw["repository"], sw["commit"]), ""]
    for s in spec["sources"]:
        st = stats[s["name"]]
        out.append("- %s: conversion started %s, completed %s"
                   % (s["title"], st["started_at"], st["completed_at"]))
    out.append("")

    out += ["## License and citation", "",
            "Licensed under [%s](%s)." % (lic["name"], lic["id"]), "",
            "Suggested citation: %s, release `%s`. %s/releases/%s/"
            % (spec["name"], rid, base_url, rid), ""]
    out += ["## Contact", "", "https://github.com/inutano/insdc-rdf/issues", ""]
    return "\n".join(out)


def layout_tree(spec):
    lines = ["%s/" % spec["release_id"]]
    for s in spec["sources"]:
        lines.append("  %s/{nt,ttl,jsonld}/chunk_NNNN.<fmt>.gz" % s["name"])
    lines += ["  schema/<source>/  (rdf-config model, ShEx, diagram, SPARQL)",
              "  provenance/  (<source>.manifest.json, triples.tsv, checksums.sha256)",
              "  ro-crate-metadata.json",
              "  README.md"]
    return lines


def encoding_for(rel):
    parts = rel.split("/")
    if rel.endswith(".gz") and len(parts) == 3 and parts[1] in GZ_ENCODING:
        return GZ_ENCODING[parts[1]]
    ext = os.path.splitext(rel)[1]
    return EXT_ENCODING.get(ext, "application/octet-stream")


def build_crate(spec, rel_root, hashes, stats):
    files = [p for p in regular_files(rel_root) if p.name != "ro-crate-metadata.json"]
    rels = [relpath(p, rel_root) for p in files]
    sw = spec["software"]
    sw_id = "#%s-%s" % (sw["name"], sw["commit"][:7])
    sources = spec["sources"]

    def ids(items):
        return [{"@id": i} for i in items]

    graph = [
        {"@id": "ro-crate-metadata.json", "@type": "CreativeWork",
         "conformsTo": {"@id": RO_CRATE_PROFILE}, "about": {"@id": "./"}},
        {"@id": "./", "@type": "Dataset", "name": spec["name"],
         "description": spec["description"], "datePublished": spec["date_published"],
         "license": {"@id": spec["license"]["id"]},
         "author": ids(a["id"] for a in spec["authors"]),
         "hasPart": ids([s["name"] + "/" for s in sources]
                        + ["schema/", "provenance/", "README.md"]),
         "isBasedOn": ids(i["id"] for i in spec["inputs"]),
         "mentions": ids("#convert-" + s["name"] for s in sources)},
    ]

    def children(prefix):
        return [r for r in rels if r.startswith(prefix)]

    for s in sources:
        n = s["name"]
        graph.append({"@id": n + "/", "@type": "Dataset", "name": s["title"],
                      "hasPart": ids("%s/%s/" % (n, f) for f in FORMATS)})
        for f in FORMATS:
            prefix = "%s/%s/" % (n, f)
            graph.append({"@id": prefix, "@type": "Dataset",
                          "name": "%s, %s chunks" % (s["title"], f),
                          "hasPart": ids(children(prefix))})
    graph.append({"@id": "schema/", "@type": "Dataset", "name": "Schema files",
                  "hasPart": ids("schema/%s/" % s["name"] for s in sources)})
    for s in sources:
        prefix = "schema/%s/" % s["name"]
        graph.append({"@id": prefix, "@type": "Dataset",
                      "name": "Schema files, %s" % s["title"],
                      "hasPart": ids(children(prefix))})
    graph.append({"@id": "provenance/", "@type": "Dataset", "name": "Provenance",
                  "hasPart": ids(children("provenance/"))})

    for p, r in zip(files, rels):
        graph.append({"@id": r, "@type": "File", "name": p.name,
                      "contentSize": str(p.stat().st_size),
                      "encodingFormat": encoding_for(r),
                      "sha256": hashes.get(r) or sha256_file(p)})

    for i in spec["inputs"]:
        e = {"@id": i["id"], "@type": i["type"], "name": i["name"]}
        if "content_size" in i:
            e["contentSize"] = str(i["content_size"])
        if "date_modified" in i:
            e["dateModified"] = i["date_modified"]
        graph.append(e)

    graph.append({"@id": sw_id, "@type": "SoftwareApplication", "name": sw["name"],
                  "description": sw["description"], "version": sw["version"],
                  "softwareVersion": sw["commit"],
                  "url": "%s/commit/%s" % (sw["repository"], sw["commit"])})
    for s in sources:
        st = stats[s["name"]]
        graph.append({"@id": "#convert-" + s["name"], "@type": "CreateAction",
                      "name": "insdc-rdf convert --source " + s["name"],
                      "instrument": {"@id": sw_id}, "object": ids(s["inputs"]),
                      "result": {"@id": s["name"] + "/"},
                      "startTime": st["started_at"], "endTime": st["completed_at"],
                      "actionStatus": "http://schema.org/CompletedActionStatus"})
    for a in spec["authors"]:
        graph.append({"@id": a["id"], "@type": "Person", "name": a["name"],
                      "affiliation": ids(a["affiliation"])})
    for o in spec["organizations"]:
        graph.append({"@id": o["id"], "@type": "Organization", "name": o["name"]})
    graph.append({"@id": spec["license"]["id"], "@type": "CreativeWork",
                  "name": spec["license"]["name"]})
    return {"@context": "https://w3id.org/ro/crate/1.1/context", "@graph": graph}


# ------------------------------------------------------------------ main


def package(spec, out_dir, data_root, repo_root, jobs=4, level=6):
    validate(spec, out_dir, data_root, repo_root)
    rid = spec["release_id"]
    out = Path(out_dir)
    final = out / rid
    part = out / (rid + ".partial")
    sources = spec["sources"]

    tasks = []
    stats = {}
    for s in sources:
        n = s["name"]
        base = Path(data_root) / s["dir"]
        manifest = json.loads((base / "manifest.json").read_text(encoding="utf-8"))
        progress = json.loads((base / "progress.json").read_text(encoding="utf-8"))
        stats[n] = {"records": manifest["total_records"], "chunks": manifest["total_chunks"],
                    "completed_at": manifest["completed_at"],
                    "started_at": progress["started_at"],
                    "triples": 0, "bytes": {f: 0 for f in FORMATS}}
        for fmt in FORMATS:
            (part / n / fmt).mkdir(parents=True)
            for f in sorted((base / fmt).iterdir()):
                dst = part / n / fmt / (f.name + ".gz")
                tasks.append((str(f), str(dst), n, fmt, level))

    hashes = {}
    ex = concurrent.futures.ProcessPoolExecutor(max_workers=jobs)
    futures = [ex.submit(gzip_chunk, t) for t in tasks]
    try:
        for fut, task in zip(futures, tasks):
            try:
                n, fmt, name, size, digest, lines = fut.result()
            except OSError as e:
                raise PackageError("%s: cannot compress %s: %s" % (task[2], task[0], e))
            hashes["%s/%s/%s.gz" % (n, fmt, name)] = digest
            stats[n]["bytes"][fmt] += size
            stats[n]["triples"] += lines
    except BaseException:
        # No cancel_futures on Python 3.8: drop every queued task by hand.
        for f in futures:
            f.cancel()
        raise
    finally:
        ex.shutdown(wait=True)

    for s in sources:
        n = s["name"]
        exp = s.get("expected_triples")
        if exp is not None and exp != stats[n]["triples"]:
            raise PackageError(
                "%s: counted %d triples, expected %d (partial output left in %s; "
                "it can be removed)" % (n, stats[n]["triples"], exp, part))

    (part / "provenance").mkdir()
    for s in sources:
        base = Path(data_root) / s["dir"]
        shutil.copyfile(str(base / "manifest.json"),
                        str(part / "provenance" / (s["name"] + ".manifest.json")))
        sdir = part / "schema" / s["name"]
        sdir.mkdir(parents=True)
        for f in sorted((Path(repo_root) / s["schema_dir"]).iterdir()):
            if f.is_file():
                shutil.copyfile(str(f), str(sdir / f.name))

    tsv = "source\ttriples\n" + "".join(
        "%s\t%d\n" % (s["name"], stats[s["name"]]["triples"]) for s in sources)
    (part / "provenance" / "triples.tsv").write_bytes(tsv.encode("utf-8"))

    (part / "README.md").write_bytes(
        build_readme(spec, stats, layout_tree(spec)).encode("utf-8"))

    skip = {"ro-crate-metadata.json", "provenance/checksums.sha256"}
    entries = []
    for p in regular_files(part):
        r = relpath(p, part)
        if r not in skip:
            entries.append((r, hashes.get(r) or sha256_file(p)))
    entries.sort()
    (part / "provenance" / "checksums.sha256").write_bytes(
        "".join("%s  %s\n" % (d, r) for r, d in entries).encode("utf-8"))

    crate = build_crate(spec, part, hashes, stats)
    (part / "ro-crate-metadata.json").write_bytes(
        (json.dumps(crate, indent=2, ensure_ascii=False) + "\n").encode("utf-8"))

    part.rename(final)

    entry = {
        "release_id": rid,
        "kind": "rdf",
        "prefix": "releases/%s/" % rid,
        "tarball": "releases/%s.tar.gz" % rid if spec.get("tarball") else None,
        "name": spec["name"],
        "description": spec["description"],
        "date_published": spec["date_published"],
        "license": spec["license"]["id"],
        "authors": [a["name"] for a in spec["authors"]],
        "ro_crate_profile": RO_CRATE_PROFILE,
        "formats": list(FORMATS),
        "triple_count": sum(st["triples"] for st in stats.values()),
        "derived_from": spec["derived_from"],
    }
    final_files = regular_files(final)
    entry["file_count"] = len(final_files)
    entry["total_bytes"] = sum(p.stat().st_size for p in final_files)

    if spec.get("tarball"):
        tb = out / (rid + ".tar.gz")
        tb_part = out / (rid + ".tar.gz.partial")
        with tarfile.open(str(tb_part), "w:gz") as t:
            t.add(str(final), arcname=rid)
        tb_part.rename(tb)
        entry["tarball_bytes"] = tb.stat().st_size

    # Written last: its presence means the whole package completed.
    (out / (rid + ".index-entry.json")).write_bytes(
        (json.dumps(entry, indent=2, ensure_ascii=False) + "\n").encode("utf-8"))
    return entry


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("spec")
    ap.add_argument("out_dir")
    ap.add_argument("--data-root", required=True)
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--level", type=int, default=6)
    args = ap.parse_args(argv)
    repo_root = Path(__file__).resolve().parent.parent
    try:
        spec = json.loads(Path(args.spec).read_text(encoding="utf-8"))
        Path(args.out_dir).mkdir(parents=True, exist_ok=True)
        entry = package(spec, args.out_dir, args.data_root, str(repo_root),
                        jobs=args.jobs, level=args.level)
    except (PackageError, OSError) as e:
        print("error: %s" % e, file=sys.stderr)
        return 1
    tsv = (Path(args.out_dir) / entry["release_id"] / "provenance" / "triples.tsv")
    triples = [l.split("\t") for l in tsv.read_text(encoding="utf-8").splitlines()[1:]]
    print("release %s: %d files, %d bytes" % (entry["release_id"], entry["file_count"],
                                              entry["total_bytes"]))
    for n, t in triples:
        print("  %s: %s triples" % (n, t))
    print("  total: %d triples" % entry["triple_count"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
