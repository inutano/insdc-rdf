#!/bin/bash
#SBATCH -p kumamoto-c768
#SBATCH -A kumamoto-group
#SBATCH -t 0-12:00:00
#SBATCH --mem-per-cpu 2g
#SBATCH -c 1
#SBATCH -J insdc-dl
#SBATCH -o /lustre10/home/inutano-chiba/insdc-rdf/logs/download_%j.log
#SBATCH -e /lustre10/home/inutano-chiba/insdc-rdf/logs/download_%j.err

set -euo pipefail
DATA=/lustre10/home/inutano-chiba/insdc-rdf/data
mkdir -p "$DATA"

echo "=== Download INSDC Data ==="
echo "Date: $(date -u)"

echo "--- biosample_set.xml.gz ---"
if [ -f "$DATA/biosample_set.xml.gz" ]; then
  echo "Already exists: $(ls -lh "$DATA/biosample_set.xml.gz")"
else
  wget -c -O "$DATA/biosample_set.xml.gz" "https://ftp.ncbi.nlm.nih.gov/biosample/biosample_set.xml.gz"
fi

echo "--- SRA_Accessions.tab ---"
rm -f "$DATA/SRA_Accessions.tab.aria2"
wget -c -O "$DATA/SRA_Accessions.tab" "https://ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/SRA_Accessions.tab"

echo "--- bioproject.xml ---"
wget -c -O "$DATA/bioproject.xml" "https://ftp.ncbi.nlm.nih.gov/bioproject/bioproject.xml"

echo "--- Done ---"
ls -lh "$DATA"
echo "Date: $(date -u)"
