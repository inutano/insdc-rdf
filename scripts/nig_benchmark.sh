#!/bin/bash
#SBATCH -p kumamoto-c768
#SBATCH -A kumamoto-group
#SBATCH -t 1-00:00:00
#SBATCH --mem-per-cpu 4g
#SBATCH -c 4
#SBATCH -J insdc-bench
#SBATCH -o /lustre10/home/inutano-chiba/insdc-rdf/logs/benchmark_%j.log
#SBATCH -e /lustre10/home/inutano-chiba/insdc-rdf/logs/benchmark_%j.err

set -euo pipefail

BASE=/lustre10/home/inutano-chiba/insdc-rdf
BIN=$BASE/insdc-rdf
DATA=$BASE/data
OUT=$BASE/output

echo "=== INSDC-RDF Benchmark (one at a time) ==="
echo "Date: $(date -u)"
echo "Node: $(hostname)"

# --- BioSample ---
echo ""
echo "========== BioSample =========="
mkdir -p "$OUT/biosample"
time $BIN convert --source biosample \
  --input "$DATA/biosample_set.xml.gz" \
  --output-dir "$OUT/biosample" \
  --chunk-size 100000
cat "$OUT/biosample/manifest.json"
du -sh "$OUT/biosample"
echo "Cleaning up BioSample output..."
rm -rf "$OUT/biosample"

# --- SRA ---
echo ""
echo "========== SRA =========="
mkdir -p "$OUT/sra"
time $BIN convert --source sra \
  --input "$DATA/SRA_Accessions.tab" \
  --output-dir "$OUT/sra" \
  --chunk-size 1000000
cat "$OUT/sra/manifest.json"
du -sh "$OUT/sra"
echo "Cleaning up SRA output..."
rm -rf "$OUT/sra"

# --- BioProject ---
echo ""
echo "========== BioProject =========="
mkdir -p "$OUT/bioproject"
time $BIN convert --source bioproject \
  --input "$DATA/bioproject.xml" \
  --output-dir "$OUT/bioproject" \
  --chunk-size 100000
cat "$OUT/bioproject/manifest.json"
du -sh "$OUT/bioproject"
echo "Cleaning up BioProject output..."
rm -rf "$OUT/bioproject"

# --- Done ---
echo ""
echo "========== Benchmark Complete =========="
echo "Date: $(date -u)"
