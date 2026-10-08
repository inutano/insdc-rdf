#!/usr/bin/env python3
"""Append release entries to the bucket catalog index.json.

Usage:
    catalog_add_release.py INDEX_JSON ENTRY_JSON... [--updated YYYY-MM-DD]

Each ENTRY_JSON (for example the OUT_DIR/<id>.index-entry.json written by
package_rdf_release.py) is appended to "releases" in argument order. Refuses,
leaving INDEX_JSON untouched, if a release_id is already present. Sets
"updated" (default: today, UTC). Nothing is uploaded.
"""
import argparse
import datetime
import json
import sys


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("index_json")
    ap.add_argument("entries", nargs="+", metavar="ENTRY_JSON")
    ap.add_argument("--updated", help="YYYY-MM-DD (default: today, UTC)")
    args = ap.parse_args(argv)

    with open(args.index_json, encoding="utf-8") as f:
        index = json.load(f)
    releases = index.setdefault("releases", [])
    seen = set(r.get("release_id") for r in releases)

    new = []
    for path in args.entries:
        with open(path, encoding="utf-8") as f:
            entry = json.load(f)
        rid = entry.get("release_id")
        if not rid:
            print("error: %s has no release_id" % path, file=sys.stderr)
            return 1
        if rid in seen:
            print("error: release_id already in catalog: %s" % rid, file=sys.stderr)
            return 1
        seen.add(rid)
        new.append(entry)

    releases.extend(new)
    index["updated"] = args.updated or datetime.datetime.utcnow().strftime("%Y-%m-%d")
    with open(args.index_json, "w", encoding="utf-8") as f:
        json.dump(index, f, indent=2, ensure_ascii=False)
        f.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
