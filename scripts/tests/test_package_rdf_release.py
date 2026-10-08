import copy
import gzip
import hashlib
import importlib.util
import json
import re
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

import pytest

REAL_REPO = Path(__file__).resolve().parents[2]
SCRIPT = REAL_REPO / "scripts" / "package_rdf_release.py"

_spec = importlib.util.spec_from_file_location("package_rdf_release", str(SCRIPT))
prr = importlib.util.module_from_spec(_spec)
sys.modules["package_rdf_release"] = prr
_spec.loader.exec_module(prr)

RID = "2099-01_test_rdf"
COMMIT = "0123456789abcdef0123456789abcdef01234567"
FMTS = ("nt", "ttl", "jsonld")

ALPHA_NT = [
    '<http://e/a> <http://e/p> "say \\"hi\\"" .\n',
    '<http://e/a> <http://e/q> "café 日本" .\n',
    '<http://e/a> <http://e/r> "x" .\n',
]
ALPHA_NT2 = [
    '<http://e/b> <http://e/p> "1" .\n',
    '<http://e/b> <http://e/p> "2" .\n',
]
BETA_NT = ['<http://e/c> <http://e/p> "%d" .\n' % i for i in range(4)]


def _write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(text.encode("utf-8"))


def _source(root, name, nt_chunks, completed, started):
    base = root / name
    for i, lines in enumerate(nt_chunks):
        stem = "chunk_%04d" % i
        _write(base / "nt" / (stem + ".nt"), "".join(lines))
        _write(base / "ttl" / (stem + ".ttl"), "# ttl %s %d\n" % (name, i))
        _write(base / "jsonld" / (stem + ".jsonld"), '{"@graph": [], "n": %d}\n' % i)
    manifest = {
        "source_file": name + ".xml",
        "total_chunks": len(nt_chunks),
        "total_records": sum(len(c) for c in nt_chunks),
        "records_skipped": 0,
        "completed_at": completed,
    }
    (base / "manifest.json").write_text(json.dumps(manifest))
    (base / "progress.json").write_text(json.dumps({"started_at": started}))
    (base / "errors.log").write_text("")


@pytest.fixture
def env(tmp_path):
    data = tmp_path / "data"
    _source(data, "alpha", [ALPHA_NT, ALPHA_NT2],
            "2026-10-08T01:00:00+00:00", "2026-10-08T00:00:00+00:00")
    _source(data, "beta", [BETA_NT],
            "2026-10-08T03:00:00+00:00", "2026-10-08T02:00:00+00:00")
    repo = tmp_path / "repo"
    for f in ("model.yaml", "shape.shex", "schema.svg"):
        _write(repo / "config" / "alpha" / f, "alpha " + f + "\n")
    for f in ("model.yaml", "shape.shex"):
        _write(repo / "config" / "beta" / f, "beta " + f + "\n")
    spec = {
        "release_id": RID,
        "bucket": "testbucket",
        "base_url": "https://example.org/base",
        "name": "Test release",
        "description": "A test release.",
        "date_published": "2099-01-02",
        "license": {"id": "https://example.org/license", "name": "Test License"},
        "authors": [
            {"id": "#ann", "name": "Ann A", "affiliation": ["#org"]},
            {"id": "#bob", "name": "Bob B", "affiliation": ["#org"]},
        ],
        "organizations": [{"id": "#org", "name": "Org Inc."}],
        "software": {
            "name": "insdc-rdf",
            "description": "Converter.",
            "version": "v1.2.3",
            "commit": COMMIT,
            "repository": "https://example.org/repo",
        },
        "inputs": [
            {"id": "https://example.org/in1.xml", "type": "File", "name": "in1.xml",
             "content_size": 1234567, "date_modified": "2099-01-01T00:00:00Z",
             "md5": "d41d8cd98f00b204e9800998ecf8427e"},
            {"id": "https://example.org/in2/", "type": "Dataset", "name": "in2 dataset"},
        ],
        "sources": [
            {"name": "alpha", "title": "Alpha", "dir": "alpha", "schema_dir": "config/alpha",
             "inputs": ["https://example.org/in1.xml"]},
            {"name": "beta", "title": "Beta", "dir": "beta", "schema_dir": "config/beta",
             "inputs": ["https://example.org/in2/"]},
        ],
        "tarball": False,
        "derived_from": ["https://example.org/in1.xml"],
        "readme": {"intro": ["Intro paragraph one.", "Intro paragraph two."],
                   "notes": ["First note with `code`.", "Second note."]},
    }
    return {"tmp": tmp_path, "data": data, "repo": repo, "spec": spec}


def run(env, out=None, spec=None, **kw):
    out = out or env["tmp"] / "out"
    return prr.package(spec or env["spec"], str(out), str(env["data"]),
                       str(env["repo"]), jobs=2, **kw)


def files_under(d):
    return sorted(p for p in d.rglob("*") if p.is_file())


def test_chunks_round_trip(env):
    run(env)
    rel = env["tmp"] / "out" / RID
    for src in ("alpha", "beta"):
        for fmt in FMTS:
            for f in (env["data"] / src / fmt).iterdir():
                gz = rel / src / fmt / (f.name + ".gz")
                assert gz.is_file()
                assert gzip.decompress(gz.read_bytes()) == f.read_bytes()


def test_gzip_is_deterministic(env):
    run(env, out=env["tmp"] / "o1")
    run(env, out=env["tmp"] / "o2")
    gzs = sorted((env["tmp"] / "o1" / RID).rglob("*.gz"))
    assert len(gzs) == 9
    for a in gzs:
        b = env["tmp"] / "o2" / a.relative_to(env["tmp"] / "o1")
        data = a.read_bytes()
        assert data == b.read_bytes()
        assert data[4:8] == b"\x00\x00\x00\x00"
        assert data[3] & 0x08 == 0


def test_checksums_verify(env):
    run(env)
    rel = env["tmp"] / "out" / RID
    r = subprocess.run(["sha256sum", "-c", "provenance/checksums.sha256"],
                       cwd=str(rel), stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert r.returncode == 0, r.stdout + r.stderr
    lines = (rel / "provenance" / "checksums.sha256").read_text().splitlines()
    paths = [l.split("  ", 1)[1] for l in lines]
    assert paths == sorted(paths)
    assert "ro-crate-metadata.json" not in paths
    assert "provenance/checksums.sha256" not in paths
    for p in ("README.md", "provenance/triples.tsv", "provenance/alpha.manifest.json",
              "schema/alpha/shape.shex"):
        assert p in paths


def _refs(node):
    if isinstance(node, dict):
        if set(node) == {"@id"}:
            yield node["@id"]
        else:
            for v in node.values():
                for r in _refs(v):
                    yield r
    elif isinstance(node, list):
        for v in node:
            for r in _refs(v):
                yield r


def test_ro_crate_metadata(env):
    run(env)
    rel = env["tmp"] / "out" / RID
    crate = json.loads((rel / "ro-crate-metadata.json").read_text(encoding="utf-8"))
    assert crate["@context"] == "https://w3id.org/ro/crate/1.1/context"
    graph = crate["@graph"]
    by_id = {}
    for e in graph:
        assert e["@id"] not in by_id
        by_id[e["@id"]] = e
    on_disk = [p for p in files_under(rel) if p.name != "ro-crate-metadata.json"]
    for p in on_disk:
        relpath = str(p.relative_to(rel))
        e = by_id[relpath]
        assert e["@type"] == "File"
        assert e["name"] == p.name
        assert e["contentSize"] == str(p.stat().st_size)
        assert e["sha256"] == hashlib.sha256(p.read_bytes()).hexdigest()
    input_ids = {i["id"] for i in env["spec"]["inputs"]}
    files = [e for e in graph if e["@type"] == "File" and e["@id"] not in input_ids]
    assert len(files) == len(on_disk)
    nt = by_id["alpha/nt/chunk_0000.nt.gz"]
    assert nt["encodingFormat"] == ["application/n-triples", "application/gzip"]
    assert by_id["./"]["hasPart"] == [{"@id": "alpha/"}, {"@id": "beta/"},
                                      {"@id": "schema/"}, {"@id": "provenance/"},
                                      {"@id": "README.md"}]
    act = by_id["#convert-alpha"]
    assert act["startTime"] == "2026-10-08T00:00:00+00:00"
    assert act["endTime"] == "2026-10-08T01:00:00+00:00"
    assert act["result"] == {"@id": "alpha/"}
    soft = by_id["#insdc-rdf-0123456"]
    assert soft["url"].endswith(COMMIT)
    assert by_id["https://example.org/in1.xml"]["contentSize"] == "1234567"
    assert "contentSize" not in by_id["https://example.org/in2/"]
    for r in _refs(graph):
        if r.startswith("#") or not re.match(r"^[a-z]+:", r):
            assert r in by_id, r


def test_triple_counts(env):
    entry = run(env)
    tsv = (env["tmp"] / "out" / RID / "provenance" / "triples.tsv").read_text()
    assert tsv == "source\ttriples\nalpha\t5\nbeta\t4\n"
    assert entry["triple_count"] == 9


def test_expected_triples_mismatch_aborts(env):
    spec = copy.deepcopy(env["spec"])
    spec["sources"][0]["expected_triples"] = 6
    with pytest.raises(prr.PackageError) as ei:
        run(env, spec=spec)
    msg = str(ei.value)
    assert "alpha" in msg and "5" in msg and "6" in msg
    assert not (env["tmp"] / "out" / RID).exists()


def test_refuses_existing_output(env):
    for name in (RID, RID + ".partial"):
        out = env["tmp"] / ("out-" + name)
        (out / name).mkdir(parents=True)
        (out / name / "keep.txt").write_text("keep")
        with pytest.raises(prr.PackageError, match="exists"):
            run(env, out=out)
        assert [p.name for p in (out / name).iterdir()] == ["keep.txt"]
        assert sorted(p.name for p in out.iterdir()) == [name]
    spec = copy.deepcopy(env["spec"])
    spec["tarball"] = True
    out = env["tmp"] / "out-tar"
    out.mkdir()
    (out / (RID + ".tar.gz")).write_bytes(b"x")
    with pytest.raises(prr.PackageError, match="exists"):
        run(env, out=out, spec=spec)
    assert not (out / RID).exists()


def test_chunk_count_mismatch_aborts(env):
    (env["data"] / "alpha" / "ttl" / "chunk_0001.ttl").unlink()
    with pytest.raises(prr.PackageError) as ei:
        run(env)
    assert "alpha" in str(ei.value) and "ttl" in str(ei.value)
    assert not (env["tmp"] / "out" / (RID + ".partial")).exists()
    assert not (env["tmp"] / "out" / RID).exists()


def test_chunk_stem_mismatch_aborts(env):
    ttl = env["data"] / "alpha" / "ttl"
    (ttl / "chunk_0001.ttl").rename(ttl / "chunk_0007.ttl")
    with pytest.raises(prr.PackageError, match="alpha"):
        run(env)
    assert not (env["tmp"] / "out" / (RID + ".partial")).exists()


def test_human_size_rounds_before_choosing_unit():
    assert prr.human_size(1023) == "1023 B"
    assert prr.human_size(1024) == "1.0 KB"
    assert prr.human_size(1024 * 1024 - 1) == "1.0 MB"
    assert prr.human_size(3 * 1024 ** 3) == "3.0 GB"


def test_worker_failure_is_package_error(env):
    import os
    if os.geteuid() == 0:
        pytest.skip("root ignores file modes")
    victim = env["data"] / "alpha" / "ttl" / "chunk_0000.ttl"
    victim.chmod(0)
    try:
        with pytest.raises(prr.PackageError, match="chunk_0000.ttl"):
            run(env)
    finally:
        victim.chmod(0o644)
    assert not (env["tmp"] / "out" / RID).exists()


def test_refuses_existing_tarball_partial(env):
    spec = copy.deepcopy(env["spec"])
    spec["tarball"] = True
    out = env["tmp"] / "out-tp"
    out.mkdir()
    (out / (RID + ".tar.gz.partial")).write_bytes(b"x")
    with pytest.raises(prr.PackageError, match="exists"):
        run(env, out=out, spec=spec)
    assert not (out / RID).exists()


def test_index_entry_written_after_tarball(env, monkeypatch):
    spec = copy.deepcopy(env["spec"])
    spec["tarball"] = True
    seen = {}
    real = Path.write_bytes

    def spy(self, data):
        if self.name.endswith(".index-entry.json"):
            seen["tar_done"] = (self.parent / (RID + ".tar.gz")).is_file()
            seen["partial_gone"] = not (self.parent / (RID + ".tar.gz.partial")).exists()
        return real(self, data)

    monkeypatch.setattr(Path, "write_bytes", spy)
    run(env, spec=spec)
    assert seen == {"tar_done": True, "partial_gone": True}


def test_tarball(env):
    spec = copy.deepcopy(env["spec"])
    spec["tarball"] = True
    entry = run(env, spec=spec)
    tb = env["tmp"] / "out" / (RID + ".tar.gz")
    with tarfile.open(str(tb)) as t:
        names = t.getnames()
    assert all(n == RID or n.startswith(RID + "/") for n in names)
    assert RID + "/ro-crate-metadata.json" in names
    assert entry["tarball"] == "releases/%s.tar.gz" % RID
    assert entry["tarball_bytes"] == tb.stat().st_size

    entry2 = run(env, out=env["tmp"] / "out2")
    assert entry2["tarball"] is None
    assert "tarball_bytes" not in entry2
    assert not (env["tmp"] / "out2" / (RID + ".tar.gz")).exists()


def test_index_entry(env):
    entry = run(env)
    assert list(entry) == [
        "release_id", "kind", "prefix", "tarball", "name", "description",
        "date_published", "license", "authors", "ro_crate_profile", "formats",
        "triple_count", "derived_from", "file_count", "total_bytes",
    ]
    assert entry["kind"] == "rdf"
    assert entry["prefix"] == "releases/%s/" % RID
    assert entry["license"] == "https://example.org/license"
    assert entry["authors"] == ["Ann A", "Bob B"]
    assert entry["formats"] == ["nt", "ttl", "jsonld"]
    files = files_under(env["tmp"] / "out" / RID)
    assert entry["file_count"] == len(files)
    assert entry["total_bytes"] == sum(p.stat().st_size for p in files)
    on_disk = json.loads((env["tmp"] / "out" / (RID + ".index-entry.json")).read_text())
    assert on_disk == entry


def test_readme(env):
    run(env)
    text = (env["tmp"] / "out" / RID / "README.md").read_text(encoding="utf-8")
    assert "`%s`" % RID in text
    assert "# Test release" in text
    assert "Total: 9 records, 9 triples, 3 chunks per format." in text
    assert "aws s3 sync --no-sign-request s3://testbucket/releases/%s/" % RID in text
    assert "sha256sum -c --ignore-missing provenance/checksums.sha256" in text
    assert "d41d8cd98f00b204e9800998ecf8427e" in text
    assert "1,234,567" in text
    row = [l for l in text.splitlines() if "in2 dataset" in l][0]
    assert "—" in row
    for n in env["spec"]["readme"]["notes"]:
        assert n in text
    assert "tar xzf" not in text
    assert "https://example.org/base/releases/%s/alpha/nt/chunk_0000.nt.gz" % RID in text


def test_cli(env):
    tmp = env["tmp"]
    tmp_repo = tmp / "cli_repo"
    shutil.copytree(str(env["repo"]), str(tmp_repo))
    (tmp_repo / "scripts").mkdir()
    script = tmp_repo / "scripts" / "package_rdf_release.py"
    shutil.copy(str(SCRIPT), str(script))
    spec_path = tmp / "spec.json"
    spec_path.write_text(json.dumps(env["spec"]))
    out = tmp / "cli_out"
    r = subprocess.run([sys.executable, str(script), str(spec_path), str(out),
                        "--data-root", str(env["data"]), "--jobs", "2"],
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                       universal_newlines=True)
    assert r.returncode == 0, r.stderr
    assert (out / RID / "ro-crate-metadata.json").is_file()
    assert "alpha" in r.stdout

    bad = copy.deepcopy(env["spec"])
    bad["sources"][0]["expected_triples"] = 6
    spec_path.write_text(json.dumps(bad))
    out2 = tmp / "cli_out2"
    r = subprocess.run([sys.executable, str(script), str(spec_path), str(out2),
                        "--data-root", str(env["data"]), "--jobs", "2"],
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                       universal_newlines=True)
    assert r.returncode == 1
    assert "alpha: counted 5 triples, expected 6" in r.stderr


def test_committed_specs():
    paths = sorted((REAL_REPO / "releases").glob("*.json"))
    assert len(paths) >= 2
    required = ["release_id", "bucket", "base_url", "name", "description",
                "date_published", "license", "authors", "organizations", "software",
                "inputs", "sources", "tarball", "derived_from", "readme"]
    for p in paths:
        spec = json.loads(p.read_text(encoding="utf-8"))
        for k in required:
            assert k in spec, (p.name, k)
        assert re.match(r"^[0-9]{4}-[0-9]{2}_[A-Za-z0-9._-]+$", spec["release_id"])
        assert p.name == spec["release_id"] + ".json"
        ids = {i["id"] for i in spec["inputs"]}
        for s in spec["sources"]:
            assert (REAL_REPO / s["schema_dir"]).is_dir()
            assert set(s["inputs"]) <= ids
        commit = spec["software"]["commit"]
        assert re.match(r"^[0-9a-f]{40}$", commit)
        r = subprocess.run(["git", "merge-base", "--is-ancestor", commit, "HEAD"],
                           cwd=str(REAL_REPO))
        assert r.returncode == 0, (p.name, commit)
