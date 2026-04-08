# SRA Experiment Metadata RDF Conversion

## Goal

Add a new conversion pipeline to insdc-rdf that extracts descriptive metadata from SRA Experiment XML and produces RDF. This complements the existing SRA accession cross-linking (from `SRA_Accessions.tab`) with rich experiment-level metadata: platform, instrument model, library strategy/source/selection/layout, title, and design description.

## Decisions

- **Experiment XML only** — run/sample/study/analysis XML are skipped (covered by existing converters or too sparse)
- **Separate from existing SRA crate** — new `sra-experiment` crate with its own CLI subcommand; outputs merge naturally in the triplestore via shared subject IRIs
- **No cross-references** — STUDY_REF and SAMPLE_DESCRIPTOR in experiment XML are ignored; cross-linking is handled by the TSV-based SRA conversion
- **No term normalization** — raw XML values are converted directly to URIs; URI collisions (e.g., `:OTHER` across fields) are accepted and can be addressed later when building the OWL ontology from SPARQL
- **Stream from tar.gz** — no disk extraction; gzip decompress → tar entry iteration → XML parsing in a single pass

## Input

**Source**: `ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/NCBI_SRA_Metadata_YYYYMMDD.tar.gz`

- ~6.3 GB compressed daily snapshot
- Structure: `{SubmissionAccession}/{SubmissionAccession}.experiment.xml`
- Each experiment XML is an `EXPERIMENT_SET` containing one or more `EXPERIMENT` elements
- experiment.xml present in ~93% of submission directories

## Data Model

```rust
pub struct SraExperimentRecord {
    pub accession: String,                          // SRX/ERX/DRX
    pub title: Option<String>,
    pub design_description: Option<String>,
    pub library_name: Option<String>,
    pub library_strategy: Option<String>,           // WGS, RNA-Seq, ChIP-Seq, etc.
    pub library_source: Option<String>,             // GENOMIC, TRANSCRIPTOMIC, etc.
    pub library_selection: Option<String>,           // RANDOM, cDNA, PolyA, etc.
    pub library_layout: Option<LibraryLayout>,
    pub library_construction_protocol: Option<String>,
    pub platform: Option<String>,                   // ILLUMINA, LS454, etc.
    pub instrument_model: Option<String>,           // Illumina NovaSeq 6000, etc.
}

pub enum LibraryLayout {
    Single,
    Paired {
        nominal_length: Option<f64>,
        nominal_sdev: Option<f64>,
    },
}
```

## RDF Output

### Prefixes

```turtle
@prefix id: <http://identifiers.org/insdc.sra/> .
@prefix : <http://ddbj.nig.ac.jp/ontologies/dra/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix dct: <http://purl.org/dc/terms/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
```

### Per-experiment output

```turtle
id:SRX000001 a :Experiment ;
  rdfs:label "SRX000001: Whole genome sequencing of sample X" ;
  dct:identifier "SRX000001" ;
  :title "Whole genome sequencing of sample X" ;
  :designDescription "Randomly fragmented genomic DNA." ;
  :platform [
    a :ILLUMINA ;
    :instrumentModel :Illumina_NovaSeq_6000
  ] ;
  :design [
    a :ExperimentDesign ;
    :libraryName "Sample X WGS" ;
    :libraryStrategy :WGS ;
    :librarySource :GENOMIC ;
    :librarySelection :RANDOM ;
    :libraryConstructionProtocol "DNA was fragmented..." ;
    :libraryLayout [
      a :PAIRED ;
      :nominalLength "300"^^xsd:decimal ;
      :nominalSdev "25.0"^^xsd:decimal
    ]
  ] .
```

**URI construction for controlled values**: strip whitespace, replace spaces with `_`, percent-encode special characters. E.g., `"Illumina NovaSeq 6000"` → `:Illumina_NovaSeq_6000`.

**Optional fields**: omitted from output when absent or empty in the XML. Blank nodes (platform, design, libraryLayout) are omitted entirely if they would have no properties.

## Pipeline Architecture

```
NCBI_SRA_Metadata_*.tar.gz
  → flate2::GzDecoder        (streaming gzip decompress)
    → tar::Archive            (iterate tar entries)
      → filter *.experiment.xml
        → quick-xml reader    (streaming XML parse per entry)
          → Vec<SraExperimentRecord>
            → ChunkWriter     (100,000 records per chunk)
              ├→ TTL chunks
              ├→ JSON-LD chunks
              └→ N-Triples chunks
```

Each tar entry that matches `*.experiment.xml` is read into a byte buffer, then parsed with `quick-xml` in streaming mode. Multiple `EXPERIMENT` elements per file are handled (one file per submission, potentially many experiments per submission).

## Crate Structure

```
crates/sra-experiment/
  Cargo.toml
  src/
    lib.rs
    model.rs                  — SraExperimentRecord, LibraryLayout
    parser.rs                 — tar.gz streaming → experiment records
    chunk.rs                  — ChunkWriter (same pattern as other crates)
    serializer/
      mod.rs
      turtle.rs
      jsonld.rs
      ntriples.rs
```

### Dependencies (new)

- `tar` — for reading tar archive entries in streaming mode
- Existing: `flate2`, `quick-xml`, `serde`, `serde_json`, `anyhow`, `thiserror`

### CLI extension

Add `--source sra-experiment` to the existing CLI in `src/main.rs`.

## Output Structure

Same as other crates:

```
output/sra-experiment/
  ttl/chunk_0000.ttl ... chunk_NNNN.ttl
  jsonld/chunk_0000.jsonld ... chunk_NNNN.jsonld
  nt/chunk_0000.nt ... chunk_NNNN.nt
  manifest.json
  progress.json
  errors.log
```

## Error Handling

- Experiments missing `accession` attribute: logged to `errors.log`, skipped
- Malformed XML in a single experiment file: logged, skip that file, continue with next tar entry
- Corrupt tar entries: logged, skipped
- Progress saved after each chunk for resumability

## Testing

- Unit tests with fixture XML (small experiment XML with various edge cases)
- Include fixtures for: single experiment, multiple experiments per file, PAIRED/SINGLE layout, missing optional fields, special characters in text fields
- Integration test: small tar.gz fixture → validate TTL/NT output
- CI: same `cargo test --workspace` + clippy + fmt as existing crates

## Performance Estimate

- BioSample (streaming XML, 53M records): 55 min at 16.2k rec/s
- The tar layer adds minimal overhead (sequential read, no decompression per entry — the whole stream is gzipped)
- Experiment XML is simpler than BioSample XML (no attributes array), so per-record parsing should be faster
- Bottleneck is likely I/O (reading 6.3 GB compressed) rather than CPU
- Estimate: comparable to or faster than BioSample conversion
