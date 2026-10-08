import json
import os
import subprocess
from pathlib import Path

import pytest

REAL_REPO = Path(__file__).resolve().parents[2]
SCRIPT = REAL_REPO / "scripts" / "upload_rdf_release.sh"
RID = "2099-01_test_rdf"

# A stand-in for the `aws s3` commands the script uses, backed by a local directory:
# s3://<bucket>/<key> is $STUB_ROOT/<bucket>/<key>, and the Content-Type of each
# uploaded key is kept in $STUB_ROOT/.meta/<bucket>/<key>.
AWS_STUB = r'''#!/usr/bin/env python3
import fnmatch, os, shutil, sys
from pathlib import Path

root = Path(os.environ["STUB_ROOT"])
args = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as log:
    log.write(" ".join(args) + "\n")
if args[0] != "s3":
    sys.exit(2)
cmd, rest = args[1], args[2:]
pos, filters, opts, flags = [], [], {}, set()
i = 0
while i < len(rest):
    a = rest[i]
    if a in ("--exclude", "--include"):
        filters.append((a[2:], rest[i + 1])); i += 2
    elif a in ("--content-type", "--cache-control"):
        opts[a] = rest[i + 1]; i += 2
    elif a.startswith("--"):
        flags.add(a); i += 1
    else:
        pos.append(a); i += 1


def split(url):
    bucket, _, key = url[len("s3://"):].partition("/")
    return bucket, key


def objects(bucket):
    base = root / bucket
    if not base.is_dir():
        return {}
    return {p.relative_to(base).as_posix(): p.stat().st_size
            for p in base.rglob("*") if p.is_file()}


def put(src, url):
    bucket, key = split(url)
    if "--dryrun" in flags:
        print("(dryrun) upload: %s to %s" % (src, url))
        return
    dst = root / bucket / key
    dst.parent.mkdir(parents=True, exist_ok=True)
    data = Path(src).read_bytes()
    if os.environ.get("STUB_TRUNCATE") and key.endswith(".tar.gz"):
        data = data[:-1]
    dst.write_bytes(data)
    meta = root / ".meta" / bucket / key
    meta.parent.mkdir(parents=True, exist_ok=True)
    meta.write_text(opts.get("--content-type", ""))
    print("upload: %s to %s" % (src, url))


def selected(rel):
    keep = True
    for kind, pat in filters:
        if fnmatch.fnmatch(rel, pat):
            keep = kind == "include"
    return keep


if cmd == "ls":
    if os.environ.get("STUB_LS_FAIL"):
        sys.stderr.write("An error occurred (AccessDenied)\n")
        sys.exit(255)
    bucket, prefix = split(pos[0])
    objs = objects(bucket)
    keys = sorted(k for k in objs if k.startswith(prefix))
    if "--recursive" in flags:
        for k in keys:
            print("2099-01-01 00:00:00 %10d %s" % (objs[k], k))
        if "--summarize" in flags:
            print("\nTotal Objects: %d\n   Total Size: %d" % (len(keys), sum(objs[k] for k in keys)))
    else:
        level = prefix[:prefix.rfind("/") + 1]
        seen = set()
        for k in keys:
            sub = k[len(level):]
            if "/" in sub:
                d = sub.split("/")[0] + "/"
                if d not in seen:
                    seen.add(d)
                    print("                           PRE %s" % d)
            else:
                print("2099-01-01 00:00:00 %10d %s" % (objs[k], sub))
    sys.exit(0 if keys else 1)
elif cmd in ("cp", "sync"):
    src, dst = pos
    if cmd == "cp" and "--recursive" not in flags:
        put(src, dst)
        sys.exit(0)
    bucket, prefix = split(dst)
    objs = objects(bucket)
    for p in sorted(Path(src).rglob("*")):
        rel = p.relative_to(src).as_posix()
        if not p.is_file() or not selected(rel):
            continue
        if cmd == "sync" and "--size-only" in flags and \
                objs.get(prefix + rel) == p.stat().st_size:
            continue
        put(str(p), dst + rel)
    sys.exit(0)
sys.exit(2)
'''

FILES = {
    "alpha/nt/chunk_0000.nt.gz": b"\x1f\x8bnt" * 10,
    "alpha/ttl/chunk_0000.ttl.gz": b"\x1f\x8bttl" * 10,
    "alpha/jsonld/chunk_0000.jsonld.gz": b"\x1f\x8bjsonld" * 10,
    "schema/alpha/model.yaml": b"model\n",
    "schema/alpha/shape.shex": b"shape\n",
    "schema/alpha/schema.svg": b"<svg/>\n",
    "provenance/triples.tsv": b"source\ttriples\nalpha\t1\n",
    "provenance/checksums.sha256": b"x  README.md\n",
    "provenance/alpha.manifest.json": b"{}\n",
    "README.md": b"# Test\n",
    "ro-crate-metadata.json": b"{}\n",
}
CTYPE = {
    ".gz": "application/gzip",
    ".json": "application/json",
    ".md": "text/markdown; charset=utf-8",
    ".sha256": "text/plain; charset=utf-8",
    ".tsv": "text/plain; charset=utf-8",
    ".shex": "text/plain; charset=utf-8",
    ".yaml": "text/plain; charset=utf-8",
    ".svg": "image/svg+xml",
}
TARBALL = b"\x1f\x8b" + b"t" * 98


def _package(out, tarball):
    for rel, data in FILES.items():
        p = out / RID / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(data)
    entry = {"release_id": RID, "tarball": None}
    if tarball:
        (out / (RID + ".tar.gz")).write_bytes(TARBALL)
        entry = {"release_id": RID, "tarball": "releases/%s.tar.gz" % RID,
                 "tarball_bytes": len(TARBALL)}
    (out / (RID + ".index-entry.json")).write_text(json.dumps(entry))


@pytest.fixture
def env(tmp_path):
    bindir = tmp_path / "bin"
    bindir.mkdir()
    stub = bindir / "aws"
    stub.write_text(AWS_STUB)
    stub.chmod(0o755)
    e = dict(os.environ)
    e.update(PATH="%s:%s" % (bindir, e["PATH"]), STUB_ROOT=str(tmp_path / "s3"),
             STUB_LOG=str(tmp_path / "aws.log"), BUCKET="testbucket")
    for k in ("STUB_TRUNCATE", "STUB_LS_FAIL"):
        e.pop(k, None)
    out = tmp_path / "out"
    _package(out, tarball=True)
    return {"tmp": tmp_path, "env": e, "out": out, "remote": tmp_path / "s3" / "testbucket"}


def _run(env, *flags, **extra):
    e = dict(env["env"], **extra)
    return subprocess.run(["bash", str(SCRIPT)] + list(flags) + [str(env["out"]), RID],
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          universal_newlines=True, env=e)


def _calls(env):
    p = Path(env["env"]["STUB_LOG"])
    return p.read_text().splitlines() if p.exists() else []


def _remote(env):
    base = env["remote"]
    if not base.is_dir():
        return {}
    return {p.relative_to(base).as_posix(): p.read_bytes()
            for p in base.rglob("*") if p.is_file()}


def _ctype(env, key):
    return (env["tmp"] / "s3" / ".meta" / "testbucket" / key).read_text()


def _expected_remote():
    want = {"releases/%s/%s" % (RID, rel): data for rel, data in FILES.items()}
    want["releases/%s.tar.gz" % RID] = TARBALL
    return want


def _check_content_types(env):
    for rel in FILES:
        assert _ctype(env, "releases/%s/%s" % (RID, rel)) == CTYPE[os.path.splitext(rel)[1]]
    assert _ctype(env, "releases/%s.tar.gz" % RID) == "application/gzip"


def test_upload(env):
    r = _run(env)
    assert r.returncode == 0, r.stdout + r.stderr
    assert _remote(env) == _expected_remote()
    _check_content_types(env)
    assert "ok: %d files" % len(FILES) in r.stdout
    assert "tarball: %d bytes" % len(TARBALL) in r.stdout


def test_dryrun_writes_nothing(env):
    r = _run(env, "--dryrun")
    assert r.returncode == 0, r.stdout + r.stderr
    assert _remote(env) == {}
    uploads = [c for c in _calls(env) if c.startswith("s3 cp")]
    assert len(uploads) == 6 and all("--dryrun" in c for c in uploads)
    assert "dry run: skipping" in r.stdout


def test_refuses_existing_prefix_without_resume(env):
    (env["remote"] / "releases" / RID / "README.md").parent.mkdir(parents=True)
    (env["remote"] / "releases" / RID / "README.md").write_bytes(b"old")
    r = _run(env)
    assert r.returncode == 1
    assert "already holds objects; releases are immutable" in r.stderr
    assert "--resume" in r.stderr
    assert not [c for c in _calls(env) if c.startswith(("s3 cp", "s3 sync"))]


def test_refuses_existing_tarball_without_resume(env):
    (env["remote"] / "releases").mkdir(parents=True)
    (env["remote"] / "releases" / (RID + ".tar.gz")).write_bytes(TARBALL)
    r = _run(env)
    assert r.returncode == 1
    assert "%s.tar.gz already exists" % RID in r.stderr
    assert not [c for c in _calls(env) if c.startswith(("s3 cp", "s3 sync"))]


def test_other_release_with_longer_name_is_not_a_conflict(env):
    other = env["remote"] / "releases" / (RID + "x") / "README.md"
    other.parent.mkdir(parents=True)
    other.write_bytes(b"other")
    (env["remote"] / "releases" / (RID + ".tar.gz.bak")).write_bytes(b"bak")
    r = _run(env)
    assert r.returncode == 0, r.stdout + r.stderr


def _interrupted(env):
    """The remote state after an upload that stopped during the second group."""
    for rel, data in FILES.items():
        if rel.endswith(".gz") or rel == "README.md":
            p = env["remote"] / "releases" / RID / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(data)
    stale = env["remote"] / "releases" / RID / "ro-crate-metadata.json"
    stale.write_bytes(b"{")  # a different size: re-uploaded


def test_resume_finishes_interrupted_upload(env):
    _interrupted(env)
    r = _run(env, "--resume")
    assert r.returncode == 0, r.stdout + r.stderr
    calls = _calls(env)
    syncs = [c for c in calls if c.startswith("s3 sync")]
    assert len(syncs) == 5 and all("--size-only" in c for c in syncs)
    assert not [c for c in calls if c.startswith("s3 cp --recursive")]
    assert _remote(env) == _expected_remote()
    assert "upload:" not in "".join(l for l in r.stdout.splitlines() if ".nt.gz" in l)
    assert "ok: %d files" % len(FILES) in r.stdout
    assert _ctype(env, "releases/%s/ro-crate-metadata.json" % RID) == "application/json"


def test_resume_skips_tarball_of_same_size(env):
    _interrupted(env)
    tb = env["remote"] / "releases" / (RID + ".tar.gz")
    tb.write_bytes(TARBALL)
    r = _run(env, "--resume")
    assert r.returncode == 0, r.stdout + r.stderr
    assert not [c for c in _calls(env) if c.startswith("s3 cp") and ".tar.gz" in c]
    assert "tarball: %d bytes" % len(TARBALL) in r.stdout


def test_resume_reuploads_tarball_of_other_size(env):
    _interrupted(env)
    tb = env["remote"] / "releases" / (RID + ".tar.gz")
    tb.write_bytes(b"short")
    r = _run(env, "--resume")
    assert r.returncode == 0, r.stdout + r.stderr
    assert [c for c in _calls(env) if c.startswith("s3 cp") and ".tar.gz" in c]
    assert tb.read_bytes() == TARBALL


def test_resume_and_dryrun_combine(env):
    _interrupted(env)
    before = _remote(env)
    for flags in (("--resume", "--dryrun"), ("--dryrun", "--resume")):
        r = _run(env, *flags)
        assert r.returncode == 0, r.stdout + r.stderr
        assert _remote(env) == before
        assert "dry run: skipping" in r.stdout
    syncs = [c for c in _calls(env) if c.startswith("s3 sync")]
    assert syncs and all("--dryrun" in c and "--size-only" in c for c in syncs)


def test_stray_tarball_refused(env):
    (env["out"] / (RID + ".index-entry.json")).write_text(
        json.dumps({"release_id": RID, "tarball": None}))
    r = _run(env)
    assert r.returncode == 1
    assert "%s.tar.gz exists, but" % RID in r.stderr
    assert _calls(env) == []


def test_no_tarball_upload(env):
    (env["out"] / (RID + ".tar.gz")).unlink()
    (env["out"] / (RID + ".index-entry.json")).write_text(
        json.dumps({"release_id": RID, "tarball": None}))
    r = _run(env)
    assert r.returncode == 0, r.stdout + r.stderr
    assert not [c for c in _calls(env) if ".tar.gz" in c]
    assert "releases/%s.tar.gz" % RID not in _remote(env)


def test_remote_tarball_size_checked(env):
    r = _run(env, STUB_TRUNCATE="1")
    assert r.returncode == 1
    assert "s3://testbucket/releases/%s.tar.gz has %d bytes, tarball_bytes is %d" % (
        RID, len(TARBALL) - 1, len(TARBALL)) in r.stderr


def test_listing_error_is_fatal(env):
    r = _run(env, STUB_LS_FAIL="1")
    assert r.returncode == 1
    assert "could not list" in r.stderr
    assert not [c for c in _calls(env) if c.startswith(("s3 cp", "s3 sync"))]


def test_usage(env):
    r = subprocess.run(["bash", str(SCRIPT), "--bogus", "a", "b"], stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, universal_newlines=True, env=env["env"])
    assert r.returncode == 2
    assert "usage:" in r.stderr
