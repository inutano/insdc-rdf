#!/bin/bash
set -euo pipefail

STORE=/data2/oxigraph-store
mkdir -p "$STORE"

echo "=== Oxigraph Bulk Load ==="
echo "Date: $(date -u)"

TOTAL_START=$(date +%s)

for source in bioproject biosample sra; do
  case "$source" in
    biosample) NT_DIR=/data2/biosample-rdf/output/nt ;;
    sra)       NT_DIR=/data2/sra/output/nt ;;
    bioproject) NT_DIR=/data2/bioproject/output/nt ;;
  esac

  echo ""
  echo "--- Loading $source from $NT_DIR ---"
  COUNT=$(ls "$NT_DIR"/*.nt | wc -l)
  echo "  Files: $COUNT"

  START=$(date +%s)
  for f in "$NT_DIR"/*.nt; do
    docker run --rm \
      -v "$STORE":/data \
      -v "$(dirname "$f")":/input:ro \
      ghcr.io/oxigraph/oxigraph \
      load --location /data --file "/input/$(basename "$f")" 2>&1 | tail -1
  done
  END=$(date +%s)
  echo "  $source loaded in $((END - START)) seconds"
done

TOTAL_END=$(date +%s)
echo ""
echo "=== Total load time: $((TOTAL_END - TOTAL_START)) seconds ==="
echo "Store size:"
du -sh "$STORE"
echo "Date: $(date -u)"
