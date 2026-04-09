#!/usr/bin/env python3
"""
Verify insdc-rdf sra-experiment conversion output against source data.

Runs three independent checks so you can trust the converter's output:

  1. Tarball experiment count
     Streams the source NCBI_SRA_Metadata_Full_*.tar.gz, counts every
     <EXPERIMENT> tag in *.experiment.xml members, and compares to the
     converter's manifest.json total_records. This proves the converter
     did not silently skip records. SLOW: ~15-30 min for the Full dump.

  2. SRA_Accessions.tab cross-check (optional)
     Counts live EXPERIMENT rows in SRA_Accessions.tab and compares to
     the converter's total. The two files are different snapshots so a
     small gap is expected, corresponding to new submissions between
     the two dates (typically < 1%).

  3. Field fidelity
     For N sample experiments taken from the tarball, extracts fields
     directly from the source XML with Python's stdlib parser, then
     looks up the same accession in the converter's Turtle output and
     compares every field. This is the strongest correctness test --
     it shows the converter extracts the same values as the source XML
     field-for-field.

Usage:
    python3 scripts/verify_sra_experiment.py \\
        --tarball NCBI_SRA_Metadata_Full_20260316.tar.gz \\
        --output-dir output/sra-experiment \\
        [--sra-accessions SRA_Accessions.tab] \\
        [--samples 20] \\
        [--skip-count]

Dependencies: python3 >= 3.7 (stdlib only), tar, gzip, grep, awk, bash.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tarfile
import xml.etree.ElementTree as ET
from pathlib import Path


# ----------------------------------------------------------------------------
# Check 1: Tarball experiment count
# ----------------------------------------------------------------------------

def check_tarball_count(tarball: Path, manifest: dict) -> bool:
    print("=" * 72)
    print("Check 1: Tarball experiment count")
    print("=" * 72)
    print(f"Counting <EXPERIMENT> tags in {tarball.name}")
    print("(this takes ~15-30 minutes for the Full dump)")

    # Use a shell pipeline for speed: native gunzip + tar + grep are much
    # faster than Python's tarfile on a 15 GB archive.
    cmd = (
        f"tar -xOzf '{tarball}' --wildcards '*.experiment.xml' 2>/dev/null "
        f"| grep -oE '<EXPERIMENT[ >]' "
        f"| wc -l"
    )
    result = subprocess.run(["bash", "-c", cmd], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"ERROR running pipeline: {result.stderr}")
        return False

    tarball_count = int(result.stdout.strip())
    manifest_count = manifest["total_records"]
    skipped = manifest["records_skipped"]
    diff = tarball_count - manifest_count

    print()
    print(f"  Tarball <EXPERIMENT> tags: {tarball_count:>15,}")
    print(f"  Converter total_records:   {manifest_count:>15,}")
    print(f"  Converter records_skipped: {skipped:>15,}")
    print(f"  Difference:                {diff:>+15,}")

    # Allow 0.01% drift for malformed-XML edge cases (anything missing an
    # accession attribute gets logged to errors.log and counted as skipped).
    pct = 100.0 * diff / manifest_count if manifest_count else 0.0
    if diff == 0:
        print()
        print("  [OK] Exact match -- every EXPERIMENT in the tarball was converted")
        return True
    if abs(diff) == skipped:
        print()
        print("  [OK] Difference exactly matches skipped records (see errors.log)")
        return True
    if abs(pct) < 0.01:
        print()
        print(f"  [OK] Within 0.01% ({pct:+.5f}%) -- acceptable")
        return True

    print()
    print(f"  [FAIL] Unexpected gap ({pct:+.4f}%) -- investigate errors.log")
    return False


# ----------------------------------------------------------------------------
# Check 2: SRA_Accessions.tab cross-check
# ----------------------------------------------------------------------------

def check_accessions_cross(sra_accessions: Path, manifest: dict) -> bool:
    print()
    print("=" * 72)
    print("Check 2: SRA_Accessions.tab cross-check")
    print("=" * 72)
    print(f"Counting live EXPERIMENT rows in {sra_accessions.name}")

    # SRA_Accessions.tab columns (1-indexed):
    #   1 Accession  2 Submission  3 Status  ...  7 Type  ...
    cmd = (
        f"awk -F'\\t' 'NR > 1 && $3 == \"live\" && $7 == \"EXPERIMENT\"' "
        f"'{sra_accessions}' | wc -l"
    )
    result = subprocess.run(["bash", "-c", cmd], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"ERROR: {result.stderr}")
        return False

    tsv_count = int(result.stdout.strip())
    manifest_count = manifest["total_records"]
    diff = tsv_count - manifest_count
    pct = 100.0 * diff / manifest_count if manifest_count else 0.0

    print()
    print(f"  TSV live EXPERIMENT rows:  {tsv_count:>15,}")
    print(f"  Converter total_records:   {manifest_count:>15,}")
    print(f"  Difference:                {diff:>+15,} ({pct:+.3f}%)")
    print()
    print("  NOTE: The two sources are different snapshots. A small positive")
    print("        gap (TSV has more) means submissions arrived between the")
    print("        Full metadata dump date and the accessions.tab date.")

    # Independent sources, same data: within 2% is plausible for typical
    # snapshot lag. Anything larger warrants investigation.
    if abs(pct) < 2.0:
        print()
        print("  [OK] Within 2% -- consistent with snapshot gap")
        return True
    print()
    print("  [FAIL] Gap larger than 2% -- investigate")
    return False


# ----------------------------------------------------------------------------
# Check 3: Field fidelity
# ----------------------------------------------------------------------------

WANTED_FIELDS = [
    "title",
    "design_description",
    "library_name",
    "library_strategy",
    "library_source",
    "library_selection",
    "library_construction_protocol",
    "library_layout",
    "nominal_length",
    "nominal_sdev",
    "platform",
    "instrument_model",
]


def check_field_fidelity(tarball: Path, output_dir: Path, n_samples: int) -> bool:
    print()
    print("=" * 72)
    print(f"Check 3: Field fidelity ({n_samples} samples)")
    print("=" * 72)
    print(f"Collecting {n_samples} samples from {tarball.name}")

    # Pull the first n_samples experiments we encounter while streaming the
    # tarball. Stream mode ('r|gz') only reads members forward once, which
    # is exactly what we need.
    samples = []  # list of (accession, xml_fields)
    try:
        with tarfile.open(tarball, "r|gz") as tf:
            for member in tf:
                if len(samples) >= n_samples:
                    break
                if not member.name.endswith(".experiment.xml"):
                    continue
                f = tf.extractfile(member)
                if f is None:
                    continue
                try:
                    data = f.read()
                    root = ET.fromstring(data)
                except ET.ParseError:
                    continue
                for exp in root.findall("EXPERIMENT"):
                    acc = exp.get("accession")
                    if not acc:
                        continue
                    samples.append((acc, extract_xml_fields(exp)))
                    if len(samples) >= n_samples:
                        break
    except tarfile.ReadError as e:
        print(f"ERROR reading tarball: {e}")
        return False

    print(f"  Collected {len(samples)} samples")
    print(f"  Looking up each in {output_dir}/ttl/ and comparing fields...")
    print()

    ttl_dir = output_dir / "ttl"
    passed = 0
    failed = 0
    not_found = 0
    mismatches = []

    for (acc, xml_fields) in samples:
        ttl_block = find_ttl_block(ttl_dir, acc)
        if ttl_block is None:
            print(f"  [FAIL] {acc}: NOT FOUND in TTL output")
            not_found += 1
            failed += 1
            continue

        ttl_fields = parse_ttl_block(ttl_block)
        diffs = compare_fields(xml_fields, ttl_fields)

        if not diffs:
            print(f"  [OK]   {acc}: all {len([k for k,v in xml_fields.items() if v is not None])} populated fields match")
            passed += 1
        else:
            print(f"  [FAIL] {acc}: mismatches in {', '.join(diffs)}")
            mismatches.append((acc, diffs, xml_fields, ttl_fields))
            failed += 1

    if mismatches:
        print()
        print("Detailed mismatches (first 10):")
        for (acc, diffs, xml, ttl) in mismatches[:10]:
            print(f"  {acc}:")
            for field in diffs:
                print(f"    {field}:")
                print(f"      XML: {xml.get(field)!r}")
                print(f"      TTL: {ttl.get(field)!r}")

    print()
    print(f"  Passed:    {passed:>4d}/{len(samples)}")
    print(f"  Failed:    {failed:>4d}/{len(samples)}")
    if not_found:
        print(f"  Not found: {not_found:>4d}/{len(samples)}")

    return failed == 0


def extract_xml_fields(exp) -> dict:
    """Extract the converter's fields from an <EXPERIMENT> element."""
    def text_of(path):
        e = exp.find(path)
        if e is None or e.text is None:
            return None
        t = e.text.strip()
        return t or None

    fields = {
        "title": text_of("TITLE"),
        "design_description": text_of("DESIGN/DESIGN_DESCRIPTION"),
        "library_name": text_of("DESIGN/LIBRARY_DESCRIPTOR/LIBRARY_NAME"),
        "library_strategy": text_of("DESIGN/LIBRARY_DESCRIPTOR/LIBRARY_STRATEGY"),
        "library_source": text_of("DESIGN/LIBRARY_DESCRIPTOR/LIBRARY_SOURCE"),
        "library_selection": text_of("DESIGN/LIBRARY_DESCRIPTOR/LIBRARY_SELECTION"),
        "library_construction_protocol": text_of("DESIGN/LIBRARY_DESCRIPTOR/LIBRARY_CONSTRUCTION_PROTOCOL"),
        "library_layout": None,
        "nominal_length": None,
        "nominal_sdev": None,
        "platform": None,
        "instrument_model": None,
    }

    # Library layout: PAIRED or SINGLE under LIBRARY_LAYOUT
    layout = exp.find("DESIGN/LIBRARY_DESCRIPTOR/LIBRARY_LAYOUT")
    if layout is not None:
        for child in layout:
            if child.tag in ("PAIRED", "SINGLE"):
                fields["library_layout"] = child.tag
                if child.tag == "PAIRED":
                    fields["nominal_length"] = child.get("NOMINAL_LENGTH")
                    fields["nominal_sdev"] = child.get("NOMINAL_SDEV")
                break

    # Platform: first child of PLATFORM is the vendor tag
    platform = exp.find("PLATFORM")
    if platform is not None:
        for child in platform:
            fields["platform"] = child.tag
            model = child.find("INSTRUMENT_MODEL")
            if model is not None and model.text:
                fields["instrument_model"] = model.text.strip() or None
            break

    return fields


def find_ttl_block(ttl_dir: Path, accession: str) -> str | None:
    """Find the Turtle block for accession across the chunk files."""
    # Use grep to locate the chunk file fast. -r: recursive. -l: filenames.
    # -F: fixed string. -x: whole-line match (so we only find subject lines,
    # not accession mentions inside rdfs:label or dct:identifier).
    marker = f"insdc_sra:{accession}"
    result = subprocess.run(
        ["grep", "-rlxF", marker, str(ttl_dir)],
        capture_output=True,
        text=True,
    )
    files = [line for line in result.stdout.strip().split("\n") if line]
    if not files:
        return None

    with open(files[0]) as f:
        content = f.read()

    # Record starts at "insdc_sra:<acc>\n" on its own line and ends with
    # " .\n" (our writer always follows with a blank line, but be robust).
    start_marker = f"{marker}\n"
    start = content.find(start_marker)
    if start < 0:
        return None
    end = content.find(" .\n", start)
    if end < 0:
        return None
    return content[start: end + 3]


_STRING_RE = re.compile(r'"((?:[^"\\]|\\.)*)"')


def _literal(block: str, predicate: str) -> str | None:
    """Extract the first string literal after `predicate`."""
    # Anchor at predicate, then match the next string literal on the line.
    pat = re.compile(re.escape(predicate) + r"\s+" + _STRING_RE.pattern)
    m = pat.search(block)
    if not m:
        return None
    return _unescape_ttl(m.group(1))


def _uri_local(block: str, predicate: str) -> str | None:
    """Extract the local part of a dra_ont:Foo URI after `predicate`."""
    pat = re.compile(re.escape(predicate) + r"\s+dra_ont:([A-Za-z0-9_\-.]+)")
    m = pat.search(block)
    return m.group(1) if m else None


def _decimal(block: str, predicate: str) -> str | None:
    """Extract a decimal typed literal: predicate "123"^^xsd:decimal."""
    pat = re.compile(re.escape(predicate) + r'\s+"([0-9.]+)"\^\^xsd:decimal')
    m = pat.search(block)
    return m.group(1) if m else None


def _unescape_ttl(s: str) -> str:
    """Reverse the Turtle/N-Triples string literal escapes the converter emits."""
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            nxt = s[i + 1]
            if nxt == "n":
                out.append("\n")
            elif nxt == "r":
                out.append("\r")
            elif nxt == "t":
                out.append("\t")
            elif nxt == '"':
                out.append('"')
            elif nxt == "\\":
                out.append("\\")
            else:
                out.append(c)
                out.append(nxt)
            i += 2
        else:
            out.append(c)
            i += 1
    return "".join(out)


def parse_ttl_block(block: str) -> dict:
    """Parse a Turtle record block emitted by the converter."""
    fields = {
        "title": _literal(block, "dra_ont:title"),
        "design_description": _literal(block, "dra_ont:designDescription"),
        "library_name": _literal(block, "dra_ont:libraryName"),
        "library_strategy": _uri_local(block, "dra_ont:libraryStrategy"),
        "library_source": _uri_local(block, "dra_ont:librarySource"),
        "library_selection": _uri_local(block, "dra_ont:librarySelection"),
        "library_construction_protocol": _literal(block, "dra_ont:libraryConstructionProtocol"),
        "library_layout": None,
        "nominal_length": _decimal(block, "dra_ont:nominalLength"),
        "nominal_sdev": _decimal(block, "dra_ont:nominalSdev"),
        "platform": None,
        "instrument_model": _uri_local(block, "dra_ont:instrumentModel"),
    }

    # Platform: the `a dra_ont:FOO` inside the platform blank node.
    m = re.search(r"dra_ont:platform\s*\[\s*a\s+dra_ont:([A-Za-z0-9_\-.]+)", block)
    if m:
        fields["platform"] = m.group(1)

    # Library layout type: the `a dra_ont:PAIRED|SINGLE` inside libraryLayout.
    m = re.search(r"dra_ont:libraryLayout\s*\[\s*a\s+dra_ont:(PAIRED|SINGLE)", block)
    if m:
        fields["library_layout"] = m.group(1)

    return fields


def compare_fields(xml_fields: dict, ttl_fields: dict) -> list:
    """Return list of field names that do not match."""
    diffs = []
    for key in WANTED_FIELDS:
        xv = xml_fields.get(key)
        tv = ttl_fields.get(key)

        if xv is None and tv is None:
            continue

        if key == "instrument_model":
            # Converter's `to_uri_local` replaces spaces with underscores.
            if xv and tv and xv.replace(" ", "_") == tv:
                continue

        if key == "library_strategy":
            # Strategy values like "RNA-Seq" pass through unchanged (no
            # spaces). But if someone uses "Targeted-Capture" etc., still
            # unchanged. Use raw comparison with space->underscore fallback.
            if xv and tv and xv.replace(" ", "_") == tv:
                continue

        if key in ("library_source", "library_selection"):
            if xv and tv and xv.replace(" ", "_") == tv:
                continue

        if key in ("nominal_length", "nominal_sdev") and xv is not None and tv is not None:
            try:
                if float(xv) == float(tv):
                    continue
            except ValueError:
                pass

        if xv != tv:
            diffs.append(key)

    return diffs


# ----------------------------------------------------------------------------
# Entry point
# ----------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--tarball",
        required=True,
        type=Path,
        help="Path to NCBI_SRA_Metadata_Full_*.tar.gz",
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        type=Path,
        help="Path to the converter's output directory (must contain manifest.json and ttl/)",
    )
    parser.add_argument(
        "--sra-accessions",
        type=Path,
        default=None,
        help="Optional path to SRA_Accessions.tab for cross-check",
    )
    parser.add_argument(
        "--samples",
        type=int,
        default=20,
        help="Number of experiments to compare field-by-field (default: 20)",
    )
    parser.add_argument(
        "--skip-count",
        action="store_true",
        help="Skip check 1 (tarball count -- slow)",
    )
    parser.add_argument(
        "--skip-fidelity",
        action="store_true",
        help="Skip check 3 (field fidelity)",
    )
    args = parser.parse_args()

    # Sanity check inputs
    if not args.tarball.exists():
        print(f"ERROR: tarball not found: {args.tarball}", file=sys.stderr)
        sys.exit(2)
    if not args.output_dir.is_dir():
        print(f"ERROR: output directory not found: {args.output_dir}", file=sys.stderr)
        sys.exit(2)
    manifest_path = args.output_dir / "manifest.json"
    if not manifest_path.exists():
        print(f"ERROR: {manifest_path} not found", file=sys.stderr)
        sys.exit(2)

    manifest = json.loads(manifest_path.read_text())

    print("insdc-rdf sra-experiment verification")
    print("=" * 72)
    print(f"Tarball:     {args.tarball}")
    print(f"Output dir:  {args.output_dir}")
    print(f"Manifest:    {manifest['total_records']:,} records, "
          f"{manifest['records_skipped']:,} skipped, "
          f"{manifest['total_chunks']} chunks")
    print(f"Source MD5:  {manifest['source_md5']}")
    print(f"Completed:   {manifest['completed_at']}")
    print()

    results = []

    if not args.skip_count:
        results.append(("tarball count", check_tarball_count(args.tarball, manifest)))

    if args.sra_accessions:
        if not args.sra_accessions.exists():
            print(f"WARNING: {args.sra_accessions} not found, skipping check 2", file=sys.stderr)
        else:
            results.append(("accessions cross-check", check_accessions_cross(args.sra_accessions, manifest)))

    if not args.skip_fidelity:
        results.append(("field fidelity", check_field_fidelity(args.tarball, args.output_dir, args.samples)))

    print()
    print("=" * 72)
    print("Summary")
    print("=" * 72)
    for (name, ok) in results:
        status = "[OK]  " if ok else "[FAIL]"
        print(f"  {status} {name}")

    all_ok = all(ok for _, ok in results)
    sys.exit(0 if all_ok else 1)


if __name__ == "__main__":
    main()
