#!/bin/bash
#SBATCH -p kumamoto-c768
#SBATCH -A kumamoto-group
#SBATCH -t 1-00:00:00
#SBATCH --mem-per-cpu 4g
#SBATCH -c 4
#SBATCH -J insdc-conv
#SBATCH -o /lustre10/home/inutano-chiba/insdc-rdf/logs/convert_%j.log
#SBATCH -e /lustre10/home/inutano-chiba/insdc-rdf/logs/convert_%j.err

set -euo pipefail

BASE=/lustre10/home/inutano-chiba/insdc-rdf
BIN=$BASE/insdc-rdf
DATA=$BASE/data
OUT=$BASE/output

mkdir -p "$OUT/biosample" "$OUT/sra" "$OUT/bioproject"

echo "=== INSDC-RDF Convert ==="
echo "Date: $(date -u)"
echo "Node: $(hostname)"

echo "--- BioSample ---"
time $BIN convert --source biosample \
  --input "$DATA/biosample_set.xml.gz" \
  --output-dir "$OUT/biosample" \
  --chunk-size 100000

echo "--- SRA ---"
time $BIN convert --source sra \
  --input "$DATA/SRA_Accessions.tab" \
  --output-dir "$OUT/sra" \
  --chunk-size 1000000

echo "--- BioProject ---"
time $BIN convert --source bioproject \
  --input "$DATA/bioproject.xml" \
  --output-dir "$OUT/bioproject" \
  --chunk-size 100000

echo "--- Summary ---"
for src in biosample sra bioproject; do
  echo "$src:"
  cat "$OUT/$src/manifest.json" 2>/dev/null || echo "  (no manifest)"
  echo ""
done

echo "Disk usage:"
du -sh "$OUT/biosample" "$OUT/sra" "$OUT/bioproject"
echo "Total:"
du -sh "$OUT"
echo "Done: $(date -u)"
