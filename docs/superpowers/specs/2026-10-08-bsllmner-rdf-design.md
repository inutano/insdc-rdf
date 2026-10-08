# bsllmner-mk2 annotations as RDF, and the 2026-10 INSDC RDF refresh

## Goal

Load the ontology annotations that bsllmner-mk2 produced for public BioSample records into the same QLever index as the INSDC RDF, using the PropertyValue model of [biosample_jsonld](https://github.com/inutano/biosample_jsonld), and rebuild that index from the latest NCBI dumps.

The scope is producing the data. Problems that only show up when an application queries it (for example, term IRIs that do not resolve) are recorded here as known limitations and solved separately.

## Inputs

### INSDC dumps (already converted)

Converted on 2026-10-08 with `insdc-rdf v0.3.0` (commit c3ba3b0). Inputs are in `/data1/work/insdc-rdf-202610/input/`, outputs in `/data3/insdc-rdf-202610/<source>/`.

| Source | Dump | Records | Skipped | Output |
|---|---|---:|---:|---:|
| BioSample | `biosample_set.xml.gz`, 2026-10-07 | 60,144,760 | 0 | 915 GB |
| SRA | `SRA_Accessions.tab`, 2026-10-07 | 143,187,558 | 0 | 314 GB |
| BioProject | `bioproject.xml`, 2026-10-07 | 1,124,118 | 0 | 1.8 GB |
| SRA Experiment | `NCBI_SRA_Metadata_Full_20260913.tar.gz` | 41,593,302 | 0 | 172 GB |

### bsllmner-mk2 release

Release `2026-06_mistral-small3.1-24b-v2` (RO-Crate, CC-BY-4.0), unpacked at `/data1/work/bsllmner/2026-06_mistral-small3.1-24b-v2/`. All 675 entries of `provenance/checksums.sha256` verify.

What the converter reads:

- `provenance/run_index.tsv`: one row per run (311). Columns used: `run_name`, `dataset`, `model`, `first_start`, `last_end`, `code_commit`, `result_file`.
  - `dataset` is one of `chipatlas_hg38` (7 runs), `chipatlas_mm10` (7), `rnaseq_human_past` (88), `rnaseq_human_5y` (60), `rnaseq_mouse` (149).
  - `code_commit` is `5a5744e`, `9a38288`, or `5a5744e+9a38288` (13 runs resumed across the change).
- `ro-crate-metadata.json`: the root Dataset (`name`, `datePublished`, `license`) and the `SoftwareApplication` entities `#bsllmner2-<short>` (`softwareVersion` = full commit hash, `url` = GitHub commit URL).
- `results/<result_file>`: per entry, only `extract.accession` and `results`. Everything else (`search_results`, `text2term_results`, `select_timings`, `ambiguous_fields`, `extract.extracted`, `raw_output`) is skipped while parsing.

Shape of `results`, from profiling all 311 files:

- `results` maps each of 9 fields (`cell_line`, `cell_type`, `tissue`, `disease`, `drug`, `knockout_gene`, `knockdown_gene`, `overexpressed_gene`, `chip_antigen`) to a list of items. A field that was extracted but not mapped is absent; a field that was not extracted is `[]`. 14 entries have `results = {}`.
- Every item has exactly `{value, term_id, term_uri, label, exact_match, reasoning}`. `value`, `term_id`, `term_uri`, `label` are strings, `exact_match` is a bool, `reasoning` is a string or null.
- 4,189,039 entries over 4,187,708 accessions. 1,331 accessions appear in two datasets (a ChIP-Atlas one and an RNA-Seq one); none appears twice within a dataset.
- 6,749,360 items over 38,737 distinct `term_uri`s. In 2,560 (entry, field) pairs the same `term_uri` occurs more than once (2,889 surplus items), almost all in gene fields and `chip_antigen`.
- Result files are up to 709 MB of JSON.

## Decisions

- **Model**: each mapped term becomes a `schema:PropertyValue` attached to the BioSample record with `schema:additionalProperty`, as in biosample_jsonld. No direct BioSample → term shortcut predicate.
- **Existing attribute nodes are not modified.** bsllmner reads the title and all attributes together, so an extracted value cannot be attributed to one original attribute.
- **Only mapped values.** Extracted values without a term (1,384,054 values) are not converted; they stay available in the RO-Crate.
- **Term IRIs are used exactly as bsllmner wrote them** (`term_uri`). No normalization.
- **Terms carry a label only**: `<term> a schema:DefinedTerm ; rdfs:label "…"`. Ontology files are not part of the generated RDF (see "Ontologies" below).
- **Provenance per run**, not per annotation: each annotation node points to its run; the run points to the release.
- **One graph.** Everything goes into the default graph, like the INSDC sources.
- **Datasets are kept apart.** An accession annotated in two datasets gets two sets of nodes, each pointing to its own run. In 180 of the 1,331 overlapping accessions the two sets differ, and in 178 of those the input records are identical, so the difference is run-to-run LLM variation that the graph should preserve.
- `exact_match` is kept as `biosample_ont:exactMatch`. `reasoning` is dropped (7 patterns: "Exact match on <property>" for the 5 label/synonym properties, "text2term score: N", or null; it remains in the RO-Crate).

## RDF model

Prefixes:

```turtle
@prefix schema: <http://schema.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix prov: <http://www.w3.org/ns/prov#> .
@prefix idorg_biosample: <http://identifiers.org/biosample/> .
@prefix biosample_ont: <http://ddbj.nig.ac.jp/ontologies/biosample/> .
@prefix rel: <https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2/> .
```

### Annotation

```turtle
idorg_biosample:SAMD00270091
  schema:additionalProperty <http://ddbj.nig.ac.jp/biosample/SAMD00270091#bsllmner/2026-06_mistral-small3.1-24b-v2/rnaseq_human_5y/cell_line/CVCL_0027> .

<http://ddbj.nig.ac.jp/biosample/SAMD00270091#bsllmner/2026-06_mistral-small3.1-24b-v2/rnaseq_human_5y/cell_line/CVCL_0027>
  a schema:PropertyValue ;
  schema:name "cell_line" ;
  schema:propertyID "cell_line" ;
  schema:value "HepG2" ;
  schema:valueReference <http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027> ;
  biosample_ont:exactMatch true ;
  prov:wasGeneratedBy <https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2/#run-rnaseq_human_5y_2021-08> .

<http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027>
  a schema:DefinedTerm ;
  rdfs:label "Hep-G2" .
```

Rules:

- **Node IRI**: `http://ddbj.nig.ac.jp/biosample/{accession}#bsllmner/{release_id}/{dataset}/{field}/{term_local}`.
  - `term_local` is `term_id` with `:` replaced by `_` (`CVCL:0027` → `CVCL_0027`, `NCBIGene:5265` → `NCBIGene_5265`), percent-encoded with the same fragment encode set that `Attribute::property_iri` uses.
  - `(accession, dataset)` is unique in the release, so the IRI is unique per (entry, field, term).
- **Merging**: items of one entry and field that share a `term_uri` become one node. It gets one `schema:value` per distinct `value`, and `biosample_ont:exactMatch` is true if any merged item has `exact_match: true`. Expected node count: 6,749,360 − 2,889 = 6,746,471.
- `schema:name` and `schema:propertyID` are both the field name. biosample_jsonld uses `name` for the submitter's attribute name; there is none here, so the field name is used for both.
- **Original attribute nodes have no `schema:propertyID`**, so `?pv schema:propertyID ?f` selects bsllmner nodes only.
- **DefinedTerm**: one `a schema:DefinedTerm` per `term_uri` and one `rdfs:label` per distinct (`term_uri`, `label`) pair, each written once, in the chunk where it first appears. The converter keeps a set of seen pairs in memory (~40k entries).

### Provenance

```turtle
<https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2/#run-rnaseq_human_5y_2021-08>
  a prov:Activity ;
  rdfs:label "rnaseq_human_5y_2021-08" ;
  prov:startedAtTime "2026-05-30T17:20:09+00:00"^^xsd:dateTime ;
  prov:endedAtTime "2026-05-30T22:16:22+00:00"^^xsd:dateTime ;
  prov:wasAssociatedWith <https://github.com/dbcls/bsllmner-mk2/commit/9a3828811f1ab4bac85615e0e1b2efcc6603265f> ;
  biosample_ont:llmModel "mistral-small3.1:24b" ;
  schema:isPartOf rel: .

rel:
  a schema:Dataset ;
  schema:name "Ontology-mapped named entities from ChIP-Atlas and RNA-Seq BioSample records" ;
  schema:version "2026-06_mistral-small3.1-24b-v2" ;
  schema:datePublished "2026-09-26"^^xsd:date ;
  schema:license <https://creativecommons.org/licenses/by/4.0/> ;
  schema:citation <https://doi.org/10.1101/2025.02.17.638570> .
```

Rules:

- **Release IRI**: `https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/{release_id}/`. `release_id` is the basename of the input directory; the converter rejects names outside `[A-Za-z0-9._-]`.
- **Run IRI**: release IRI + `#run-{run_name}`, the RO-Crate's own `#run-…` id resolved against the crate root.
- `prov:startedAtTime` / `prov:endedAtTime` come from `first_start` / `last_end` in `run_index.tsv`, written as they appear.
- **`prov:wasAssociatedWith`**: each short hash in `code_commit` (split on `+`) is resolved through `ro-crate-metadata.json`'s `#bsllmner2-<short>` entity to its `url`. An unresolvable hash is an error.
- `name`, `datePublished` and `license` come from the root Dataset.
  - The root Dataset has no citation, so the DOI of the bsllmner-mk2 publication (cited in the bucket's README and `index.json`) is a constant in the converter.
- **Placement**: provenance triples (1 release + 311 runs, about 2.5k triples) are written at the start of `chunk_0000`.

## Converter

A new workspace crate `crates/bsllmner` (`insdc-rdf-bsllmner`), following the layout of `crates/sra-experiment`:

- `model.rs`: run, release and entry types. Entries deserialize with serde, and unknown fields are ignored so the large candidate lists are never materialized.
- `parser.rs`: reads `run_index.tsv` and `ro-crate-metadata.json`, then yields entries file by file in `run_index.tsv` order, records in file order. One result file is held in memory at a time, which peaks at a few GB for the 709 MB file.
- `serializer/{turtle,jsonld,ntriples}.rs`: one record = one entry, which serializes to its annotation nodes plus any first-seen DefinedTerms.
- `chunk.rs`: same chunked output, `progress.json` and `manifest.json` as the other sources.
  - One record = one entry, so `total_records` = 4,189,039.
  - `source_file` is the crate directory; `source_md5` is the MD5 of `provenance/checksums.sha256`, which fingerprints the whole payload.
- CLI: `insdc-rdf convert --source bsllmner --input <crate dir> --output-dir <dir>`. The `--input` help text becomes "Path to input file or directory".
- At the end it prints entries, annotation nodes, distinct terms and runs to stderr.

### Error handling

- An entry whose accession does not match `^SAM[NDE][A-Z]?[0-9]+$` is skipped and logged to `errors.log`, counted in `records_skipped`.
- An item whose `term_uri` is not an absolute `http(s)://` IRI, or contains a character not allowed in an IRI, is dropped and logged. The entry still counts as processed. Profiling found none.
- Entries with no mapped items (`results = {}` or all lists empty) produce no triples and count as processed.
- A missing or unparsable `run_index.tsv`, `ro-crate-metadata.json` or result file, or an unresolvable commit, aborts the conversion.

### Tests

- A fixture crate under `tests/fixtures/bsllmner/2026-06_test-release/` with a 2-run `run_index.tsv`, a trimmed `ro-crate-metadata.json`, and small result files cut from real entries. It covers:
  - a scalar field,
  - an array field with two values mapping to the same term (merge),
  - one accession in two datasets,
  - an entry with `results = {}`,
  - a field absent from `results`,
  - a run with two commits,
  - a malformed accession.
- Unit tests:
  - node IRI building, including percent-encoding and the `:` → `_` rule,
  - merging,
  - DefinedTerm de-duplication across chunks,
  - commit resolution,
  - release-id validation.
- An end-to-end test on the fixture crate checks the output layout and manifest counts, that the N-Triples pass `validate_ntriples`, and that the JSON-LD is valid JSON.
- The run on the full release must report 4,189,039 records, 0 skipped, 6,746,471 annotation nodes, 38,737 distinct terms and 311 runs.

### Schema documentation

- `config/bsllmner/` gets `model.yaml`, `prefix.yaml`, `metadata.yaml`, `endpoint.yaml`, `description.yaml` and `sparql.yaml`, in the rdf-config conventions of the other sources.
  - It also gets `shape.shex` and `schema.svg` if rdf-config runs here; otherwise the ShEx is written by hand.
- `config/biosample/model.yaml` currently has placeholder `schema:valueReference`, `prov:wasAttributedTo` and `AnnotatedSampleType` parts for the old BioSamplePlus pipeline, which the BioSample converter never emits. Remove them and point to `config/bsllmner/`. Regenerate `config/biosample/shape.shex` and `schema.svg` if rdf-config runs.

## Ontologies

The RO-Crate ships the 10 ontology files that bsllmner resolved terms against (`ontology/*.owl`, RDF/XML, 399 MB).

- They are **not** part of the generated RDF. They are third-party files with their own licenses: CC BY 4.0 except Uberon (CC BY 3.0) and NCBI Gene (public domain). The CL subsets also contain 60 EFO classes (Apache-2.0), which the bucket's `SOURCES.md` does not mention.
- The README recommends loading them alongside the bsllmner RDF, and `scripts/bsllmner_ontology_to_nt.py` converts them to N-Triples with rdflib, unchanged.
- These files are flat term lists: label, synonyms, definition, id. They contain **no `rdfs:subClassOf`**, so loading them adds synonyms and definitions, not hierarchy. The README says so.
- On this workstation they are loaded into the QLever index.

## Known limitations (to be solved separately)

- Cellosaurus (`http://purl.obolibrary.org/obo/Cellosaurus#CVCL_…`) and NCBI Gene (`http://purl.obolibrary.org/obo/NCBIGene_…`) term IRIs are bsllmner's own and do not resolve (both return 404). They match the crate's ontology files but no other dataset.
- No hierarchy queries over the annotations, since the shipped ontologies have none.
- Annotations reflect the BioSample snapshot that bsllmner read (May–June 2026), not the 2026-10 dump.

## QLever index and switch-over

1. Convert bsllmner to `/data3/insdc-rdf-202610/bsllmner/`, and the ontologies to `/data3/insdc-rdf-202610/bsllmner-ontology/nt/`.
2. Build one index in `/data1/work/qlever-insdc-202610/` (NVMe) from the `nt/` of BioProject, BioSample, SRA, SRA Experiment, bsllmner and the ontologies.
   - Settings are the same as the current index (`num-triples-per-batch` 1,000,000, `--stxxl-memory 10G`, `on-disk-compressed` vocabulary, default graph).
   - Check free space before starting; if `/data1` is too tight, build on `/data3` instead.
   - The commands go into `scripts/qlever_rebuild_index.sh`, with paths as variables and the server flags from the fixed `/data2/qlever-insdc/rebuild-index.sh` (`--entrypoint bash`, `-s 300s`).
3. Start a trial server on port 7011 from the new index and check:
   - `?s a ?type` counts: `biosample_ont:BioSampleRecord` = 60,144,760 and `bioproject_ont:BioProjectRecord` = 1,124,118 (their manifests). Each SRA class (`dra_ont:Run`, `Sample`, `Study`, `Submission`, `Analysis`) equals the count of that `Type` in `SRA_Accessions.tab`. `dra_ont:Experiment` is typed by both SRA and SRA Experiment, so it is checked as the number of distinct subjects of either.
   - bsllmner counts: 6,746,471 PropertyValues with `prov:wasGeneratedBy` a run of the release, 38,737 DefinedTerms, 311 runs.
   - The number of annotated accessions with no `biosample_ont:BioSampleRecord` in the new dump is reported, not treated as a failure.
   - Spot check `SAMD00270091`.
4. Ask the user before switching port 7001 to the new index. The old index in `/data2/qlever-insdc/` and the April outputs stay until the user says to delete them.

## README

- Add bsllmner as a fifth source: usage, model example, record and triple counts.
- Add the ontology recommendation, the license and citation note for the release, and the known limitations.
- Update the summary and record-count tables to the 2026-10 data.
- Replace the roadmap item "bsllmner-mk2 — … enriches `OriginalSampleProperty` with `valueReference` and `AnnotatedSampleType`" with a description of what was built.
