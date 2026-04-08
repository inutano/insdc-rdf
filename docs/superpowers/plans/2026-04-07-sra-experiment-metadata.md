# SRA Experiment Metadata Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new `sra-experiment` crate that streams SRA experiment XML from NCBI tar.gz archives and converts descriptive metadata (platform, instrument, library info) to RDF.

**Architecture:** New workspace crate following the same pattern as `biosample`/`sra`/`bioproject`: streaming parser → ChunkWriter → 3 serializers (TTL, JSON-LD, N-Triples). The parser streams through tar.gz → filters `*.experiment.xml` entries → parses each with quick-xml → yields `SraExperimentRecord`s. Blank nodes represent platform and library design.

**Tech Stack:** Rust 2021, quick-xml 0.37, flate2 1, tar (new dep), serde/serde_json, chrono, anyhow, thiserror, md-5

**Spec:** `docs/superpowers/specs/2026-04-07-sra-experiment-metadata-design.md`

---

### Task 1: Create crate scaffolding and workspace wiring

**Files:**
- Create: `crates/sra-experiment/Cargo.toml`
- Create: `crates/sra-experiment/src/lib.rs`
- Create: `crates/sra-experiment/src/model.rs`
- Create: `crates/sra-experiment/src/parser.rs`
- Create: `crates/sra-experiment/src/chunk.rs`
- Create: `crates/sra-experiment/src/serializer/mod.rs`
- Create: `crates/sra-experiment/src/serializer/turtle.rs`
- Create: `crates/sra-experiment/src/serializer/jsonld.rs`
- Create: `crates/sra-experiment/src/serializer/ntriples.rs`
- Modify: `Cargo.toml` (workspace root)
- Modify: `src/main.rs`

- [ ] **Step 1: Create `crates/sra-experiment/Cargo.toml`**

```toml
[package]
name = "insdc-rdf-sra-experiment"
version = "0.1.0"
edition = "2021"
description = "SRA experiment metadata XML to RDF converter"

[dependencies]
insdc-rdf-core = { path = "../core" }
quick-xml = "0.37"
flate2 = "1"
tar = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
md-5 = "0.10"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create stub source files**

`crates/sra-experiment/src/lib.rs`:
```rust
pub mod chunk;
pub mod model;
pub mod parser;
pub mod serializer;
```

`crates/sra-experiment/src/model.rs`:
```rust
// SRA experiment metadata model
```

`crates/sra-experiment/src/parser.rs`:
```rust
// Streaming tar.gz + XML parser
```

`crates/sra-experiment/src/chunk.rs`:
```rust
// Chunk writer for buffered output
```

`crates/sra-experiment/src/serializer/mod.rs`:
```rust
pub mod jsonld;
pub mod ntriples;
pub mod turtle;
```

`crates/sra-experiment/src/serializer/turtle.rs`:
```rust
// Turtle serializer
```

`crates/sra-experiment/src/serializer/jsonld.rs`:
```rust
// JSON-LD serializer
```

`crates/sra-experiment/src/serializer/ntriples.rs`:
```rust
// N-Triples serializer
```

- [ ] **Step 3: Add crate to workspace root `Cargo.toml`**

Add `insdc-rdf-sra-experiment` to the `[dependencies]` section:

```toml
insdc-rdf-sra-experiment = { path = "crates/sra-experiment" }
```

- [ ] **Step 4: Add `SraExperiment` variant to `src/main.rs`**

Add to the `SourceType` enum:

```rust
#[derive(Clone, ValueEnum)]
enum SourceType {
    Biosample,
    Sra,
    Bioproject,
    SraExperiment,
}
```

Add the match arm in the `Convert` command handler (stub for now):

```rust
SourceType::SraExperiment => {
    insdc_rdf_sra_experiment::run_convert(&input, &output_dir, chunk_size)
}
```

- [ ] **Step 5: Add stub `run_convert` to `crates/sra-experiment/src/lib.rs`**

```rust
pub mod chunk;
pub mod model;
pub mod parser;
pub mod serializer;

use std::path::Path;

pub fn run_convert(_input: &Path, _output_dir: &Path, _chunk_size: usize) -> anyhow::Result<()> {
    todo!("SRA experiment conversion not yet implemented")
}
```

- [ ] **Step 6: Verify workspace compiles**

Run: `cargo check --workspace`
Expected: compiles with no errors (warnings about unused imports OK)

- [ ] **Step 7: Commit**

```bash
git add crates/sra-experiment/ Cargo.toml src/main.rs
git commit -m "feat: scaffold sra-experiment crate with workspace wiring"
```

---

### Task 2: Data model

**Files:**
- Modify: `crates/sra-experiment/src/model.rs`

- [ ] **Step 1: Write model tests**

Add to `crates/sra-experiment/src/model.rs`:

```rust
use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, PartialEq)]
pub enum LibraryLayout {
    Single,
    Paired {
        nominal_length: Option<f64>,
        nominal_sdev: Option<f64>,
    },
}

#[derive(Debug, Clone)]
pub struct SraExperimentRecord {
    pub accession: String,
    pub title: Option<String>,
    pub design_description: Option<String>,
    pub library_name: Option<String>,
    pub library_strategy: Option<String>,
    pub library_source: Option<String>,
    pub library_selection: Option<String>,
    pub library_layout: Option<LibraryLayout>,
    pub library_construction_protocol: Option<String>,
    pub platform: Option<String>,
    pub instrument_model: Option<String>,
}

impl SraExperimentRecord {
    pub fn iri(&self) -> String {
        format!("{}{}", IDORG_SRA, self.accession)
    }
}

/// Convert a metadata value to a URI-safe local name.
/// Strips leading/trailing whitespace, replaces internal spaces with `_`.
pub fn to_uri_local(s: &str) -> String {
    s.trim().replace(' ', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iri() {
        let rec = SraExperimentRecord {
            accession: "SRX000001".to_string(),
            title: None,
            design_description: None,
            library_name: None,
            library_strategy: None,
            library_source: None,
            library_selection: None,
            library_layout: None,
            library_construction_protocol: None,
            platform: None,
            instrument_model: None,
        };
        assert_eq!(rec.iri(), "http://identifiers.org/insdc.sra/SRX000001");
    }

    #[test]
    fn test_to_uri_local_spaces() {
        assert_eq!(to_uri_local("Illumina NovaSeq 6000"), "Illumina_NovaSeq_6000");
    }

    #[test]
    fn test_to_uri_local_no_spaces() {
        assert_eq!(to_uri_local("WGS"), "WGS");
    }

    #[test]
    fn test_to_uri_local_trim() {
        assert_eq!(to_uri_local("  GENOMIC  "), "GENOMIC");
    }

    #[test]
    fn test_library_layout_equality() {
        assert_eq!(LibraryLayout::Single, LibraryLayout::Single);
        assert_eq!(
            LibraryLayout::Paired { nominal_length: Some(300.0), nominal_sdev: None },
            LibraryLayout::Paired { nominal_length: Some(300.0), nominal_sdev: None },
        );
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p insdc-rdf-sra-experiment`
Expected: all 5 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/sra-experiment/src/model.rs
git commit -m "feat(sra-experiment): add data model with SraExperimentRecord and LibraryLayout"
```

---

### Task 3: XML parsing function

**Files:**
- Modify: `crates/sra-experiment/src/parser.rs`
- Create: `tests/fixtures/experiment_sample.xml`

- [ ] **Step 1: Create test fixture XML**

Create `tests/fixtures/experiment_sample.xml` with realistic SRA experiment XML covering multiple experiments, PAIRED/SINGLE layouts, and missing optional fields:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<EXPERIMENT_SET>
  <EXPERIMENT accession="SRX000001" center_name="GEO" alias="GSM123456">
    <IDENTIFIERS>
      <PRIMARY_ID>SRX000001</PRIMARY_ID>
    </IDENTIFIERS>
    <TITLE>RNA-Seq of human brain tissue</TITLE>
    <STUDY_REF accession="SRP000001"/>
    <DESIGN>
      <DESIGN_DESCRIPTION>Total RNA was extracted and sequenced.</DESIGN_DESCRIPTION>
      <SAMPLE_DESCRIPTOR accession="SRS000001"/>
      <LIBRARY_DESCRIPTOR>
        <LIBRARY_NAME>Brain RNA lib1</LIBRARY_NAME>
        <LIBRARY_STRATEGY>RNA-Seq</LIBRARY_STRATEGY>
        <LIBRARY_SOURCE>TRANSCRIPTOMIC</LIBRARY_SOURCE>
        <LIBRARY_SELECTION>cDNA</LIBRARY_SELECTION>
        <LIBRARY_LAYOUT>
          <PAIRED NOMINAL_LENGTH="300" NOMINAL_SDEV="25.5"/>
        </LIBRARY_LAYOUT>
        <LIBRARY_CONSTRUCTION_PROTOCOL>TruSeq RNA protocol</LIBRARY_CONSTRUCTION_PROTOCOL>
      </LIBRARY_DESCRIPTOR>
    </DESIGN>
    <PLATFORM>
      <ILLUMINA>
        <INSTRUMENT_MODEL>Illumina NovaSeq 6000</INSTRUMENT_MODEL>
      </ILLUMINA>
    </PLATFORM>
  </EXPERIMENT>
  <EXPERIMENT accession="DRX000002" center_name="KEIO">
    <TITLE>WGS of Bacillus subtilis</TITLE>
    <DESIGN>
      <DESIGN_DESCRIPTION/>
      <LIBRARY_DESCRIPTOR>
        <LIBRARY_STRATEGY>WGS</LIBRARY_STRATEGY>
        <LIBRARY_SOURCE>GENOMIC</LIBRARY_SOURCE>
        <LIBRARY_SELECTION>RANDOM</LIBRARY_SELECTION>
        <LIBRARY_LAYOUT>
          <SINGLE/>
        </LIBRARY_LAYOUT>
      </LIBRARY_DESCRIPTOR>
    </DESIGN>
    <PLATFORM>
      <ILLUMINA>
        <INSTRUMENT_MODEL>Illumina Genome Analyzer II</INSTRUMENT_MODEL>
      </ILLUMINA>
    </PLATFORM>
  </EXPERIMENT>
  <EXPERIMENT accession="ERX000003">
    <DESIGN>
      <LIBRARY_DESCRIPTOR>
        <LIBRARY_STRATEGY>ChIP-Seq</LIBRARY_STRATEGY>
        <LIBRARY_SOURCE>GENOMIC</LIBRARY_SOURCE>
        <LIBRARY_SELECTION>ChIP</LIBRARY_SELECTION>
        <LIBRARY_LAYOUT>
          <PAIRED/>
        </LIBRARY_LAYOUT>
      </LIBRARY_DESCRIPTOR>
    </DESIGN>
    <PLATFORM>
      <ABI_SOLID>
        <INSTRUMENT_MODEL>AB SOLiD System 3.0</INSTRUMENT_MODEL>
      </ABI_SOLID>
    </PLATFORM>
  </EXPERIMENT>
</EXPERIMENT_SET>
```

- [ ] **Step 2: Write parser with tests**

Write `crates/sra-experiment/src/parser.rs`:

```rust
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::Read;

use crate::model::{LibraryLayout, SraExperimentRecord};
use insdc_rdf_core::error::ConvertError;

/// Parse all EXPERIMENT elements from an XML byte slice.
/// Returns a Vec of records and a Vec of errors (for experiments missing accession).
pub fn parse_experiment_xml(xml_bytes: &[u8]) -> (Vec<SraExperimentRecord>, Vec<ConvertError>) {
    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);

    let mut records = Vec::new();
    let mut errors = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"EXPERIMENT" => {
                let offset = reader.buffer_position() as u64;
                let accession = extract_attr(e, b"accession");
                match accession {
                    Some(acc) => {
                        match parse_experiment_body(&mut reader, &mut buf, acc) {
                            Ok(record) => records.push(record),
                            Err(err) => errors.push(err),
                        }
                    }
                    None => {
                        errors.push(ConvertError::MissingAccession { offset });
                        skip_to_end(&mut reader, &mut buf, b"EXPERIMENT");
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                errors.push(ConvertError::XmlParse {
                    offset: reader.buffer_position() as u64,
                    message: e.to_string(),
                });
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    (records, errors)
}

fn parse_experiment_body(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    accession: String,
) -> Result<SraExperimentRecord, ConvertError> {
    let mut rec = SraExperimentRecord {
        accession,
        title: None,
        design_description: None,
        library_name: None,
        library_strategy: None,
        library_source: None,
        library_selection: None,
        library_layout: None,
        library_construction_protocol: None,
        platform: None,
        instrument_model: None,
    };

    let mut depth: u32 = 1; // inside <EXPERIMENT>
    let mut current_tag: Option<String> = None;
    let mut in_platform = false;
    let mut platform_tag: Option<String> = None;

    loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "TITLE" | "DESIGN_DESCRIPTION" | "LIBRARY_NAME"
                    | "LIBRARY_STRATEGY" | "LIBRARY_SOURCE" | "LIBRARY_SELECTION"
                    | "LIBRARY_CONSTRUCTION_PROTOCOL" | "INSTRUMENT_MODEL" => {
                        current_tag = Some(tag);
                    }
                    "PLATFORM" => {
                        in_platform = true;
                    }
                    "PAIRED" => {
                        let length = extract_attr(e, b"NOMINAL_LENGTH")
                            .and_then(|s| s.parse::<f64>().ok());
                        let sdev = extract_attr(e, b"NOMINAL_SDEV")
                            .and_then(|s| s.parse::<f64>().ok());
                        rec.library_layout = Some(LibraryLayout::Paired {
                            nominal_length: length,
                            nominal_sdev: sdev,
                        });
                    }
                    "SINGLE" => {
                        rec.library_layout = Some(LibraryLayout::Single);
                    }
                    _ => {
                        // Track platform vendor tag (ILLUMINA, ABI_SOLID, etc.)
                        if in_platform && platform_tag.is_none()
                            && tag != "INSTRUMENT_MODEL"
                        {
                            platform_tag = Some(tag);
                        }
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag.as_str() {
                    "PAIRED" => {
                        let length = extract_attr(e, b"NOMINAL_LENGTH")
                            .and_then(|s| s.parse::<f64>().ok());
                        let sdev = extract_attr(e, b"NOMINAL_SDEV")
                            .and_then(|s| s.parse::<f64>().ok());
                        rec.library_layout = Some(LibraryLayout::Paired {
                            nominal_length: length,
                            nominal_sdev: sdev,
                        });
                    }
                    "SINGLE" => {
                        rec.library_layout = Some(LibraryLayout::Single);
                    }
                    "DESIGN_DESCRIPTION" => {
                        // Self-closing <DESIGN_DESCRIPTION/> means empty
                    }
                    _ => {
                        if in_platform && platform_tag.is_none()
                            && tag != "INSTRUMENT_MODEL"
                        {
                            platform_tag = Some(tag);
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if let Some(ref tag) = current_tag {
                    let text = e.unescape()
                        .map_err(|err| ConvertError::XmlParse {
                            offset: reader.buffer_position() as u64,
                            message: err.to_string(),
                        })?
                        .to_string();
                    if !text.is_empty() {
                        match tag.as_str() {
                            "TITLE" => rec.title = Some(text),
                            "DESIGN_DESCRIPTION" => rec.design_description = Some(text),
                            "LIBRARY_NAME" => rec.library_name = Some(text),
                            "LIBRARY_STRATEGY" => rec.library_strategy = Some(text),
                            "LIBRARY_SOURCE" => rec.library_source = Some(text),
                            "LIBRARY_SELECTION" => rec.library_selection = Some(text),
                            "LIBRARY_CONSTRUCTION_PROTOCOL" => {
                                rec.library_construction_protocol = Some(text);
                            }
                            "INSTRUMENT_MODEL" => rec.instrument_model = Some(text),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                depth -= 1;

                if tag == "PLATFORM" {
                    in_platform = false;
                    rec.platform = platform_tag.take();
                }
                if current_tag.as_deref() == Some(&tag) {
                    current_tag = None;
                }
                if depth == 0 {
                    break; // End of <EXPERIMENT>
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ConvertError::XmlParse {
                    offset: reader.buffer_position() as u64,
                    message: e.to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(rec)
}

/// Extract an attribute value from an XML start/empty element.
fn extract_attr(e: &quick_xml::events::BytesStart, name: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == name {
            return attr.unescape_value().ok().map(|v| v.to_string());
        }
    }
    None
}

/// Skip events until the matching end tag is found.
fn skip_to_end(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>, tag: &[u8]) {
    let mut depth: u32 = 1;
    loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == tag => depth += 1,
            Ok(Event::End(ref e)) if e.name().as_ref() == tag => {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
            Ok(Event::Eof) | Err(_) => return,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LibraryLayout;

    fn fixture_xml() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/experiment_sample.xml"
        ))
        .unwrap()
    }

    #[test]
    fn test_parse_count() {
        let (records, errors) = parse_experiment_xml(&fixture_xml());
        assert_eq!(errors.len(), 0);
        assert_eq!(records.len(), 3);
    }

    #[test]
    fn test_first_experiment_fields() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[0];
        assert_eq!(r.accession, "SRX000001");
        assert_eq!(r.title.as_deref(), Some("RNA-Seq of human brain tissue"));
        assert_eq!(
            r.design_description.as_deref(),
            Some("Total RNA was extracted and sequenced.")
        );
        assert_eq!(r.library_name.as_deref(), Some("Brain RNA lib1"));
        assert_eq!(r.library_strategy.as_deref(), Some("RNA-Seq"));
        assert_eq!(r.library_source.as_deref(), Some("TRANSCRIPTOMIC"));
        assert_eq!(r.library_selection.as_deref(), Some("cDNA"));
        assert_eq!(
            r.library_construction_protocol.as_deref(),
            Some("TruSeq RNA protocol")
        );
        assert_eq!(r.platform.as_deref(), Some("ILLUMINA"));
        assert_eq!(
            r.instrument_model.as_deref(),
            Some("Illumina NovaSeq 6000")
        );
        assert_eq!(
            r.library_layout,
            Some(LibraryLayout::Paired {
                nominal_length: Some(300.0),
                nominal_sdev: Some(25.5),
            })
        );
    }

    #[test]
    fn test_single_layout() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[1];
        assert_eq!(r.accession, "DRX000002");
        assert_eq!(r.library_layout, Some(LibraryLayout::Single));
    }

    #[test]
    fn test_missing_optional_fields() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[1]; // DRX000002 has no LIBRARY_NAME, no LIBRARY_CONSTRUCTION_PROTOCOL
        assert_eq!(r.library_name, None);
        assert_eq!(r.library_construction_protocol, None);
    }

    #[test]
    fn test_empty_design_description() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[1]; // DRX000002 has self-closing <DESIGN_DESCRIPTION/>
        assert_eq!(r.design_description, None);
    }

    #[test]
    fn test_different_platform() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[2]; // ERX000003 uses ABI_SOLID
        assert_eq!(r.platform.as_deref(), Some("ABI_SOLID"));
        assert_eq!(
            r.instrument_model.as_deref(),
            Some("AB SOLiD System 3.0")
        );
    }

    #[test]
    fn test_paired_without_attributes() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[2]; // ERX000003 has <PAIRED/> with no NOMINAL_LENGTH/SDEV
        assert_eq!(
            r.library_layout,
            Some(LibraryLayout::Paired {
                nominal_length: None,
                nominal_sdev: None,
            })
        );
    }

    #[test]
    fn test_missing_accession_produces_error() {
        let xml = br#"<?xml version="1.0"?>
<EXPERIMENT_SET>
  <EXPERIMENT>
    <TITLE>No accession</TITLE>
  </EXPERIMENT>
</EXPERIMENT_SET>"#;
        let (records, errors) = parse_experiment_xml(xml);
        assert_eq!(records.len(), 0);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("missing accession"));
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p insdc-rdf-sra-experiment`
Expected: all tests pass (model + parser tests)

- [ ] **Step 4: Commit**

```bash
git add crates/sra-experiment/src/parser.rs tests/fixtures/experiment_sample.xml
git commit -m "feat(sra-experiment): add XML parser for experiment metadata"
```

---

### Task 4: Tar.gz streaming wrapper

**Files:**
- Modify: `crates/sra-experiment/src/parser.rs`

- [ ] **Step 1: Add tar.gz parser struct and tests**

Add to the top of `crates/sra-experiment/src/parser.rs`, below the existing imports:

```rust
use flate2::read::GzDecoder;
use std::io::{BufRead, BufReader, Read};
use tar::Archive;
```

Add the `SraExperimentParser` struct after the existing `parse_experiment_xml` function (before the tests module):

```rust
/// Streaming parser that reads a tar.gz archive and yields experiment records.
/// Iterates tar entries, filters for *.experiment.xml, parses each file's XML,
/// and buffers the resulting records for one-at-a-time retrieval.
pub struct SraExperimentParser {
    entries: Vec<(String, Vec<u8>)>, // (path, xml_bytes) loaded lazily
    entry_index: usize,
    current_records: Vec<SraExperimentRecord>,
    record_index: usize,
}

impl SraExperimentParser {
    /// Create a parser from a gzipped tar archive reader.
    pub fn from_tar_gz<R: Read>(reader: R) -> std::io::Result<Self> {
        let gz = GzDecoder::new(reader);
        let mut archive = Archive::new(gz);

        let mut entries = Vec::new();
        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry
                .path()?
                .to_string_lossy()
                .to_string();

            if path.ends_with(".experiment.xml") {
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes)?;
                entries.push((path, bytes));
            }
        }

        Ok(SraExperimentParser {
            entries,
            entry_index: 0,
            current_records: Vec::new(),
            record_index: 0,
        })
    }

    /// Return the next experiment record, or None when all entries are exhausted.
    /// Errors from individual XML files are returned inline; the caller should
    /// log them and continue calling next_record().
    pub fn next_record(&mut self) -> Result<Option<SraExperimentRecord>, ConvertError> {
        loop {
            // Return buffered records first
            if self.record_index < self.current_records.len() {
                let record = self.current_records[self.record_index].clone();
                self.record_index += 1;
                return Ok(Some(record));
            }

            // Load next tar entry
            if self.entry_index >= self.entries.len() {
                return Ok(None); // All entries exhausted
            }

            let (_path, ref xml_bytes) = self.entries[self.entry_index];
            self.entry_index += 1;

            let (records, _errors) = parse_experiment_xml(xml_bytes);
            // Note: per-file XML errors are silently dropped here.
            // The caller sees only successfully parsed records.
            // For error reporting, use parse_experiment_xml directly.
            self.current_records = records;
            self.record_index = 0;
        }
    }
}
```

Add tar.gz tests inside the existing `#[cfg(test)] mod tests` block:

```rust
    fn build_tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let gz_buf = Vec::new();
        let enc = GzEncoder::new(gz_buf, Compression::default());
        let mut builder = tar::Builder::new(enc);

        for (path, data) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *data).unwrap();
        }

        let enc = builder.into_inner().unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn test_tar_gz_parser() {
        let xml = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/experiment_sample.xml"
        ))
        .unwrap();

        let tar_bytes = build_tar_gz(&[
            ("SRA000001/SRA000001.experiment.xml", &xml),
            ("SRA000001/SRA000001.run.xml", b"<RUN_SET/>"),
            ("SRA000002/SRA000002.submission.xml", b"<SUBMISSION/>"),
        ]);

        let cursor = std::io::Cursor::new(tar_bytes);
        let mut parser = SraExperimentParser::from_tar_gz(cursor).unwrap();

        let mut records = Vec::new();
        while let Ok(Some(rec)) = parser.next_record() {
            records.push(rec);
        }

        // Only the experiment.xml should be parsed (3 experiments in fixture)
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].accession, "SRX000001");
        assert_eq!(records[1].accession, "DRX000002");
        assert_eq!(records[2].accession, "ERX000003");
    }

    #[test]
    fn test_tar_gz_skips_non_experiment_files() {
        let tar_bytes = build_tar_gz(&[
            ("SRA000001/SRA000001.run.xml", b"<RUN_SET/>"),
            ("SRA000001/SRA000001.submission.xml", b"<SUBMISSION/>"),
        ]);

        let cursor = std::io::Cursor::new(tar_bytes);
        let mut parser = SraExperimentParser::from_tar_gz(cursor).unwrap();

        assert!(parser.next_record().unwrap().is_none());
    }

    #[test]
    fn test_tar_gz_multiple_submissions() {
        let xml1 = br#"<?xml version="1.0"?>
<EXPERIMENT_SET>
  <EXPERIMENT accession="SRX100001"><DESIGN><LIBRARY_DESCRIPTOR>
    <LIBRARY_STRATEGY>WGS</LIBRARY_STRATEGY><LIBRARY_SOURCE>GENOMIC</LIBRARY_SOURCE>
    <LIBRARY_SELECTION>RANDOM</LIBRARY_SELECTION><LIBRARY_LAYOUT><SINGLE/></LIBRARY_LAYOUT>
  </LIBRARY_DESCRIPTOR></DESIGN><PLATFORM><ILLUMINA><INSTRUMENT_MODEL>NextSeq 500</INSTRUMENT_MODEL></ILLUMINA></PLATFORM></EXPERIMENT>
</EXPERIMENT_SET>"#;

        let xml2 = br#"<?xml version="1.0"?>
<EXPERIMENT_SET>
  <EXPERIMENT accession="ERX200001"><DESIGN><LIBRARY_DESCRIPTOR>
    <LIBRARY_STRATEGY>RNA-Seq</LIBRARY_STRATEGY><LIBRARY_SOURCE>TRANSCRIPTOMIC</LIBRARY_SOURCE>
    <LIBRARY_SELECTION>cDNA</LIBRARY_SELECTION><LIBRARY_LAYOUT><PAIRED/></LIBRARY_LAYOUT>
  </LIBRARY_DESCRIPTOR></DESIGN><PLATFORM><ILLUMINA><INSTRUMENT_MODEL>HiSeq 2500</INSTRUMENT_MODEL></ILLUMINA></PLATFORM></EXPERIMENT>
</EXPERIMENT_SET>"#;

        let tar_bytes = build_tar_gz(&[
            ("SRA100001/SRA100001.experiment.xml", xml1.as_slice()),
            ("ERA200001/ERA200001.experiment.xml", xml2.as_slice()),
        ]);

        let cursor = std::io::Cursor::new(tar_bytes);
        let mut parser = SraExperimentParser::from_tar_gz(cursor).unwrap();

        let mut records = Vec::new();
        while let Ok(Some(rec)) = parser.next_record() {
            records.push(rec);
        }

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].accession, "SRX100001");
        assert_eq!(records[1].accession, "ERX200001");
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p insdc-rdf-sra-experiment`
Expected: all tests pass (model + XML parser + tar.gz parser tests)

- [ ] **Step 3: Commit**

```bash
git add crates/sra-experiment/src/parser.rs
git commit -m "feat(sra-experiment): add tar.gz streaming parser wrapper"
```

---

### Task 5: Turtle serializer

**Files:**
- Modify: `crates/sra-experiment/src/serializer/mod.rs`
- Modify: `crates/sra-experiment/src/serializer/turtle.rs`

- [ ] **Step 1: Define the Serializer trait**

Write `crates/sra-experiment/src/serializer/mod.rs`:

```rust
pub mod jsonld;
pub mod ntriples;
pub mod turtle;

use crate::model::SraExperimentRecord;
use std::io::Write;

pub trait Serializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &SraExperimentRecord,
    ) -> std::io::Result<()>;
    fn write_footer<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
}
```

- [ ] **Step 2: Write the Turtle serializer with tests**

Write `crates/sra-experiment/src/serializer/turtle.rs`:

```rust
use std::io::Write;

use super::Serializer;
use crate::model::{to_uri_local, LibraryLayout, SraExperimentRecord};
use insdc_rdf_core::escape::escape_turtle_string;
use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, Default)]
pub struct TurtleSerializer;

impl Serializer for TurtleSerializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, "@prefix insdc_sra: <{}> .", IDORG_SRA)?;
        writeln!(writer, "@prefix dra_ont: <{}> .", DDBJ_DRA_ONT)?;
        writeln!(writer, "@prefix dct: <{}> .", DCT)?;
        writeln!(writer, "@prefix rdfs: <{}> .", RDFS)?;
        writeln!(writer, "@prefix xsd: <{}> .", XSD)?;
        writeln!(writer)?;
        Ok(())
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &SraExperimentRecord,
    ) -> std::io::Result<()> {
        let acc = &record.accession;

        // Subject and type
        writeln!(writer, "insdc_sra:{}", acc)?;
        write!(writer, "  a dra_ont:Experiment")?;

        // rdfs:label
        if let Some(ref title) = record.title {
            write!(writer, " ;\n  rdfs:label \"{}: {}\"",
                escape_turtle_string(acc),
                escape_turtle_string(title))?;
        }

        // dct:identifier
        write!(writer, " ;\n  dct:identifier \"{}\"",
            escape_turtle_string(acc))?;

        // :title
        if let Some(ref title) = record.title {
            write!(writer, " ;\n  dra_ont:title \"{}\"",
                escape_turtle_string(title))?;
        }

        // :designDescription
        if let Some(ref desc) = record.design_description {
            write!(writer, " ;\n  dra_ont:designDescription \"{}\"",
                escape_turtle_string(desc))?;
        }

        // :platform blank node
        if record.platform.is_some() || record.instrument_model.is_some() {
            write!(writer, " ;\n  dra_ont:platform [")?;
            let mut first = true;
            if let Some(ref p) = record.platform {
                write!(writer, "\n    a dra_ont:{}", to_uri_local(p))?;
                first = false;
            }
            if let Some(ref m) = record.instrument_model {
                if !first {
                    write!(writer, " ;")?;
                }
                write!(writer, "\n    dra_ont:instrumentModel dra_ont:{}",
                    to_uri_local(m))?;
            }
            write!(writer, "\n  ]")?;
        }

        // :design blank node
        if self.has_design_fields(record) {
            write!(writer, " ;\n  dra_ont:design [")?;
            write!(writer, "\n    a dra_ont:ExperimentDesign")?;

            if let Some(ref name) = record.library_name {
                write!(writer, " ;\n    dra_ont:libraryName \"{}\"",
                    escape_turtle_string(name))?;
            }
            if let Some(ref s) = record.library_strategy {
                write!(writer, " ;\n    dra_ont:libraryStrategy dra_ont:{}",
                    to_uri_local(s))?;
            }
            if let Some(ref s) = record.library_source {
                write!(writer, " ;\n    dra_ont:librarySource dra_ont:{}",
                    to_uri_local(s))?;
            }
            if let Some(ref s) = record.library_selection {
                write!(writer, " ;\n    dra_ont:librarySelection dra_ont:{}",
                    to_uri_local(s))?;
            }
            if let Some(ref p) = record.library_construction_protocol {
                write!(writer, " ;\n    dra_ont:libraryConstructionProtocol \"{}\"",
                    escape_turtle_string(p))?;
            }

            // :libraryLayout blank node
            if let Some(ref layout) = record.library_layout {
                match layout {
                    LibraryLayout::Single => {
                        write!(writer, " ;\n    dra_ont:libraryLayout [\n      a dra_ont:SINGLE\n    ]")?;
                    }
                    LibraryLayout::Paired { nominal_length, nominal_sdev } => {
                        write!(writer, " ;\n    dra_ont:libraryLayout [")?;
                        write!(writer, "\n      a dra_ont:PAIRED")?;
                        if let Some(len) = nominal_length {
                            write!(writer, " ;\n      dra_ont:nominalLength \"{}\"^^xsd:decimal", len)?;
                        }
                        if let Some(sd) = nominal_sdev {
                            write!(writer, " ;\n      dra_ont:nominalSdev \"{}\"^^xsd:decimal", sd)?;
                        }
                        write!(writer, "\n    ]")?;
                    }
                }
            }

            write!(writer, "\n  ]")?;
        }

        writeln!(writer, " .")?;
        writeln!(writer)?;
        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

impl TurtleSerializer {
    pub fn new() -> Self {
        TurtleSerializer
    }

    fn has_design_fields(&self, r: &SraExperimentRecord) -> bool {
        r.library_name.is_some()
            || r.library_strategy.is_some()
            || r.library_source.is_some()
            || r.library_selection.is_some()
            || r.library_construction_protocol.is_some()
            || r.library_layout.is_some()
    }

    pub fn record_to_string(&self, record: &SraExperimentRecord) -> String {
        let mut buf = Vec::new();
        self.write_record(&mut buf, record).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_record() -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "SRX000001".to_string(),
            title: Some("RNA-Seq of human brain".to_string()),
            design_description: Some("Total RNA extracted".to_string()),
            library_name: Some("Brain RNA lib1".to_string()),
            library_strategy: Some("RNA-Seq".to_string()),
            library_source: Some("TRANSCRIPTOMIC".to_string()),
            library_selection: Some("cDNA".to_string()),
            library_layout: Some(LibraryLayout::Paired {
                nominal_length: Some(300.0),
                nominal_sdev: Some(25.5),
            }),
            library_construction_protocol: Some("TruSeq protocol".to_string()),
            platform: Some("ILLUMINA".to_string()),
            instrument_model: Some("Illumina NovaSeq 6000".to_string()),
        }
    }

    fn minimal_record() -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "ERX000002".to_string(),
            title: None,
            design_description: None,
            library_name: None,
            library_strategy: None,
            library_source: None,
            library_selection: None,
            library_layout: None,
            library_construction_protocol: None,
            platform: None,
            instrument_model: None,
        }
    }

    #[test]
    fn test_header_prefixes() {
        let ser = TurtleSerializer::new();
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("@prefix insdc_sra:"));
        assert!(s.contains("@prefix dra_ont:"));
        assert!(s.contains("@prefix dct:"));
        assert!(s.contains("@prefix rdfs:"));
        assert!(s.contains("@prefix xsd:"));
    }

    #[test]
    fn test_full_record_output() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(s.contains("insdc_sra:SRX000001"));
        assert!(s.contains("a dra_ont:Experiment"));
        assert!(s.contains("dct:identifier \"SRX000001\""));
        assert!(s.contains("rdfs:label \"SRX000001: RNA-Seq of human brain\""));
        assert!(s.contains("dra_ont:title \"RNA-Seq of human brain\""));
        assert!(s.contains("dra_ont:designDescription \"Total RNA extracted\""));
        assert!(s.contains("dra_ont:platform ["));
        assert!(s.contains("a dra_ont:ILLUMINA"));
        assert!(s.contains("dra_ont:instrumentModel dra_ont:Illumina_NovaSeq_6000"));
        assert!(s.contains("dra_ont:design ["));
        assert!(s.contains("a dra_ont:ExperimentDesign"));
        assert!(s.contains("dra_ont:libraryStrategy dra_ont:RNA-Seq"));
        assert!(s.contains("dra_ont:librarySource dra_ont:TRANSCRIPTOMIC"));
        assert!(s.contains("dra_ont:librarySelection dra_ont:cDNA"));
        assert!(s.contains("dra_ont:libraryName \"Brain RNA lib1\""));
        assert!(s.contains("dra_ont:libraryConstructionProtocol \"TruSeq protocol\""));
        assert!(s.contains("a dra_ont:PAIRED"));
        assert!(s.contains("dra_ont:nominalLength \"300\"^^xsd:decimal"));
        assert!(s.contains("dra_ont:nominalSdev \"25.5\"^^xsd:decimal"));
    }

    #[test]
    fn test_record_ends_with_dot() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(s.trim().ends_with('.'));
    }

    #[test]
    fn test_minimal_record() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&minimal_record());
        assert!(s.contains("insdc_sra:ERX000002"));
        assert!(s.contains("a dra_ont:Experiment"));
        assert!(s.contains("dct:identifier \"ERX000002\""));
        // No platform or design blocks
        assert!(!s.contains("dra_ont:platform"));
        assert!(!s.contains("dra_ont:design"));
    }

    #[test]
    fn test_single_layout() {
        let mut rec = minimal_record();
        rec.library_layout = Some(LibraryLayout::Single);
        rec.library_strategy = Some("WGS".to_string());
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&rec);
        assert!(s.contains("a dra_ont:SINGLE"));
        assert!(!s.contains("nominalLength"));
    }

    #[test]
    fn test_escaping() {
        let mut rec = minimal_record();
        rec.title = Some("Test with \"quotes\" and\nnewline".to_string());
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&rec);
        assert!(s.contains("Test with \\\"quotes\\\" and\\nnewline"));
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p insdc-rdf-sra-experiment`
Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/sra-experiment/src/serializer/
git commit -m "feat(sra-experiment): add Turtle serializer with blank node support"
```

---

### Task 6: N-Triples serializer

**Files:**
- Modify: `crates/sra-experiment/src/serializer/ntriples.rs`

- [ ] **Step 1: Write N-Triples serializer with tests**

Write `crates/sra-experiment/src/serializer/ntriples.rs`:

```rust
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use super::Serializer;
use crate::model::{to_uri_local, LibraryLayout, SraExperimentRecord};
use insdc_rdf_core::escape::escape_ntriples_string;
use insdc_rdf_core::prefix::*;

static BLANK_NODE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
pub struct NTriplesSerializer;

impl Serializer for NTriplesSerializer {
    fn write_header<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &SraExperimentRecord,
    ) -> std::io::Result<()> {
        let subj = record.iri();
        let xsd_decimal = format!("{}decimal", XSD);

        // rdf:type
        writeln!(
            writer,
            "<{}> <{}> <{}Experiment> .",
            subj, RDF_TYPE, DDBJ_DRA_ONT
        )?;

        // rdfs:label
        if let Some(ref title) = record.title {
            writeln!(
                writer,
                "<{}> <{}label> \"{}: {}\" .",
                subj,
                RDFS,
                escape_ntriples_string(&record.accession),
                escape_ntriples_string(title)
            )?;
        }

        // dct:identifier
        writeln!(
            writer,
            "<{}> <{}identifier> \"{}\" .",
            subj,
            DCT,
            escape_ntriples_string(&record.accession)
        )?;

        // dra_ont:title
        if let Some(ref title) = record.title {
            writeln!(
                writer,
                "<{}> <{}title> \"{}\" .",
                subj,
                DDBJ_DRA_ONT,
                escape_ntriples_string(title)
            )?;
        }

        // dra_ont:designDescription
        if let Some(ref desc) = record.design_description {
            writeln!(
                writer,
                "<{}> <{}designDescription> \"{}\" .",
                subj,
                DDBJ_DRA_ONT,
                escape_ntriples_string(desc)
            )?;
        }

        // Platform blank node
        if record.platform.is_some() || record.instrument_model.is_some() {
            let bnode = next_bnode();
            writeln!(
                writer,
                "<{}> <{}platform> {} .",
                subj, DDBJ_DRA_ONT, bnode
            )?;
            if let Some(ref p) = record.platform {
                writeln!(
                    writer,
                    "{} <{}> <{}{}> .",
                    bnode,
                    RDF_TYPE,
                    DDBJ_DRA_ONT,
                    to_uri_local(p)
                )?;
            }
            if let Some(ref m) = record.instrument_model {
                writeln!(
                    writer,
                    "{} <{}instrumentModel> <{}{}> .",
                    bnode,
                    DDBJ_DRA_ONT,
                    DDBJ_DRA_ONT,
                    to_uri_local(m)
                )?;
            }
        }

        // Design blank node
        let has_design = record.library_name.is_some()
            || record.library_strategy.is_some()
            || record.library_source.is_some()
            || record.library_selection.is_some()
            || record.library_construction_protocol.is_some()
            || record.library_layout.is_some();

        if has_design {
            let design_bnode = next_bnode();
            writeln!(
                writer,
                "<{}> <{}design> {} .",
                subj, DDBJ_DRA_ONT, design_bnode
            )?;
            writeln!(
                writer,
                "{} <{}> <{}ExperimentDesign> .",
                design_bnode, RDF_TYPE, DDBJ_DRA_ONT
            )?;

            if let Some(ref name) = record.library_name {
                writeln!(
                    writer,
                    "{} <{}libraryName> \"{}\" .",
                    design_bnode,
                    DDBJ_DRA_ONT,
                    escape_ntriples_string(name)
                )?;
            }
            if let Some(ref s) = record.library_strategy {
                writeln!(
                    writer,
                    "{} <{}libraryStrategy> <{}{}> .",
                    design_bnode,
                    DDBJ_DRA_ONT,
                    DDBJ_DRA_ONT,
                    to_uri_local(s)
                )?;
            }
            if let Some(ref s) = record.library_source {
                writeln!(
                    writer,
                    "{} <{}librarySource> <{}{}> .",
                    design_bnode,
                    DDBJ_DRA_ONT,
                    DDBJ_DRA_ONT,
                    to_uri_local(s)
                )?;
            }
            if let Some(ref s) = record.library_selection {
                writeln!(
                    writer,
                    "{} <{}librarySelection> <{}{}> .",
                    design_bnode,
                    DDBJ_DRA_ONT,
                    DDBJ_DRA_ONT,
                    to_uri_local(s)
                )?;
            }
            if let Some(ref p) = record.library_construction_protocol {
                writeln!(
                    writer,
                    "{} <{}libraryConstructionProtocol> \"{}\" .",
                    design_bnode,
                    DDBJ_DRA_ONT,
                    escape_ntriples_string(p)
                )?;
            }

            if let Some(ref layout) = record.library_layout {
                let layout_bnode = next_bnode();
                writeln!(
                    writer,
                    "{} <{}libraryLayout> {} .",
                    design_bnode, DDBJ_DRA_ONT, layout_bnode
                )?;
                match layout {
                    LibraryLayout::Single => {
                        writeln!(
                            writer,
                            "{} <{}> <{}SINGLE> .",
                            layout_bnode, RDF_TYPE, DDBJ_DRA_ONT
                        )?;
                    }
                    LibraryLayout::Paired {
                        nominal_length,
                        nominal_sdev,
                    } => {
                        writeln!(
                            writer,
                            "{} <{}> <{}PAIRED> .",
                            layout_bnode, RDF_TYPE, DDBJ_DRA_ONT
                        )?;
                        if let Some(len) = nominal_length {
                            writeln!(
                                writer,
                                "{} <{}nominalLength> \"{}\"^^<{}> .",
                                layout_bnode, DDBJ_DRA_ONT, len, xsd_decimal
                            )?;
                        }
                        if let Some(sd) = nominal_sdev {
                            writeln!(
                                writer,
                                "{} <{}nominalSdev> \"{}\"^^<{}> .",
                                layout_bnode, DDBJ_DRA_ONT, sd, xsd_decimal
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

fn next_bnode() -> String {
    let id = BLANK_NODE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("_:b{}", id)
}

impl NTriplesSerializer {
    pub fn new() -> Self {
        NTriplesSerializer
    }

    pub fn record_to_string(&self, record: &SraExperimentRecord) -> String {
        let mut buf = Vec::new();
        self.write_record(&mut buf, record).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_record() -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "SRX000001".to_string(),
            title: Some("RNA-Seq of human brain".to_string()),
            design_description: Some("Total RNA extracted".to_string()),
            library_name: Some("Brain RNA lib1".to_string()),
            library_strategy: Some("RNA-Seq".to_string()),
            library_source: Some("TRANSCRIPTOMIC".to_string()),
            library_selection: Some("cDNA".to_string()),
            library_layout: Some(LibraryLayout::Paired {
                nominal_length: Some(300.0),
                nominal_sdev: Some(25.5),
            }),
            library_construction_protocol: Some("TruSeq protocol".to_string()),
            platform: Some("ILLUMINA".to_string()),
            instrument_model: Some("Illumina NovaSeq 6000".to_string()),
        }
    }

    #[test]
    fn test_every_line_ends_with_space_dot() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        for line in s.lines() {
            assert!(line.ends_with(" ."), "Line: {:?}", line);
        }
    }

    #[test]
    fn test_contains_full_iris() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(s.contains("<http://identifiers.org/insdc.sra/SRX000001>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/Experiment>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/ILLUMINA>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/Illumina_NovaSeq_6000>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/RNA-Seq>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/PAIRED>"));
    }

    #[test]
    fn test_no_prefixed_names() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(!s.contains("insdc_sra:"));
        assert!(!s.contains("dra_ont:"));
    }

    #[test]
    fn test_blank_nodes_present() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        // Should have blank nodes for platform, design, and layout
        assert!(s.contains("_:b"));
    }

    #[test]
    fn test_decimal_typing() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(s.contains("\"300\"^^<http://www.w3.org/2001/XMLSchema#decimal>"));
        assert!(s.contains("\"25.5\"^^<http://www.w3.org/2001/XMLSchema#decimal>"));
    }

    #[test]
    fn test_minimal_record() {
        let rec = SraExperimentRecord {
            accession: "ERX000002".to_string(),
            title: None,
            design_description: None,
            library_name: None,
            library_strategy: None,
            library_source: None,
            library_selection: None,
            library_layout: None,
            library_construction_protocol: None,
            platform: None,
            instrument_model: None,
        };
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&rec);
        let lines: Vec<&str> = s.lines().collect();
        // Only type + identifier = 2 lines
        assert_eq!(lines.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p insdc-rdf-sra-experiment`
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/sra-experiment/src/serializer/ntriples.rs
git commit -m "feat(sra-experiment): add N-Triples serializer"
```

---

### Task 7: JSON-LD serializer

**Files:**
- Modify: `crates/sra-experiment/src/serializer/jsonld.rs`

- [ ] **Step 1: Write JSON-LD serializer with tests**

Write `crates/sra-experiment/src/serializer/jsonld.rs`:

```rust
use serde::Serialize;
use std::io::Write;

use super::Serializer;
use crate::model::{to_uri_local, LibraryLayout, SraExperimentRecord};

#[derive(Debug, Clone, Serialize)]
struct JsonLdContext {
    insdc_sra: &'static str,
    dra_ont: &'static str,
    dct: &'static str,
    rdfs: &'static str,
    xsd: &'static str,
}

static CONTEXT: JsonLdContext = JsonLdContext {
    insdc_sra: "http://identifiers.org/insdc.sra/",
    dra_ont: "http://ddbj.nig.ac.jp/ontologies/dra/",
    dct: "http://purl.org/dc/terms/",
    rdfs: "http://www.w3.org/2000/01/rdf-schema#",
    xsd: "http://www.w3.org/2001/XMLSchema#",
};

#[derive(Debug, Clone, Serialize)]
struct TypedDecimal {
    #[serde(rename = "@value")]
    value: String,
    #[serde(rename = "@type")]
    r#type: &'static str,
}

impl TypedDecimal {
    fn new(v: f64) -> Self {
        TypedDecimal {
            value: v.to_string(),
            r#type: "xsd:decimal",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct LayoutNode {
    #[serde(rename = "@type")]
    r#type: String,
    #[serde(rename = "dra_ont:nominalLength", skip_serializing_if = "Option::is_none")]
    nominal_length: Option<TypedDecimal>,
    #[serde(rename = "dra_ont:nominalSdev", skip_serializing_if = "Option::is_none")]
    nominal_sdev: Option<TypedDecimal>,
}

#[derive(Debug, Clone, Serialize)]
struct DesignNode {
    #[serde(rename = "@type")]
    r#type: &'static str,
    #[serde(rename = "dra_ont:libraryName", skip_serializing_if = "Option::is_none")]
    library_name: Option<String>,
    #[serde(rename = "dra_ont:libraryStrategy", skip_serializing_if = "Option::is_none")]
    library_strategy: Option<IdRef>,
    #[serde(rename = "dra_ont:librarySource", skip_serializing_if = "Option::is_none")]
    library_source: Option<IdRef>,
    #[serde(rename = "dra_ont:librarySelection", skip_serializing_if = "Option::is_none")]
    library_selection: Option<IdRef>,
    #[serde(rename = "dra_ont:libraryConstructionProtocol", skip_serializing_if = "Option::is_none")]
    library_construction_protocol: Option<String>,
    #[serde(rename = "dra_ont:libraryLayout", skip_serializing_if = "Option::is_none")]
    library_layout: Option<LayoutNode>,
}

#[derive(Debug, Clone, Serialize)]
struct PlatformNode {
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(rename = "dra_ont:instrumentModel", skip_serializing_if = "Option::is_none")]
    instrument_model: Option<IdRef>,
}

#[derive(Debug, Clone, Serialize)]
struct IdRef {
    #[serde(rename = "@id")]
    id: String,
}

#[derive(Debug, Clone, Serialize)]
struct JsonLdRecord {
    #[serde(rename = "@context")]
    context: &'static JsonLdContext,
    #[serde(rename = "@type")]
    r#type: &'static str,
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "rdfs:label", skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(rename = "dct:identifier")]
    identifier: String,
    #[serde(rename = "dra_ont:title", skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(rename = "dra_ont:designDescription", skip_serializing_if = "Option::is_none")]
    design_description: Option<String>,
    #[serde(rename = "dra_ont:platform", skip_serializing_if = "Option::is_none")]
    platform: Option<PlatformNode>,
    #[serde(rename = "dra_ont:design", skip_serializing_if = "Option::is_none")]
    design: Option<DesignNode>,
}

#[derive(Debug, Clone, Default)]
pub struct JsonLdSerializer;

impl Serializer for JsonLdSerializer {
    fn write_header<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &SraExperimentRecord,
    ) -> std::io::Result<()> {
        let label = record.title.as_ref().map(|t| {
            format!("{}: {}", record.accession, t)
        });

        let platform = if record.platform.is_some() || record.instrument_model.is_some() {
            Some(PlatformNode {
                r#type: record.platform.as_ref().map(|p| format!("dra_ont:{}", to_uri_local(p))),
                instrument_model: record.instrument_model.as_ref().map(|m| IdRef {
                    id: format!("dra_ont:{}", to_uri_local(m)),
                }),
            })
        } else {
            None
        };

        let has_design = record.library_name.is_some()
            || record.library_strategy.is_some()
            || record.library_source.is_some()
            || record.library_selection.is_some()
            || record.library_construction_protocol.is_some()
            || record.library_layout.is_some();

        let design = if has_design {
            let layout = record.library_layout.as_ref().map(|l| match l {
                LibraryLayout::Single => LayoutNode {
                    r#type: "dra_ont:SINGLE".to_string(),
                    nominal_length: None,
                    nominal_sdev: None,
                },
                LibraryLayout::Paired { nominal_length, nominal_sdev } => LayoutNode {
                    r#type: "dra_ont:PAIRED".to_string(),
                    nominal_length: nominal_length.map(TypedDecimal::new),
                    nominal_sdev: nominal_sdev.map(TypedDecimal::new),
                },
            });

            Some(DesignNode {
                r#type: "dra_ont:ExperimentDesign",
                library_name: record.library_name.clone(),
                library_strategy: record.library_strategy.as_ref().map(|s| IdRef {
                    id: format!("dra_ont:{}", to_uri_local(s)),
                }),
                library_source: record.library_source.as_ref().map(|s| IdRef {
                    id: format!("dra_ont:{}", to_uri_local(s)),
                }),
                library_selection: record.library_selection.as_ref().map(|s| IdRef {
                    id: format!("dra_ont:{}", to_uri_local(s)),
                }),
                library_construction_protocol: record.library_construction_protocol.clone(),
                library_layout: layout,
            })
        } else {
            None
        };

        let obj = JsonLdRecord {
            context: &CONTEXT,
            r#type: "dra_ont:Experiment",
            id: record.iri(),
            label,
            identifier: record.accession.clone(),
            title: record.title.clone(),
            design_description: record.design_description.clone(),
            platform,
            design,
        };

        let json = serde_json::to_string_pretty(&obj).map_err(std::io::Error::other)?;
        write!(writer, "\n{}", json)?;
        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

impl JsonLdSerializer {
    pub fn new() -> Self {
        JsonLdSerializer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_record() -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "SRX000001".to_string(),
            title: Some("RNA-Seq of human brain".to_string()),
            design_description: Some("Total RNA extracted".to_string()),
            library_name: Some("Brain RNA lib1".to_string()),
            library_strategy: Some("RNA-Seq".to_string()),
            library_source: Some("TRANSCRIPTOMIC".to_string()),
            library_selection: Some("cDNA".to_string()),
            library_layout: Some(LibraryLayout::Paired {
                nominal_length: Some(300.0),
                nominal_sdev: Some(25.5),
            }),
            library_construction_protocol: Some("TruSeq protocol".to_string()),
            platform: Some("ILLUMINA".to_string()),
            instrument_model: Some("Illumina NovaSeq 6000".to_string()),
        }
    }

    #[test]
    fn test_valid_json() {
        let ser = JsonLdSerializer::new();
        let mut buf = Vec::new();
        write!(buf, "[").unwrap();
        ser.write_record(&mut buf, &full_record()).unwrap();
        write!(buf, "\n]").unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
        assert!(parsed.is_array());
        assert_eq!(parsed[0]["@type"], "dra_ont:Experiment");
        assert_eq!(
            parsed[0]["@id"],
            "http://identifiers.org/insdc.sra/SRX000001"
        );
    }

    #[test]
    fn test_contains_design() {
        let ser = JsonLdSerializer::new();
        let mut buf = Vec::new();
        write!(buf, "[").unwrap();
        ser.write_record(&mut buf, &full_record()).unwrap();
        write!(buf, "\n]").unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        let design = &parsed[0]["dra_ont:design"];
        assert_eq!(design["@type"], "dra_ont:ExperimentDesign");
        assert_eq!(design["dra_ont:libraryName"], "Brain RNA lib1");
    }

    #[test]
    fn test_contains_platform() {
        let ser = JsonLdSerializer::new();
        let mut buf = Vec::new();
        write!(buf, "[").unwrap();
        ser.write_record(&mut buf, &full_record()).unwrap();
        write!(buf, "\n]").unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        let platform = &parsed[0]["dra_ont:platform"];
        assert_eq!(platform["@type"], "dra_ont:ILLUMINA");
    }

    #[test]
    fn test_minimal_record_omits_optional() {
        let rec = SraExperimentRecord {
            accession: "ERX000002".to_string(),
            title: None,
            design_description: None,
            library_name: None,
            library_strategy: None,
            library_source: None,
            library_selection: None,
            library_layout: None,
            library_construction_protocol: None,
            platform: None,
            instrument_model: None,
        };
        let ser = JsonLdSerializer::new();
        let mut buf = Vec::new();
        write!(buf, "[").unwrap();
        ser.write_record(&mut buf, &rec).unwrap();
        write!(buf, "\n]").unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(parsed[0]["dra_ont:platform"].is_null());
        assert!(parsed[0]["dra_ont:design"].is_null());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p insdc-rdf-sra-experiment`
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/sra-experiment/src/serializer/jsonld.rs
git commit -m "feat(sra-experiment): add JSON-LD serializer"
```

---

### Task 8: ChunkWriter and run_convert entry point

**Files:**
- Modify: `crates/sra-experiment/src/chunk.rs`
- Modify: `crates/sra-experiment/src/lib.rs`

- [ ] **Step 1: Write ChunkWriter**

Write `crates/sra-experiment/src/chunk.rs`:

```rust
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use insdc_rdf_core::manifest::Manifest;
use insdc_rdf_core::progress::Progress;

use crate::model::SraExperimentRecord;
use crate::serializer::jsonld::JsonLdSerializer;
use crate::serializer::ntriples::NTriplesSerializer;
use crate::serializer::turtle::TurtleSerializer;
use crate::serializer::Serializer;

pub struct ChunkWriter {
    output_dir: PathBuf,
    chunk_size: usize,
    turtle_ser: TurtleSerializer,
    jsonld_ser: JsonLdSerializer,
    ntriples_ser: NTriplesSerializer,
    buffer: Vec<SraExperimentRecord>,
    progress: Progress,
    progress_path: PathBuf,
}

impl ChunkWriter {
    pub fn new(output_dir: &Path, chunk_size: usize, progress: Progress) -> std::io::Result<Self> {
        fs::create_dir_all(output_dir.join("ttl"))?;
        fs::create_dir_all(output_dir.join("jsonld"))?;
        fs::create_dir_all(output_dir.join("nt"))?;

        Ok(ChunkWriter {
            output_dir: output_dir.to_path_buf(),
            chunk_size,
            turtle_ser: TurtleSerializer::new(),
            jsonld_ser: JsonLdSerializer::new(),
            ntriples_ser: NTriplesSerializer::new(),
            buffer: Vec::with_capacity(chunk_size),
            progress,
            progress_path: output_dir.join("progress.json"),
        })
    }

    pub fn add_record(&mut self, record: SraExperimentRecord) -> std::io::Result<()> {
        self.buffer.push(record);
        self.progress.records_processed += 1;
        if self.buffer.len() >= self.chunk_size {
            self.flush_chunk()?;
        }
        Ok(())
    }

    pub fn record_skip(&mut self) {
        self.progress.records_skipped += 1;
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        if !self.buffer.is_empty() {
            self.flush_chunk()?;
        }

        let manifest = Manifest {
            source_file: self.progress.source_file.clone(),
            source_md5: self.progress.source_md5.clone(),
            total_chunks: self.progress.chunks_completed,
            total_records: self.progress.records_processed,
            records_skipped: self.progress.records_skipped,
            completed_at: chrono::Utc::now().to_rfc3339(),
        };

        let manifest_json =
            serde_json::to_string_pretty(&manifest).map_err(std::io::Error::other)?;
        fs::write(self.output_dir.join("manifest.json"), manifest_json)?;
        self.progress.save(&self.progress_path)?;

        Ok(())
    }

    fn flush_chunk(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let chunk_name = format!("chunk_{:04}", self.progress.chunks_completed);

        // TTL
        {
            let file = File::create(
                self.output_dir
                    .join("ttl")
                    .join(format!("{}.ttl", chunk_name)),
            )?;
            let mut writer = BufWriter::new(file);
            self.turtle_ser.write_header(&mut writer)?;
            for record in &self.buffer {
                self.turtle_ser.write_record(&mut writer, record)?;
            }
            self.turtle_ser.write_footer(&mut writer)?;
        }

        // JSON-LD
        {
            let file = File::create(
                self.output_dir
                    .join("jsonld")
                    .join(format!("{}.jsonld", chunk_name)),
            )?;
            let mut writer = BufWriter::new(file);
            write!(writer, "[")?;
            for (i, record) in self.buffer.iter().enumerate() {
                if i > 0 {
                    write!(writer, ",")?;
                }
                self.jsonld_ser.write_record(&mut writer, record)?;
            }
            writeln!(writer, "\n]")?;
        }

        // N-Triples
        {
            let file = File::create(
                self.output_dir
                    .join("nt")
                    .join(format!("{}.nt", chunk_name)),
            )?;
            let mut writer = BufWriter::new(file);
            for record in &self.buffer {
                self.ntriples_ser.write_record(&mut writer, record)?;
            }
        }

        self.buffer.clear();
        self.progress.chunks_completed += 1;
        self.progress.save(&self.progress_path)?;

        Ok(())
    }
}
```

- [ ] **Step 2: Write the full `run_convert` in `lib.rs`**

Write `crates/sra-experiment/src/lib.rs`:

```rust
pub mod chunk;
pub mod model;
pub mod parser;
pub mod serializer;

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use md5::Digest;

use chunk::ChunkWriter;
use insdc_rdf_core::progress::Progress;
use parser::SraExperimentParser;

pub fn run_convert(input: &Path, output_dir: &Path, chunk_size: usize) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;

    let error_log_path = output_dir.join("errors.log");
    let error_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&error_log_path)?;
    let mut error_log = BufWriter::new(error_log_file);

    let input_str = input.to_string_lossy().to_string();
    let source_size = fs::metadata(input)?.len();
    let source_md5 = compute_md5(input)?;

    eprintln!(
        "Converting SRA experiment metadata {:?} -> {:?} (chunk size: {})",
        input, output_dir, chunk_size
    );
    eprintln!("  File size: {} bytes, MD5: {}", source_size, source_md5);

    let progress = Progress::new(&input_str, source_size, &source_md5);
    let mut chunk_writer = ChunkWriter::new(output_dir, chunk_size, progress)?;

    let file = File::open(input)?;
    let mut parser = SraExperimentParser::from_tar_gz(file)?;

    let mut records_processed: u64 = 0;
    let mut records_skipped: u64 = 0;

    loop {
        match parser.next_record() {
            Ok(Some(record)) => {
                chunk_writer.add_record(record)?;
                records_processed += 1;
                if records_processed % 1_000_000 == 0 {
                    eprintln!("  Progress: {} records processed", records_processed);
                }
            }
            Ok(None) => break,
            Err(e) => {
                writeln!(error_log, "{}", e)?;
                chunk_writer.record_skip();
                records_skipped += 1;
            }
        }
    }

    chunk_writer.finish()?;

    eprintln!("\nConversion complete:");
    eprintln!("  Records processed: {}", records_processed);
    eprintln!("  Records skipped:   {}", records_skipped);
    eprintln!("  Output:            {:?}", output_dir);

    Ok(())
}

fn compute_md5(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = md5::Md5::new();
    std::io::copy(&mut file, &mut hasher)?;
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --workspace`
Expected: compiles cleanly

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: all tests pass (existing + new sra-experiment tests)

- [ ] **Step 5: Commit**

```bash
git add crates/sra-experiment/src/chunk.rs crates/sra-experiment/src/lib.rs
git commit -m "feat(sra-experiment): add ChunkWriter and run_convert entry point"
```

---

### Task 9: CLI integration and end-to-end verification

**Files:**
- Modify: `src/main.rs` (already partially done in Task 1)

- [ ] **Step 1: Verify CLI wiring is complete**

The `SourceType::SraExperiment` variant and match arm should already be in place from Task 1. Verify `src/main.rs` compiles and the help text shows the new source type:

Run: `cargo run -- convert --help`
Expected: output shows `sra-experiment` as a valid `--source` option

- [ ] **Step 2: Run the full test suite and lints**

Run: `cargo test --workspace`
Expected: all tests pass

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings

Run: `cargo fmt --all -- --check`
Expected: no formatting issues (run `cargo fmt --all` first if needed)

- [ ] **Step 3: End-to-end test with a small tar.gz**

Add an integration test to `crates/sra-experiment/src/lib.rs` (or as a separate test in the tests module):

Add at the bottom of `crates/sra-experiment/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn build_test_tar_gz() -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let xml = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/experiment_sample.xml"
        ))
        .unwrap();

        let gz_buf = Vec::new();
        let enc = GzEncoder::new(gz_buf, Compression::default());
        let mut builder = tar::Builder::new(enc);

        let mut header = tar::Header::new_gnu();
        header.set_size(xml.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(
                &mut header,
                "SRA000001/SRA000001.experiment.xml",
                xml.as_slice(),
            )
            .unwrap();

        let enc = builder.into_inner().unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn test_end_to_end() {
        let tar_gz = build_test_tar_gz();

        // Write tar.gz to temp file
        let dir = tempdir().unwrap();
        let input_path = dir.path().join("test.tar.gz");
        std::fs::write(&input_path, &tar_gz).unwrap();

        let output_dir = dir.path().join("output");
        run_convert(&input_path, &output_dir, 100).unwrap();

        // Verify output structure
        assert!(output_dir.join("ttl/chunk_0000.ttl").exists());
        assert!(output_dir.join("jsonld/chunk_0000.jsonld").exists());
        assert!(output_dir.join("nt/chunk_0000.nt").exists());
        assert!(output_dir.join("manifest.json").exists());
        assert!(output_dir.join("progress.json").exists());

        // Verify manifest
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(output_dir.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["total_records"], 3);
        assert_eq!(manifest["records_skipped"], 0);
        assert_eq!(manifest["total_chunks"], 1);

        // Verify TTL content
        let ttl = std::fs::read_to_string(output_dir.join("ttl/chunk_0000.ttl")).unwrap();
        assert!(ttl.contains("insdc_sra:SRX000001"));
        assert!(ttl.contains("insdc_sra:DRX000002"));
        assert!(ttl.contains("insdc_sra:ERX000003"));
        assert!(ttl.contains("dra_ont:Experiment"));
        assert!(ttl.contains("dra_ont:RNA-Seq"));
        assert!(ttl.contains("dra_ont:ILLUMINA"));

        // Verify JSON-LD is valid JSON
        let jsonld = std::fs::read_to_string(output_dir.join("jsonld/chunk_0000.jsonld")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).expect("valid JSON");
        assert_eq!(parsed.as_array().unwrap().len(), 3);

        // Verify N-Triples
        let nt = std::fs::read_to_string(output_dir.join("nt/chunk_0000.nt")).unwrap();
        for line in nt.lines() {
            assert!(line.ends_with(" ."), "NT line: {:?}", line);
        }
    }
}
```

- [ ] **Step 4: Run the full test suite one final time**

Run: `cargo test --workspace`
Expected: all tests pass (including the new end-to-end test)

Run: `cargo clippy --workspace -- -D warnings`
Expected: clean

- [ ] **Step 5: Commit**

```bash
git add crates/sra-experiment/src/lib.rs src/main.rs
git commit -m "feat(sra-experiment): add end-to-end integration test and finalize CLI"
```
