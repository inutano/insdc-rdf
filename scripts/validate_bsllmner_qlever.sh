#!/bin/bash
# Count checks for bsllmner-mk2 annotations in a QLever endpoint.
# Usage: validate_bsllmner_qlever.sh [endpoint] [release-id]
set -euo pipefail

ENDPOINT="${1:-http://localhost:7001}"
RELEASE="${2:-2026-06_mistral-small3.1-24b-v2}"
REL="https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/$RELEASE/"
PREFIXES='PREFIX schema: <http://schema.org/>
PREFIX prov: <http://www.w3.org/ns/prov#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX oboInOwl: <http://www.geneontology.org/formats/oboInOwl#>
PREFIX biosample_ont: <http://ddbj.nig.ac.jp/ontologies/biosample/>'

query() {
  echo "--- $1 ---"
  curl -s "$ENDPOINT" --data-urlencode "query=$PREFIXES
$2" --data-urlencode "action=tsv_export"
  echo
}

echo "=== bsllmner annotation checks: $ENDPOINT ($RELEASE) ==="
query "Runs (expect 311)" \
  "SELECT (COUNT(?run) AS ?n) WHERE { ?run schema:isPartOf <$REL> }"
query "Annotation nodes (expect 6746471)" \
  "SELECT (COUNT(?pv) AS ?n) WHERE { ?pv prov:wasGeneratedBy ?run . ?run schema:isPartOf <$REL> }"
query "Annotation nodes by field" \
  "SELECT ?field (COUNT(?pv) AS ?n) WHERE { ?pv prov:wasGeneratedBy ?run ; schema:propertyID ?field . ?run schema:isPartOf <$REL> } GROUP BY ?field ORDER BY DESC(?n)"
query "DefinedTerms (expect 38737)" \
  "SELECT (COUNT(?t) AS ?n) WHERE { ?t a schema:DefinedTerm }"
query "Annotated BioSamples" \
  "SELECT (COUNT(DISTINCT ?bs) AS ?n) WHERE { ?bs schema:additionalProperty ?pv . ?pv prov:wasGeneratedBy ?run . ?run schema:isPartOf <$REL> }"
query "Annotated BioSamples missing from the BioSample data (reported, not a failure)" \
  "SELECT (COUNT(DISTINCT ?bs) AS ?n) WHERE { ?bs schema:additionalProperty ?pv . ?pv prov:wasGeneratedBy ?run . ?run schema:isPartOf <$REL> . FILTER NOT EXISTS { ?bs a biosample_ont:BioSampleRecord } }"
query "Ontology join: synonyms reachable from cell_line annotations (expect > 0)" \
  "SELECT (COUNT(*) AS ?n) WHERE { ?pv schema:propertyID \"cell_line\" ; schema:valueReference ?t . ?t oboInOwl:hasRelatedSynonym ?syn }"
query "Spot check SAMD00270091" \
  "SELECT ?field ?value ?term ?label WHERE { <http://identifiers.org/biosample/SAMD00270091> schema:additionalProperty ?pv . ?pv schema:propertyID ?field ; schema:value ?value ; schema:valueReference ?term . OPTIONAL { ?term rdfs:label ?label } } ORDER BY ?field"
echo "=== Done ==="
