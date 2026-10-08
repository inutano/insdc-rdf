#!/usr/bin/env bash
# Upload a packaged RDF release to the public bucket.
# Usage: upload_rdf_release.sh [--dryrun] [--resume] OUT_DIR RELEASE_ID
#   --dryrun  print what would be uploaded; skip the post-upload comparison
#   --resume  finish an interrupted upload: accept objects already under the release
#             prefix, sync each group by size, and upload the tarball only if the remote
#             one is absent or of another size. The final checks still run.
# Env:   BUCKET (default biosampleplus), AWS_PROFILE as usual.
# The catalog files (index.json etc.) are not handled here.
set -euo pipefail

DRYRUN=()
RESUME=""
while [[ "${1:-}" == --* ]]; do
  case "$1" in
    --dryrun) DRYRUN=(--dryrun) ;;
    --resume) RESUME=1 ;;
    *) break ;;
  esac
  shift
done
if [[ $# -ne 2 ]]; then
  echo "usage: $0 [--dryrun] [--resume] OUT_DIR RELEASE_ID" >&2
  exit 2
fi
OUT_DIR=$1
ID=$2
BUCKET=${BUCKET:-biosampleplus}
DIR="$OUT_DIR/$ID"
TARBALL="$OUT_DIR/$ID.tar.gz"
REMOTE_TARBALL="s3://$BUCKET/releases/$ID.tar.gz"
CACHE="public, max-age=86400"

die() { echo "error: $*" >&2; exit 1; }

[[ -f "$DIR/ro-crate-metadata.json" ]] || die "$DIR/ro-crate-metadata.json is missing"

# Prints the `aws s3 ls` listing of $1, or nothing when no key matches. Any other
# outcome (permissions, network, credentials) is a hard error. `aws s3 ls` exits 1
# when nothing matches.
s3_ls() {
  local out err rc errfile
  errfile=$(mktemp)
  rc=0
  out=$(aws s3 ls "$1" 2>"$errfile") || rc=$?
  err=$(cat "$errfile")
  rm -f "$errfile"
  if [[ $rc -eq 0 && -n "$out" ]]; then
    printf '%s\n' "$out"
  elif [[ $rc -eq 1 && -z "$out" && -z "$err" ]]; then
    :
  else
    echo "error: could not list $1 (exit $rc): $err" >&2
    exit 1
  fi
}

# Prints the size of the remote tarball, or nothing when it is absent. `aws s3 ls`
# matches by prefix, so only the line naming exactly $ID.tar.gz counts.
remote_tarball_size() {
  s3_ls "$REMOTE_TARBALL" | awk -v name="$ID.tar.gz" '$4 == name {print $3}'
}

ENTRY="$OUT_DIR/$ID.index-entry.json"
[[ -f "$ENTRY" ]] || die "$ENTRY is missing; packaging did not complete"
tb_bytes=$(python3 -c 'import json,sys; v=json.load(open(sys.argv[1])); print("" if v.get("tarball") is None else v["tarball_bytes"])' "$ENTRY")
if [[ -n "$tb_bytes" ]]; then
  [[ -f "$TARBALL" ]] || die "$TARBALL is missing"
  [[ "$(stat -c %s "$TARBALL")" == "$tb_bytes" ]] || die "$TARBALL size differs from tarball_bytes in $ENTRY"
elif [[ -e "$TARBALL" ]]; then
  die "$TARBALL exists, but $ENTRY has no tarball; remove it or re-package"
fi

# Trailing slash matters: without it, ...v2 would match ...v2_rdf.
listing=$(s3_ls "s3://$BUCKET/releases/$ID/")
if [[ -n "$listing" && -z "$RESUME" ]]; then
  die "s3://$BUCKET/releases/$ID/ already holds objects; releases are immutable (use --resume to finish an interrupted upload)"
fi
upload_tarball=""
if [[ -n "$tb_bytes" ]]; then
  remote_tb=$(remote_tarball_size)
  if [[ -z "$remote_tb" ]]; then
    upload_tarball=1
  elif [[ -z "$RESUME" ]]; then
    die "$REMOTE_TARBALL already exists; releases are immutable"
  elif [[ "$remote_tb" != "$tb_bytes" ]]; then
    echo "resume: $REMOTE_TARBALL has $remote_tb bytes, not $tb_bytes; uploading it again"
    upload_tarball=1
  else
    echo "resume: $REMOTE_TARBALL already has $tb_bytes bytes; not uploading it"
  fi
fi

# Every local file must match one of the upload patterns.
unmatched=$(find "$DIR" -type f \
  ! -name '*.gz' ! -name '*.json' ! -name '*.md' ! -name '*.sha256' \
  ! -name '*.tsv' ! -name '*.shex' ! -name '*.yaml' ! -name '*.svg')
[[ -z "$unmatched" ]] || die "files without an upload content type:"$'\n'"$unmatched"

# One call per content type. --resume syncs instead, which skips the objects already
# uploaded (an S3 object is either complete or absent) and sets the same metadata.
upload() {
  local ctype=$1
  shift
  local includes=() mode=(cp --recursive)
  local p
  for p in "$@"; do includes+=(--include "$p"); done
  if [[ -n "$RESUME" ]]; then mode=(sync --size-only); fi
  aws s3 "${mode[@]}" "$DIR/" "s3://$BUCKET/releases/$ID/" \
    --exclude "*" "${includes[@]}" \
    --content-type "$ctype" --cache-control "$CACHE" --no-progress \
    ${DRYRUN[@]+"${DRYRUN[@]}"}
}

upload "application/gzip" '*.gz'
upload "application/json" '*.json'
upload "text/markdown; charset=utf-8" '*.md'
upload "text/plain; charset=utf-8" '*.sha256' '*.tsv' '*.shex' '*.yaml'
upload "image/svg+xml" '*.svg'

if [[ -n "$upload_tarball" ]]; then
  aws s3 cp "$TARBALL" "$REMOTE_TARBALL" \
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
if [[ -n "$tb_bytes" ]]; then
  remote_tb=$(remote_tarball_size)
  [[ "$remote_tb" == "$tb_bytes" ]] || die "$REMOTE_TARBALL has ${remote_tb:-no} bytes, tarball_bytes is $tb_bytes"
  echo "ok: tarball: $tb_bytes bytes at $REMOTE_TARBALL"
fi
