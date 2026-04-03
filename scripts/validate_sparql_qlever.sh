#!/bin/bash
# Run validation SPARQL queries against QLever endpoint.
# Usage: bash validate_sparql_qlever.sh <endpoint_url>

set -euo pipefail

ENDPOINT="${1:-http://localhost:7001}"

query() {
  local name="$1"
  local sparql="$2"
  echo "--- $name ---"
  local start=$(date +%s%N)
  curl -s "$ENDPOINT" \
    --data-urlencode "query=$sparql" \
    --data-urlencode "action=tsv_export"
  local end=$(date +%s%N)
  local ms=$(( (end - start) / 1000000 ))
  echo ""
  echo "  (${ms} ms)"
  echo ""
}

echo "=== INSDC-RDF Validation Queries (QLever) ==="
echo "Endpoint: $ENDPOINT"
echo "Date: $(date -u)"
echo ""

query "Record counts by type" \
  'SELECT ?type (COUNT(?s) AS ?count) WHERE { ?s a ?type . } GROUP BY ?type ORDER BY DESC(?count)'

query "Total triple count" \
  'SELECT (COUNT(*) AS ?triples) WHERE { ?s ?p ?o }'

query "Predicate distribution" \
  'SELECT ?p (COUNT(*) AS ?count) WHERE { ?s ?p ?o . } GROUP BY ?p ORDER BY DESC(?count)'

query "BioSample spot check (SAMN00000002)" \
  'SELECT ?p ?o WHERE { <http://identifiers.org/biosample/SAMN00000002> ?p ?o . } ORDER BY ?p'

query "SRA spot check (DRR000001)" \
  'SELECT ?p ?o WHERE { <http://identifiers.org/insdc.sra/DRR000001> ?p ?o . } ORDER BY ?p'

query "BioProject spot check (PRJNA3)" \
  'SELECT ?p ?o WHERE { <http://identifiers.org/bioproject/PRJNA3> ?p ?o . } ORDER BY ?p'

query "PropertyValue count" \
  'SELECT (COUNT(*) AS ?count) WHERE { ?pv a <http://schema.org/PropertyValue> . }'

query "Cross-link: SRA records linking to BioSample" \
  'SELECT (COUNT(DISTINCT ?sra) AS ?count) WHERE { ?sra <http://www.w3.org/2000/01/rdf-schema#seeAlso> ?bs . FILTER(STRSTARTS(STR(?bs), "http://identifiers.org/biosample/")) }'

echo "=== Done ==="
