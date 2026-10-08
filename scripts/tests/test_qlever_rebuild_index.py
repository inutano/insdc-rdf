import gzip
import os
import subprocess
from pathlib import Path

import pytest

REAL_REPO = Path(__file__).resolve().parents[2]
SCRIPT = REAL_REPO / "scripts" / "qlever_rebuild_index.sh"

# A stand-in for `docker`: `run` executes the container's `bash -c` command on the
# host, with the mounted container paths rewritten to the host directories.
DOCKER_STUB = r"""#!/bin/bash
echo "docker $*" >> "$STUB_LOG"
[ "$1" = run ] || exit 0
shift
maps=(); cmd=""; wd=""
while [ $# -gt 0 ]; do
  case "$1" in
    -v) maps+=("$2"); shift 2 ;;
    -w) wd=$2; shift 2 ;;
    -u|-p|--name|--restart|--entrypoint) shift 2 ;;
    -c) cmd=$2; shift 2 ;;
    *) shift ;;
  esac
done
case "$cmd" in qlever-server*) exit 0 ;; esac
for m in "${maps[@]}"; do
  IFS=: read -r src dst _ <<< "$m"
  cmd=${cmd//"$dst/"/"$src/"}
  [ "$dst" = "$wd" ] && wd=$src
done
cd "$wd" && exec bash -c "$cmd"
"""

# A stand-in for `qlever-index`: counts the lines of its -f input and logs them the
# way QLever does.
QLEVER_INDEX_STUB = r"""#!/bin/bash
while [ $# -gt 0 ]; do
  case "$1" in -f) input=$2; shift 2 ;; *) shift ;; esac
done
n=$(wc -l < "$input")
printf 'INFO: Triples parsed: %s [average speed 0.3 M/s]\r' 1
printf 'INFO: Triples parsed: %s [average speed 0.3 M/s]\n' "$(printf "%'d" "$n")"
"""


def _write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(text.encode("utf-8"))


def _write_gz(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    with gzip.open(str(path), "wb") as f:
        f.write(text.encode("utf-8"))


def _run(args, env=None, cwd=None):
    return subprocess.run(
        ["bash", str(SCRIPT)] + [str(a) for a in args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        universal_newlines=True,
        env=env,
        cwd=cwd,
    )


@pytest.fixture
def inputs(tmp_path):
    d1 = tmp_path / "d1"
    _write(d1 / "a.nt", "<http://e/a> <http://e/p> 1 .\n")
    _write_gz(d1 / "b.nt.gz", "<http://e/b> <http://e/p> 2 .\n<http://e/b> <http://e/p> 3 .\n")
    _write(d1 / "c.txt", "not rdf\n")
    (d1 / "sub.nt").mkdir()
    d2 = tmp_path / "d2"
    _write_gz(d2 / "z.nt.gz", "<http://e/z> <http://e/p> 4 .\n")
    _write(d2 / "B.nt", "<http://e/B> <http://e/p> 5 .\n")
    _write(d2 / "m.nt", "<http://e/m> <http://e/p> 6 .\n")
    return d1, d2


@pytest.fixture
def stubs(tmp_path):
    bindir = tmp_path / "bin"
    bindir.mkdir()
    for name, body in (("docker", DOCKER_STUB), ("qlever-index", QLEVER_INDEX_STUB)):
        p = bindir / name
        p.write_text(body)
        p.chmod(0o755)
    env = dict(os.environ)
    env["PATH"] = "%s:%s" % (bindir, env["PATH"])
    env["STUB_LOG"] = str(tmp_path / "docker.log")
    env.pop("TRIPLES_EXPECTED", None)
    return env


def test_list_inputs_reads_nt_and_nt_gz_sorted_per_directory(inputs):
    d1, d2 = inputs
    r = _run(["--list-inputs", d2, d1])
    assert r.returncode == 0, r.stderr
    # Directories in argument order; files in byte order within each; c.txt and
    # the directory sub.nt skipped.
    assert r.stdout.splitlines() == [
        str(d2 / "B.nt"),
        str(d2 / "m.nt"),
        str(d2 / "z.nt.gz"),
        str(d1 / "a.nt"),
        str(d1 / "b.nt.gz"),
    ]


def test_list_inputs_refuses_directory_without_nt(inputs, tmp_path):
    d1, _ = inputs
    empty = tmp_path / "only-txt"
    _write(empty / "c.txt", "x\n")
    r = _run(["--list-inputs", d1, empty])
    assert r.returncode != 0
    assert "no .nt or .nt.gz files in %s" % empty in r.stderr


def test_usage_without_arguments():
    r = _run([])
    assert r.returncode == 1
    assert "Usage:" in r.stdout


def test_build_streams_every_input(inputs, stubs, tmp_path):
    d1, d2 = inputs
    idx = tmp_path / "index"
    r = _run([idx, 7099, d1, d2], env=stubs)
    assert r.returncode == 0, r.stdout + r.stderr
    log = (idx / "insdc-rdf.index-log.txt").read_text()
    assert "Triples parsed: 6 " in log
    assert not (idx / ".input-failed").exists()
    calls = Path(stubs["STUB_LOG"]).read_text()
    assert "zcat -f -- /nt0/a.nt /nt0/b.nt.gz /nt1/B.nt /nt1/m.nt /nt1/z.nt.gz ||" in calls
    assert "-v %s:/nt0:ro" % d1 in calls and "-v %s:/nt1:ro" % d2 in calls
    assert "c.txt" not in calls and "sub.nt" not in calls
    assert "qlever-server" in calls


def test_build_fails_when_an_input_cannot_be_read(inputs, stubs, tmp_path):
    d1, d2 = inputs
    (d2 / "m.nt.gz").write_bytes(b"\x1f\x8b\x08\x00not really gzip")
    idx = tmp_path / "index"
    r = _run([idx, 7099, d1, d2], env=stubs)
    assert r.returncode != 0
    assert (idx / ".input-failed").exists()
    assert "could not read every input file" in r.stderr
    assert "qlever-server" not in Path(stubs["STUB_LOG"]).read_text()


def test_build_checks_triples_expected(inputs, stubs, tmp_path):
    d1, d2 = inputs
    env = dict(stubs, TRIPLES_EXPECTED="7")
    r = _run([tmp_path / "index", 7099, d1, d2], env=env)
    assert r.returncode != 0
    assert "parsed 6 triples, expected 7" in r.stderr
    assert "qlever-server" not in Path(stubs["STUB_LOG"]).read_text()

    env = dict(stubs, TRIPLES_EXPECTED="6")
    r = _run([tmp_path / "index2", 7099, d1, d2], env=env)
    assert r.returncode == 0, r.stdout + r.stderr
    assert "qlever-server" in Path(stubs["STUB_LOG"]).read_text()


def test_stale_input_failed_marker_is_cleared(inputs, stubs, tmp_path):
    d1, d2 = inputs
    idx = tmp_path / "index"
    idx.mkdir()
    (idx / ".input-failed").write_text("")
    r = _run([idx, 7099, d1, d2], env=stubs)
    assert r.returncode == 0, r.stdout + r.stderr
    assert not (idx / ".input-failed").exists()
