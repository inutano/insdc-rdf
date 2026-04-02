#!/bin/bash
#SBATCH -p kumamoto-c768
#SBATCH -A kumamoto-group
#SBATCH -t 1-00:00:00
#SBATCH --mem-per-cpu 4g
#SBATCH -c 4
#SBATCH -J insdc-rdf-full
#SBATCH -o /lustre10/home/inutano-chiba/insdc-rdf/logs/full_test_%j.log
#SBATCH -e /lustre10/home/inutano-chiba/insdc-rdf/logs/full_test_%j.err

set -euo pipefail

BASE=/lustre10/home/inutano-chiba/insdc-rdf
BIN=$BASE/insdc-rdf
DATA=$BASE/data
OUT=$BASE/output

mkdir -p "$DATA" "$OUT/biosample" "$OUT/sra" "$OUT/bioproject" "$BASE/logs"

echo "=== INSDC-RDF Full Test ==="
echo "Date: $(date -u)"
echo "Node: $(hostname)"
echo "Data dir: $DATA"
echo "Output dir: $OUT"

# Step 1: Download source files
echo "--- Step 1: Download ---"

echo "Downloading biosample_set.xml.gz..."
aria2c -x 16 -s 16 -c -d "$DATA" -o biosample_set.xml.gz \
  "https://ftp.ncbi.nlm.nih.gov/biosample/biosample_set.xml.gz" 2>&1 | tail -3

echo "Downloading SRA_Accessions.tab..."
aria2c -x 16 -s 16 -c -d "$DATA" -o SRA_Accessions.tab \
  "https://ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/SRA_Accessions.tab" 2>&1 | tail -3

echo "Downloading bioproject.xml..."
aria2c -x 16 -s 16 -c -d "$DATA" -o bioproject.xml \
  "https://ftp.ncbi.nlm.nih.gov/bioproject/bioproject.xml" 2>&1 | tail -3

echo "Downloads complete:"
ls -lh "$DATA"

# Step 2: Convert BioSample
echo "--- Step 2: BioSample ---"
time $BIN convert --source biosample \
  --input "$DATA/biosample_set.xml.gz" \
  --output-dir "$OUT/biosample" \
  --chunk-size 100000

# Step 3: Convert SRA
echo "--- Step 3: SRA ---"
time $BIN convert --source sra \
  --input "$DATA/SRA_Accessions.tab" \
  --output-dir "$OUT/sra" \
  --chunk-size 1000000

# Step 4: Convert BioProject (if supported)
echo "--- Step 4: BioProject ---"
if $BIN convert --source bioproject --input /dev/null --output-dir /dev/null 2>&1 | grep -q "not yet implemented"; then
  echo "BioProject converter not yet implemented, skipping"
else
  time $BIN convert --source bioproject \
    --input "$DATA/bioproject.xml" \
    --output-dir "$OUT/bioproject" \
    --chunk-size 100000
fi

# Step 5: Summary
echo "--- Summary ---"
echo "BioSample:"
cat "$OUT/biosample/manifest.json" 2>/dev/null || echo "  (no manifest)"
echo ""
echo "SRA:"
cat "$OUT/sra/manifest.json" 2>/dev/null || echo "  (no manifest)"
echo ""
echo "BioProject:"
cat "$OUT/bioproject/manifest.json" 2>/dev/null || echo "  (no manifest)"
echo ""
echo "Disk usage:"
du -sh "$OUT/biosample" "$OUT/sra" "$OUT/bioproject" 2>/dev/null
echo ""
echo "Total:"
du -sh "$OUT"
echo ""
echo "Done: $(date -u)"
