#!/usr/bin/env python3
"""Convert the ontology files shipped in a bsllmner-mk2 RO-Crate to N-Triples.

The files are converted unchanged, one .nt per .owl, into <out-dir>/nt/, so
that `insdc-rdf validate <out-dir>` and the QLever index build treat them like
any other source. They are third-party files with their own licenses; see
SOURCES.md in the bsllmner release bucket. Requires rdflib.

Usage: bsllmner_ontology_to_nt.py <crate>/ontology <out-dir>
"""
import json
import sys
from pathlib import Path

from rdflib import Graph


def main(argv):
    if len(argv) != 3:
        sys.exit(__doc__)
    src, out = Path(argv[1]), Path(argv[2])
    owls = sorted(src.glob("*.owl"))
    if not owls:
        sys.exit(f"no .owl files in {src}")
    nt_dir = out / "nt"
    if nt_dir.exists() and any(nt_dir.iterdir()):
        sys.exit(f"{nt_dir} is not empty; convert into an empty directory")
    nt_dir.mkdir(parents=True, exist_ok=True)

    counts = {}
    for owl in owls:
        graph = Graph()
        graph.parse(owl, format="xml")
        graph.serialize(destination=nt_dir / f"{owl.stem}.nt", format="nt", encoding="utf-8")
        counts[owl.name] = len(graph)
        print(f"{owl.name}: {len(graph)} triples", file=sys.stderr)

    manifest = {
        "source_dir": str(src.resolve()),
        "triples": counts,
        "total_triples": sum(counts.values()),
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main(sys.argv)
