#!/usr/bin/env bash
# Upload a packaged RDF release to the public bucket.
# Usage: upload_rdf_release.sh [--dryrun] OUT_DIR RELEASE_ID
# Env:   BUCKET (default biosampleplus), AWS_PROFILE as usual.
# The catalog files (index.json etc.) are not handled here.
set -euo pipefail

DRYRUN=()
if [[ "${1:-}" == "--dryrun" ]]; then
  DRYRUN=(--dryrun)
  shift
fi
if [[ $# -ne 2 ]]; then
  echo "usage: $0 [--dryrun] OUT_DIR RELEASE_ID" >&2
  exit 2
fi
OUT_DIR=$1
ID=$2
BUCKET=${BUCKET:-biosampleplus}
DIR="$OUT_DIR/$ID"
TARBALL="$OUT_DIR/$ID.tar.gz"
CACHE="public, max-age=86400"

die() { echo "error: $*" >&2; exit 1; }

[[ -f "$DIR/ro-crate-metadata.json" ]] || die "$DIR/ro-crate-metadata.json is missing"

# Prints "exists" or "absent"; any other outcome (permissions, network,
# credentials) is a hard error. `aws s3 ls` exits 1 when nothing matches.
remote_state() {
  local out err rc errfile
  errfile=$(mktemp)
  rc=0
  out=$(aws s3 ls "$1" 2>"$errfile") || rc=$?
  err=$(cat "$errfile")
  rm -f "$errfile"
  if [[ $rc -eq 0 && -n "$out" ]]; then
    echo exists
  elif [[ $rc -eq 1 && -z "$out" && -z "$err" ]]; then
    echo absent
  else
    echo "error: could not list $1 (exit $rc): $err" >&2
    exit 1
  fi
}

ENTRY="$OUT_DIR/$ID.index-entry.json"
[[ -f "$ENTRY" ]] || die "$ENTRY is missing; packaging did not complete"
tb_bytes=$(python3 -c 'import json,sys; v=json.load(open(sys.argv[1])); print("" if v.get("tarball") is None else v["tarball_bytes"])' "$ENTRY")
if [[ -n "$tb_bytes" ]]; then
  [[ -f "$TARBALL" ]] || die "$TARBALL is missing"
  [[ "$(stat -c %s "$TARBALL")" == "$tb_bytes" ]] || die "$TARBALL size differs from tarball_bytes in $ENTRY"
fi

# Trailing slash matters: without it, ...v2 would match ...v2_rdf.
state=$(remote_state "s3://$BUCKET/releases/$ID/")
[[ "$state" == absent ]] || die "s3://$BUCKET/releases/$ID/ already holds objects; releases are immutable"
if [[ -f "$TARBALL" ]]; then
  state=$(remote_state "s3://$BUCKET/releases/$ID.tar.gz")
  [[ "$state" == absent ]] || die "s3://$BUCKET/releases/$ID.tar.gz already exists; releases are immutable"
fi

# Every local file must match one of the upload patterns.
unmatched=$(find "$DIR" -type f \
  ! -name '*.gz' ! -name '*.json' ! -name '*.md' ! -name '*.sha256' \
  ! -name '*.tsv' ! -name '*.shex' ! -name '*.yaml' ! -name '*.svg')
[[ -z "$unmatched" ]] || die "files without an upload content type:"$'\n'"$unmatched"

upload() {
  local ctype=$1
  shift
  local includes=()
  local p
  for p in "$@"; do includes+=(--include "$p"); done
  aws s3 cp --recursive "$DIR/" "s3://$BUCKET/releases/$ID/" \
    --exclude "*" "${includes[@]}" \
    --content-type "$ctype" --cache-control "$CACHE" --no-progress \
    ${DRYRUN[@]+"${DRYRUN[@]}"}
}

upload "application/gzip" '*.gz'
upload "application/json" '*.json'
upload "text/markdown; charset=utf-8" '*.md'
upload "text/plain; charset=utf-8" '*.sha256' '*.tsv' '*.shex' '*.yaml'
upload "image/svg+xml" '*.svg'

if [[ -f "$TARBALL" ]]; then
  aws s3 cp "$TARBALL" "s3://$BUCKET/releases/$ID.tar.gz" \
    --content-type "application/gzip" --cache-control "$CACHE" --no-progress \
    ${DRYRUN[@]+"${DRYRUN[@]}"}
fi

if [[ ${#DRYRUN[@]} -gt 0 ]]; then
  echo "dry run: skipping the post-upload comparison"
  exit 0
fi

local_count=$(find "$DIR" -type f | wc -l)
local_bytes=$(find "$DIR" -type f -printf '%s\n' | awk '{s+=$1} END {print s+0}')
summary=$(aws s3 ls --recursive --summarize "s3://$BUCKET/releases/$ID/")
remote_count=$(printf '%s\n' "$summary" | awk '/Total Objects:/ {print $3}')
remote_bytes=$(printf '%s\n' "$summary" | awk '/Total Size:/ {print $3}')
if [[ "$local_count" != "$remote_count" || "$local_bytes" != "$remote_bytes" ]]; then
  die "mismatch: local $local_count files / $local_bytes bytes, remote $remote_count files / $remote_bytes bytes"
fi
echo "ok: $local_count files, $local_bytes bytes uploaded to s3://$BUCKET/releases/$ID/"
