import json
import subprocess
import sys
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "catalog_add_release.py"

INDEX = {
    "name": "BioSample Plus",
    "schema_version": 1,
    "updated": "2026-10-01",
    "releases": [
        {"release_id": "2026-06_a", "zeta": 1, "alpha": ["é", 2]},
        {"release_id": "2026-06_b", "run_count": 3},
    ],
}


def run(*args):
    return subprocess.run(
        [sys.executable, str(SCRIPT)] + [str(a) for a in args],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True,
    )


def write(path, obj):
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return path


def setup(tmp_path):
    return write(tmp_path / "index.json", INDEX)


def test_appends_two_entries_in_argument_order(tmp_path):
    idx = setup(tmp_path)
    e1 = write(tmp_path / "e1.json", {"release_id": "2026-10_x", "kind": "rdf"})
    e2 = write(tmp_path / "e2.json", {"release_id": "2026-10_w"})
    r = run(idx, e1, e2, "--updated", "2026-10-09")
    assert r.returncode == 0, r.stderr
    data = json.loads(idx.read_text(encoding="utf-8"))
    assert [x["release_id"] for x in data["releases"]] == [
        "2026-06_a", "2026-06_b", "2026-10_x", "2026-10_w"]
    assert idx.read_text(encoding="utf-8").endswith("}\n")


def test_duplicate_refused_file_untouched(tmp_path):
    idx = setup(tmp_path)
    before = idx.read_bytes()
    ok = write(tmp_path / "ok.json", {"release_id": "2026-10_new"})
    dup = write(tmp_path / "dup.json", {"release_id": "2026-06_b"})
    r = run(idx, ok, dup)
    assert r.returncode == 1
    assert "2026-06_b" in r.stderr
    assert idx.read_bytes() == before


def test_duplicate_within_arguments_refused(tmp_path):
    idx = setup(tmp_path)
    before = idx.read_bytes()
    e = write(tmp_path / "e.json", {"release_id": "2026-10_new"})
    assert run(idx, e, e).returncode == 1
    assert idx.read_bytes() == before


def test_existing_entries_and_keys_unchanged(tmp_path):
    idx = setup(tmp_path)
    e = write(tmp_path / "e.json", {"release_id": "2026-10_x"})
    assert run(idx, e, "--updated", "2026-10-09").returncode == 0
    data = json.loads(idx.read_text(encoding="utf-8"))
    assert data["releases"][:2] == INDEX["releases"]
    assert list(data["releases"][0]) == ["release_id", "zeta", "alpha"]
    assert list(data) == list(INDEX)
    assert data["name"] == "BioSample Plus"
    assert "é" in idx.read_text(encoding="utf-8")  # ensure_ascii=False


def test_updated_flag_and_default(tmp_path):
    idx = setup(tmp_path)
    e = write(tmp_path / "e.json", {"release_id": "2026-10_x"})
    run(idx, e, "--updated", "2030-01-02")
    assert json.loads(idx.read_text(encoding="utf-8"))["updated"] == "2030-01-02"

    idx2 = setup(tmp_path)
    e2 = write(tmp_path / "e2.json", {"release_id": "2026-10_y"})
    assert run(idx2, e2).returncode == 0
    import datetime
    today = datetime.datetime.utcnow().strftime("%Y-%m-%d")
    got = json.loads(idx2.read_text(encoding="utf-8"))["updated"]
    assert got in (today, (datetime.datetime.utcnow() - datetime.timedelta(days=1)).strftime("%Y-%m-%d"))
