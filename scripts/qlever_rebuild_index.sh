#!/bin/bash
# Build a QLever index for insdc-rdf from directories of N-Triples, then start a server.
#
# Usage: qlever_rebuild_index.sh <index-dir> <port> <nt-dir>...
#   index-dir  directory for the index (created; must not hold an index already)
#   port       port for qlever-server
#   nt-dir     one or more directories of *.nt files, mounted read-only
# Env: QLEVER_NAME (default insdc-rdf), QLEVER_CONTAINER (default qlever-insdc-<port>),
#      QLEVER_IMAGE (default docker.io/adfreiburg/qlever)
#
# Server flags (see the endpoint notes): --entrypoint bash with -u uid:gid, and -s is the
# query timeout (-t would enable the text index).
set -euo pipefail

if [ $# -lt 3 ]; then
  sed -n '2,12p' "$0"; exit 1
fi
INDEX_DIR=$(realpath -m "$1"); PORT=$2; shift 2
NAME=${QLEVER_NAME:-insdc-rdf}
CONTAINER=${QLEVER_CONTAINER:-qlever-insdc-$PORT}
IMAGE=${QLEVER_IMAGE:-docker.io/adfreiburg/qlever}

mkdir -p "$INDEX_DIR"
if compgen -G "$INDEX_DIR/$NAME.index.*" > /dev/null; then
  echo "$INDEX_DIR already holds an index named $NAME" >&2; exit 1
fi
echo '{"num-triples-per-batch": 1000000}' > "$INDEX_DIR/$NAME.settings.json"

mounts=(); files=(); i=0
for d in "$@"; do
  compgen -G "$d/*.nt" > /dev/null || { echo "no .nt files in $d" >&2; exit 1; }
  mounts+=(-v "$(realpath "$d"):/nt$i:ro"); files+=("/nt$i/*.nt"); i=$((i + 1))
done

echo "$(date -u +%FT%TZ) index build start: $INDEX_DIR <- $*"
docker run --rm -u "$(id -u):$(id -g)" -v "$INDEX_DIR:/index" "${mounts[@]}" \
  -w /index --init --entrypoint bash "$IMAGE" -c \
  "qlever-index -i $NAME -s $NAME.settings.json --vocabulary-type on-disk-compressed \
     -f <(cat ${files[*]}) -g - -F nt -p false --stxxl-memory 10G > $NAME.index-log.txt 2>&1"
echo "$(date -u +%FT%TZ) index build done"

docker rm -f "$CONTAINER" 2>/dev/null || true
docker run -d --name "$CONTAINER" --restart unless-stopped -u "$(id -u):$(id -g)" \
  -p "$PORT:$PORT" -v "$INDEX_DIR:/index" -w /index --entrypoint bash "$IMAGE" -c \
  "qlever-server -i $NAME -p $PORT -m 20G -s 300s > $NAME.server-log.txt 2>&1"
echo "QLever server $CONTAINER started on port $PORT"
