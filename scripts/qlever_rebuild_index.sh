#!/bin/bash
# Build a QLever index for insdc-rdf from directories of N-Triples, then start a server.
#
# Usage: qlever_rebuild_index.sh <index-dir> <port> <nt-dir>...
#        qlever_rebuild_index.sh --list-inputs <nt-dir>...
#   index-dir  directory for the index (created; must not hold an index already)
#   port       port for qlever-server
#   nt-dir     one or more directories of *.nt and/or *.nt.gz files, mounted read-only.
#              Each directory's files are read in byte order, directories in the order given.
#   --list-inputs  print the files the build would read, in order, and exit
# Env: QLEVER_NAME (default insdc-rdf), QLEVER_CONTAINER (default qlever-insdc-<port>),
#      QLEVER_IMAGE (default docker.io/adfreiburg/qlever),
#      TRIPLES_EXPECTED (optional: fail unless the index log's last "Triples parsed" equals it)
#
# The build fails if any input file cannot be read in full, so a truncated input never
# yields an index silently. Server flags (see the endpoint notes): --entrypoint bash with
# -u uid:gid, and -s is the query timeout (-t would enable the text index).
set -euo pipefail

usage() { sed -n '2,17p' "$0"; exit 1; }

# Print the input files of one directory (*.nt and *.nt.gz, regular files only) in byte order.
list_inputs() {
  local d=$1 f
  local -a found=()
  for f in "$d"/*.nt "$d"/*.nt.gz; do
    if [ -f "$f" ]; then found+=("$f"); fi
  done
  if [ ${#found[@]} -eq 0 ]; then
    echo "no .nt or .nt.gz files in $d" >&2; return 1
  fi
  printf '%s\n' "${found[@]}" | LC_ALL=C sort
}

main() {
  if [ "${1:-}" = "--list-inputs" ]; then
    shift; [ $# -ge 1 ] || usage
    local d
    for d in "$@"; do list_inputs "$d"; done
    return 0
  fi
  [ $# -ge 3 ] || usage

  local INDEX_DIR PORT NAME CONTAINER IMAGE
  INDEX_DIR=$(realpath -m "$1"); PORT=$2; shift 2
  NAME=${QLEVER_NAME:-insdc-rdf}
  CONTAINER=${QLEVER_CONTAINER:-qlever-insdc-$PORT}
  IMAGE=${QLEVER_IMAGE:-docker.io/adfreiburg/qlever}

  mkdir -p "$INDEX_DIR"
  if compgen -G "$INDEX_DIR/$NAME.index.*" > /dev/null; then
    echo "$INDEX_DIR already holds an index named $NAME" >&2; exit 1
  fi
  echo '{"num-triples-per-batch": 1000000}' > "$INDEX_DIR/$NAME.settings.json"

  # Container paths of every input file, in order; one read-only mount per directory.
  local -a mounts=() files=() found=()
  local d listed f i=0
  for d in "$@"; do
    listed=$(list_inputs "$d")
    mapfile -t found <<< "$listed"
    mounts+=(-v "$(realpath "$d"):/nt$i:ro")
    for f in "${found[@]}"; do files+=("/nt$i/${f##*/}"); done
    i=$((i + 1))
  done

  # zcat -f passes plain .nt through. If it cannot read a file, the marker records it:
  # qlever-index would otherwise index the truncated stream and exit 0. The file list goes
  # into one bash -c argument (at most 128 KiB, about 5,000 chunk files).
  local quoted marker="$INDEX_DIR/.input-failed"
  quoted=$(printf ' %q' "${files[@]}")
  rm -f "$marker"
  echo "$(date -u +%FT%TZ) index build start: $INDEX_DIR <- $* (${#files[@]} files)"
  docker run --rm -u "$(id -u):$(id -g)" -v "$INDEX_DIR:/index" "${mounts[@]}" \
    -w /index --init --entrypoint bash "$IMAGE" -c \
    "qlever-index -i $NAME -s $NAME.settings.json --vocabulary-type on-disk-compressed \
       -f <(zcat -f --$quoted || : > /index/.input-failed) \
       -g - -F nt -p false --stxxl-memory 10G > $NAME.index-log.txt 2>&1; \
     rc=\$?; wait \$! 2>/dev/null || true; exit \$rc"
  echo "$(date -u +%FT%TZ) index build done"

  if [ -e "$marker" ]; then
    echo "index build FAILED: could not read every input file (see $marker); the index in $INDEX_DIR is incomplete" >&2
    exit 1
  fi
  if [ -n "${TRIPLES_EXPECTED:-}" ]; then
    local parsed
    parsed=$(tr '\r' '\n' < "$INDEX_DIR/$NAME.index-log.txt" \
      | grep -o 'Triples parsed: [0-9,]*' | tail -n 1 | tr -dc '0-9' || true)
    if [ "$parsed" != "$TRIPLES_EXPECTED" ]; then
      echo "index build FAILED: parsed ${parsed:-no} triples, expected $TRIPLES_EXPECTED (see $INDEX_DIR/$NAME.index-log.txt)" >&2
      exit 1
    fi
  fi

  docker rm -f "$CONTAINER" 2>/dev/null || true
  docker run -d --name "$CONTAINER" --restart unless-stopped -u "$(id -u):$(id -g)" \
    -p "$PORT:$PORT" -v "$INDEX_DIR:/index" -w /index --entrypoint bash "$IMAGE" -c \
    "qlever-server -i $NAME -p $PORT -m 20G -s 300s > $NAME.server-log.txt 2>&1"
  echo "QLever server $CONTAINER started on port $PORT"
}

# The whole script is parsed before main runs, so editing this file during a long build
# does not change the running build.
main "$@"; exit
