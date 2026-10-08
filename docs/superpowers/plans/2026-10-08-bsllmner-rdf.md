# bsllmner RDF and 2026-10 Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the bsllmner-mk2 release `2026-06_mistral-small3.1-24b-v2` to RDF as a fifth `insdc-rdf` source, then build one QLever index from it, its ontology files and the already-converted 2026-10 INSDC dumps.

**Architecture:**
- **New crate:** `crates/bsllmner` mirrors the other converters, with model, parser, three serializers and a chunk writer.
  - It reads the RO-Crate's `provenance/run_index.tsv` and `ro-crate-metadata.json`, then each `results/*.json`.
  - Each entry becomes one record: `schema:PropertyValue` annotation nodes, plus `schema:DefinedTerm` labels the first time a term is seen.
  - Provenance (one `schema:Dataset` and 311 `prov:Activity` runs) goes at the start of `chunk_0000`.
- **Ontology files:** converted unchanged by a separate Python script.
- **Index:** a parameterised shell script builds the index and starts a trial server. A validation script checks counts. Switching port 7001 is gated on the user.

**Tech Stack:** Rust 2021 workspace (serde/serde_json, percent-encoding, md-5, chrono, anyhow), Python 3 + rdflib 7, rdf-config (Ruby, local checkout at `~/repos/rdf-config`), QLever (docker image `adfreiburg/qlever`).

**Spec:** `docs/superpowers/specs/2026-10-08-bsllmner-rdf-design.md`

## Global Constraints

- Work only in the worktree `/home/inutano/repos/insdc-rdf/.worktrees/bsllmner-rdf` on branch `bsllmner-rdf`. Do not touch the main checkout's uncommitted files.
- CI must stay green: `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`.
- New crate version `0.3.0`, edition `2021`, like the other crates.
- Term IRIs are written exactly as bsllmner's `term_uri`. No normalization.
- Annotation node IRI: `http://ddbj.nig.ac.jp/biosample/{accession}#bsllmner/{release_id}/{dataset}/{field}/{term_local}`, where `term_local` = `term_id` with `:` → `_`, and the fragment is percent-encoded with the BioSample attribute fragment encode set.
- Release IRI: `https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/{release_id}/`. Run IRI: release IRI + `#run-{run_name}`. `release_id` is the input directory's basename and must match `[A-Za-z0-9._-]+`.
- Citation constant: `https://doi.org/10.1101/2025.02.17.638570`.
- Only items in `results` are converted. `reasoning` is not converted. `exact_match` becomes `biosample_ont:exactMatch` (`xsd:boolean`).
- Everything goes into the default graph.
- Ontology files are not part of the converter's output. They are converted unchanged into a separate directory.
- Do not touch `/data2/` (the live endpoint's index and the April outputs) or the `qlever-insdc` container on port 7001 without the user's explicit OK. Delete nothing under `/data1/work/insdc-rdf-202610`, `/data1/work/bsllmner`, `/data2`, `/data3`.
- Every commit message ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **A term whose first occurrence is in a skipped entry (malformed accession) or a dropped item** must still get its `DefinedTerm` and label when a later valid item uses it. Pinned by `test_malformed_accession_registers_nothing` (Task 3) and the `MONDO_0004992` assertion in the end-to-end test (Task 5).
2. **Text with quotes, backslashes, newlines, tabs or non-ASCII** in a value, label or the release name must give valid N-Triples/Turtle and round-trip through JSON-LD. Pinned by `test_escaping_in_all_formats` (Task 4).
3. **A `term_id` with characters that need encoding** (space, `#`, `%`, non-ASCII) must give a usable node IRI. Pinned by `test_annotation_iri_percent_encodes_term_id` (Task 3).
4. **Re-running into an output directory that already holds chunks** must refuse, not leave stale chunks that `cat nt/*.nt` would load into the index. Pinned by `test_refuses_output_dir_with_chunks` (Task 5).
5. **A run in `run_index.tsv` whose result file is missing** must abort and name the file, not quietly convert fewer entries. Pinned by `test_missing_result_file_is_named` (Task 5).

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/core/src/iri.rs` (new) | Fragment percent-encoding and http(s) IRI check, shared by `biosample` and `bsllmner` |
| `crates/biosample/src/model.rs` (modify) | Use `insdc_rdf_core::iri::encode_fragment` instead of its private encode set |
| `crates/core/src/prefix.rs` (modify) | Add `PROV` |
| `crates/bsllmner/Cargo.toml` (new) | Crate manifest |
| `crates/bsllmner/src/lib.rs` (new) | `run_convert`: drive runs → entries → records → chunks |
| `crates/bsllmner/src/meta.rs` (new) | Release, Run, CrateMeta; read `ro-crate-metadata.json` and `run_index.tsv` |
| `crates/bsllmner/src/model.rs` (new) | Serde types for result files; Annotation, TermOut, Record |
| `crates/bsllmner/src/annotate.rs` (new) | Accession check, node IRIs, TermRegistry, `build_record` (merging) |
| `crates/bsllmner/src/serializer/{mod,ntriples,turtle,jsonld}.rs` (new) | Write provenance and records in three formats |
| `crates/bsllmner/src/chunk.rs` (new) | Chunked output, provenance in the first chunk, manifest/progress |
| `src/main.rs`, `Cargo.toml` (modify) | `--source bsllmner` |
| `tests/fixtures/bsllmner/2026-06_test-release/` (new) | Fixture RO-Crate |
| `tests/fixtures/bsllmner/ontology/sample.owl` (new) | Fixture ontology |
| `scripts/bsllmner_ontology_to_nt.py` (new) | OWL → N-Triples for the crate's ontology files |
| `config/bsllmner/*` (new), `config/biosample/{model,sparql}.yaml` + generated files (modify) | rdf-config schema |
| `scripts/qlever_rebuild_index.sh` (new) | Build an index from N-Triples dirs and start a server |
| `scripts/validate_bsllmner_qlever.sh` (new) | SPARQL count checks for the annotations |
| `README.md` (modify) | Document the fifth source, the refresh, ontologies and limitations |

---

### Task 1: Shared IRI helpers in core

**Files:**
- Create: `crates/core/src/iri.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`, `crates/core/src/prefix.rs`, `crates/biosample/src/model.rs:34-70`, `crates/biosample/Cargo.toml`

**Interfaces:**
- Produces:
  - `insdc_rdf_core::iri::encode_fragment(s: &str) -> String`
  - `insdc_rdf_core::iri::is_http_iri(iri: &str) -> bool`
  - `insdc_rdf_core::prefix::PROV: &str`

- [ ] **Step 1: Write the failing tests**

Create `crates/core/src/iri.rs` with only the tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_fragment_spaces_and_hash() {
        assert_eq!(encode_fragment("sample name#1"), "sample%20name%231");
    }

    #[test]
    fn test_encode_fragment_keeps_slash_colon_underscore() {
        assert_eq!(encode_fragment("bsllmner/a:b_c-d.e"), "bsllmner/a:b_c-d.e");
    }

    #[test]
    fn test_encode_fragment_non_ascii_and_percent() {
        assert_eq!(encode_fragment("β%"), "%CE%B2%25");
    }

    #[test]
    fn test_is_http_iri() {
        assert!(is_http_iri("http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027"));
        assert!(is_http_iri("https://example.org/x"));
        for bad in [
            "",
            "not an iri",
            "ftp://x",
            "http://x y",
            "http://x>y",
            "http://x\"y",
            "http://x\ny",
            "http://x\\y",
        ] {
            assert!(!is_http_iri(bad), "{:?}", bad);
        }
    }
}
```

Add `pub mod iri;` to `crates/core/src/lib.rs` (keep the list alphabetical: `error, escape, iri, manifest, prefix, progress`).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p insdc-rdf-core iri`
Expected: compile error, `cannot find function encode_fragment` / `is_http_iri`.

- [ ] **Step 3: Implement**

Add to `crates/core/Cargo.toml` under `[dependencies]`: `percent-encoding = "2"`.

Prepend to `crates/core/src/iri.rs`:

```rust
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

/// Characters that must be percent-encoded in an IRI fragment.
const IRI_FRAGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b']')
    .add(b'{')
    .add(b'}')
    .add(b'|')
    .add(b'^')
    .add(b'`')
    .add(b'\\');

/// Percent-encodes `s` for use as (part of) an IRI fragment. Non-ASCII is always encoded.
pub fn encode_fragment(s: &str) -> String {
    utf8_percent_encode(s, IRI_FRAGMENT_ENCODE_SET).to_string()
}

/// True for an absolute http(s) IRI that can be written between `<` and `>` in N-Triples as is.
pub fn is_http_iri(iri: &str) -> bool {
    (iri.starts_with("http://") || iri.starts_with("https://"))
        && !iri.chars().any(|c| {
            c.is_control()
                || c.is_whitespace()
                || matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\')
        })
}
```

Add to `crates/core/src/prefix.rs` after `SCHEMA`:

```rust
pub const PROV: &str = "http://www.w3.org/ns/prov#";
```

In `crates/biosample/src/model.rs`:
- Delete the `use percent_encoding::…` line and the `IRI_FRAGMENT_ENCODE_SET` constant (lines 34–51).
- Change the body of `property_iri` to:

```rust
    pub fn property_iri(&self, accession: &str) -> String {
        format!(
            "{}{}#{}",
            insdc_rdf_core::prefix::DDBJ_BIOSAMPLE,
            accession,
            insdc_rdf_core::iri::encode_fragment(self.preferred_name())
        )
    }
```

Remove `percent-encoding = "2"` from `crates/biosample/Cargo.toml`. It was only used in `model.rs`; check with `grep -rn percent_encoding crates/biosample`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --workspace`
Expected: all pass. That is the 108 baseline tests, including biosample's `test_attribute_property_iri_encodes_spaces`, plus 4 new ones.

- [ ] **Step 5: Lint and commit**

```bash
cargo fmt --all && cargo clippy --workspace -- -D warnings
git add crates/core crates/biosample
git commit -m "refactor: move IRI fragment encoding to core and add is_http_iri

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: bsllmner crate scaffold and RO-Crate metadata

**Files:**
- Create: `crates/bsllmner/Cargo.toml`, `crates/bsllmner/src/lib.rs`, `crates/bsllmner/src/meta.rs`
- Create: `tests/fixtures/bsllmner/2026-06_test-release/ro-crate-metadata.json`, `…/provenance/run_index.tsv`, `…/provenance/checksums.sha256`

**Interfaces:**
- Consumes: `insdc_rdf_core::iri::is_http_iri`
- Produces (in `insdc_rdf_bsllmner::meta`):
  - `pub const RELEASE_BASE: &str`, `pub const CITATION: &str`
  - `pub struct Release { pub release_id: String, pub name: String, pub date_published: String, pub license: String }` with `fn iri(&self) -> String` and `fn run_iri(&self, run_name: &str) -> String`
  - `pub struct Run { pub run_name: String, pub dataset: String, pub model: String, pub first_start: String, pub last_end: String, pub commit_urls: Vec<String>, pub result_file: String }`
  - `pub struct CrateMeta { pub release: Release, pub runs: Vec<Run> }`
  - `pub fn load_crate(dir: &Path) -> anyhow::Result<CrateMeta>`
  - `pub fn release_id_from_dir(dir: &Path) -> anyhow::Result<String>`
  - `pub fn parse_ro_crate(meta: &serde_json::Value, release_id: String) -> anyhow::Result<(Release, HashMap<String, String>)>`
  - `pub fn parse_run_index(tsv: &str, commits: &HashMap<String, String>) -> anyhow::Result<Vec<Run>>`
  - All four structs derive `Debug, Clone, PartialEq`.

- [ ] **Step 1: Create the fixture crate's metadata**

```bash
F=tests/fixtures/bsllmner/2026-06_test-release
mkdir -p $F/provenance $F/results
cat > $F/ro-crate-metadata.json <<'EOF'
{
  "@context": "https://w3id.org/ro/crate/1.1/context",
  "@graph": [
    {"@id": "ro-crate-metadata.json", "@type": "CreativeWork", "conformsTo": {"@id": "https://w3id.org/ro/crate/1.1"}, "about": {"@id": "./"}},
    {"@id": "./", "@type": "Dataset", "name": "Test release of ontology-mapped named entities", "description": "Fixture for insdc-rdf tests.", "datePublished": "2026-09-26", "license": {"@id": "https://creativecommons.org/licenses/by/4.0/"}},
    {"@id": "#bsllmner2-5a5744e", "@type": "SoftwareApplication", "name": "bsllmner2", "softwareVersion": "5a5744ee95cac05ce02bf272a690d3975261d0a0", "url": "https://github.com/dbcls/bsllmner-mk2/commit/5a5744ee95cac05ce02bf272a690d3975261d0a0"},
    {"@id": "#bsllmner2-9a38288", "@type": "SoftwareApplication", "name": "bsllmner2", "softwareVersion": "9a3828811f1ab4bac85615e0e1b2efcc6603265f", "url": "https://github.com/dbcls/bsllmner-mk2/commit/9a3828811f1ab4bac85615e0e1b2efcc6603265f"},
    {"@id": "#model-mistral-small3-1-24b", "@type": "SoftwareApplication", "name": "mistral-small3.1:24b"}
  ]
}
EOF
{
  printf 'run_name\tdataset\tinput_file\tselect_config\tmodel\tnum_ctx\tbatch_size\tthinking\tinclude_reasoning\tfirst_start\tlast_end\tsegments\tstatus\ttotal_input_entries\tcompleted_count\ttotal_wall_sec\terrors_count\tcode_commit\tresult_file\tresult_bytes\tresult_sha256\tdeduplicated_input_entries\tdeduplicated_result_entries\n'
  printf 'chipatlas_hg38_part1\tchipatlas_hg38\tbs_entries_chipatlas_hg38_part1.jsonl\tselect-config-hg38.json\tmistral-small3.1:24b\t4096\t2048\tfalse\tfalse\t2026-05-27T07:37:37+00:00\t2026-05-27T22:45:00+00:00\t1\tcompleted\t4\t4\t54423.2\t0\t5a5744e\tselect_chipatlas_hg38_part1.json\t0\t0\t4\t4\n'
  printf 'rnaseq_human_5y_2021-08\trnaseq_human_5y\tbs_entries_rnaseq_human_5y_2021-08.jsonl\tselect-config-hg38.json\tmistral-small3.1:24b\t4096\t2048\tfalse\tfalse\t2026-05-30T17:20:09+00:00\t2026-05-30T22:16:22+00:00\t2\tcompleted\t3\t3\t17761.7\t0\t5a5744e+9a38288\tselect_rnaseq_human_5y_2021-08.json\t0\t0\t3\t3\n'
} > $F/provenance/run_index.tsv
printf '0000000000000000000000000000000000000000000000000000000000000000  results/select_chipatlas_hg38_part1.json\n' > $F/provenance/checksums.sha256
```

- [ ] **Step 2: Create the crate with failing tests**

`crates/bsllmner/Cargo.toml`:

```toml
[package]
name = "insdc-rdf-bsllmner"
version = "0.3.0"
edition = "2021"
description = "bsllmner-mk2 ontology annotation RO-Crate to RDF converter"

[dependencies]
insdc-rdf-core = { path = "../core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
md-5 = "0.10"

[dev-dependencies]
insdc-rdf-biosample = { path = "../biosample" }
tempfile = "3"
```

`crates/bsllmner/src/lib.rs`:

```rust
pub mod meta;
```

`crates/bsllmner/src/meta.rs`, with tests only for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/bsllmner/2026-06_test-release"
        ))
    }

    const HEADER: &str =
        "run_name\tdataset\tmodel\tfirst_start\tlast_end\tcode_commit\tresult_file\n";

    #[test]
    fn test_load_crate_release() {
        let meta = load_crate(&fixture_dir()).unwrap();
        let rel = &meta.release;
        assert_eq!(rel.release_id, "2026-06_test-release");
        assert_eq!(rel.name, "Test release of ontology-mapped named entities");
        assert_eq!(rel.date_published, "2026-09-26");
        assert_eq!(rel.license, "https://creativecommons.org/licenses/by/4.0/");
        assert_eq!(
            rel.iri(),
            "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_test-release/"
        );
        assert_eq!(
            rel.run_iri("chipatlas_hg38_part1"),
            "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_test-release/#run-chipatlas_hg38_part1"
        );
    }

    #[test]
    fn test_load_crate_accepts_dot_path() {
        let meta = load_crate(&fixture_dir().join(".")).unwrap();
        assert_eq!(meta.release.release_id, "2026-06_test-release");
    }

    #[test]
    fn test_load_crate_runs_in_order_with_commits() {
        let meta = load_crate(&fixture_dir()).unwrap();
        assert_eq!(meta.runs.len(), 2);
        assert_eq!(
            meta.runs[0],
            Run {
                run_name: "chipatlas_hg38_part1".into(),
                dataset: "chipatlas_hg38".into(),
                model: "mistral-small3.1:24b".into(),
                first_start: "2026-05-27T07:37:37+00:00".into(),
                last_end: "2026-05-27T22:45:00+00:00".into(),
                commit_urls: vec![
                    "https://github.com/dbcls/bsllmner-mk2/commit/5a5744ee95cac05ce02bf272a690d3975261d0a0".into()
                ],
                result_file: "select_chipatlas_hg38_part1.json".into(),
            }
        );
        assert_eq!(
            meta.runs[1].commit_urls,
            vec![
                "https://github.com/dbcls/bsllmner-mk2/commit/5a5744ee95cac05ce02bf272a690d3975261d0a0".to_string(),
                "https://github.com/dbcls/bsllmner-mk2/commit/9a3828811f1ab4bac85615e0e1b2efcc6603265f".to_string(),
            ]
        );
    }

    #[test]
    fn test_release_id_rejects_unsafe_names() {
        assert!(release_id_from_dir(Path::new("/x/2026-06_ok.v2")).is_ok());
        assert!(release_id_from_dir(Path::new("/x/bad name")).is_err());
        assert!(release_id_from_dir(Path::new("/x/bad#id")).is_err());
    }

    #[test]
    fn test_unknown_commit_is_an_error() {
        let tsv = format!("{}r1\td1\tm\ts\te\tdeadbee\tf.json\n", HEADER);
        let err = parse_run_index(&tsv, &HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("deadbee"), "{}", err);
    }

    #[test]
    fn test_missing_column_is_an_error() {
        let err = parse_run_index("run_name\tdataset\nr1\td1\n", &HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("model"), "{}", err);
    }

    #[test]
    fn test_unsafe_run_name_or_dataset_is_an_error() {
        let mut commits = HashMap::new();
        commits.insert("abc".to_string(), "https://example.org/abc".to_string());
        for row in ["r 1\td1\tm\ts\te\tabc\tf.json\n", "r1\td#1\tm\ts\te\tabc\tf.json\n"] {
            assert!(parse_run_index(&format!("{}{}", HEADER, row), &commits).is_err(), "{}", row);
        }
    }

    #[test]
    fn test_ro_crate_without_root_is_an_error() {
        let meta: serde_json::Value = serde_json::json!({"@graph": []});
        assert!(parse_ro_crate(&meta, "r".into()).is_err());
    }

    #[test]
    fn test_ro_crate_with_bad_commit_url_is_an_error() {
        let meta = serde_json::json!({"@graph": [
            {"@id": "./", "name": "n", "datePublished": "2026-01-01", "license": {"@id": "https://creativecommons.org/licenses/by/4.0/"}},
            {"@id": "#bsllmner2-abc", "url": "not a url"}
        ]});
        assert!(parse_ro_crate(&meta, "r".into()).is_err());
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p insdc-rdf-bsllmner`
Expected: compile errors (`load_crate`, `Run`, `parse_run_index` … not found).

- [ ] **Step 4: Implement `meta.rs`**

Prepend to `crates/bsllmner/src/meta.rs`:

```rust
//! Release and run metadata read from a bsllmner-mk2 RO-Crate.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context};
use insdc_rdf_core::iri::is_http_iri;

/// Base URL under which bsllmner-mk2 releases are published.
pub const RELEASE_BASE: &str = "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/";

/// The bsllmner-mk2 publication. The RO-Crate root carries no citation, so it is fixed here.
pub const CITATION: &str = "https://doi.org/10.1101/2025.02.17.638570";

const RUN_INDEX_COLUMNS: [&str; 7] = [
    "run_name",
    "dataset",
    "model",
    "first_start",
    "last_end",
    "code_commit",
    "result_file",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub release_id: String,
    pub name: String,
    pub date_published: String,
    pub license: String,
}

impl Release {
    pub fn iri(&self) -> String {
        format!("{}{}/", RELEASE_BASE, self.release_id)
    }

    /// The RO-Crate's `#run-<name>` id, resolved against the crate root.
    pub fn run_iri(&self, run_name: &str) -> String {
        format!("{}#run-{}", self.iri(), run_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub run_name: String,
    pub dataset: String,
    pub model: String,
    pub first_start: String,
    pub last_end: String,
    pub commit_urls: Vec<String>,
    pub result_file: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrateMeta {
    pub release: Release,
    pub runs: Vec<Run>,
}

fn is_safe_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The release id is the crate directory's name, e.g. `2026-06_mistral-small3.1-24b-v2`.
pub fn release_id_from_dir(dir: &Path) -> anyhow::Result<String> {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("cannot take a release id from {:?}", dir))?;
    if !is_safe_name(name) {
        bail!(
            "release id {:?} (the input directory name) must match [A-Za-z0-9._-]+",
            name
        );
    }
    Ok(name.to_string())
}

/// Reads release and run metadata from an unpacked RO-Crate directory.
pub fn load_crate(dir: &Path) -> anyhow::Result<CrateMeta> {
    let dir = dir
        .canonicalize()
        .with_context(|| format!("input directory {:?}", dir))?;
    let release_id = release_id_from_dir(&dir)?;

    let meta_path = dir.join("ro-crate-metadata.json");
    let text =
        fs::read_to_string(&meta_path).with_context(|| format!("reading {:?}", meta_path))?;
    let meta: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {:?}", meta_path))?;
    let (release, commits) = parse_ro_crate(&meta, release_id)?;

    let tsv_path = dir.join("provenance").join("run_index.tsv");
    let tsv = fs::read_to_string(&tsv_path).with_context(|| format!("reading {:?}", tsv_path))?;
    let runs = parse_run_index(&tsv, &commits)?;

    Ok(CrateMeta { release, runs })
}

/// Reads the root Dataset and the `#bsllmner2-<short commit>` SoftwareApplications.
/// Returns the release and a map from short commit hash to commit URL.
pub fn parse_ro_crate(
    meta: &serde_json::Value,
    release_id: String,
) -> anyhow::Result<(Release, HashMap<String, String>)> {
    let graph = meta["@graph"]
        .as_array()
        .ok_or_else(|| anyhow!("ro-crate-metadata.json has no @graph array"))?;
    let root = graph
        .iter()
        .find(|e| e["@id"] == "./")
        .ok_or_else(|| anyhow!("ro-crate-metadata.json has no root Dataset \"./\""))?;
    let text = |key: &str| -> anyhow::Result<String> {
        root[key]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("root Dataset has no string {:?}", key))
    };
    let license = root["license"]["@id"]
        .as_str()
        .or_else(|| root["license"].as_str())
        .filter(|l| is_http_iri(l))
        .ok_or_else(|| anyhow!("root Dataset has no http(s) license IRI"))?
        .to_string();
    let release = Release {
        release_id,
        name: text("name")?,
        date_published: text("datePublished")?,
        license,
    };

    let mut commits = HashMap::new();
    for entity in graph {
        let Some(short) = entity["@id"]
            .as_str()
            .and_then(|id| id.strip_prefix("#bsllmner2-"))
        else {
            continue;
        };
        let url = entity["url"]
            .as_str()
            .filter(|u| is_http_iri(u))
            .ok_or_else(|| anyhow!("#bsllmner2-{} has no http(s) url", short))?;
        commits.insert(short.to_string(), url.to_string());
    }
    Ok((release, commits))
}

/// Parses `provenance/run_index.tsv`, keeping file order. Each `code_commit`
/// (short hashes joined by `+`) is resolved to commit URLs through `commits`.
pub fn parse_run_index(tsv: &str, commits: &HashMap<String, String>) -> anyhow::Result<Vec<Run>> {
    let mut lines = tsv.lines();
    let header: Vec<&str> = lines
        .next()
        .ok_or_else(|| anyhow!("run_index.tsv is empty"))?
        .split('\t')
        .collect();
    let mut idx: HashMap<&str, usize> = HashMap::new();
    for col in RUN_INDEX_COLUMNS {
        let i = header
            .iter()
            .position(|h| *h == col)
            .ok_or_else(|| anyhow!("run_index.tsv has no column {:?}", col))?;
        idx.insert(col, i);
    }

    let mut runs = Vec::new();
    for (n, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let line_no = n + 2;
        let cols: Vec<&str> = line.split('\t').collect();
        let get = |col: &str| -> anyhow::Result<String> {
            cols.get(idx[col])
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("run_index.tsv line {}: no value for {:?}", line_no, col))
        };
        let run_name = get("run_name")?;
        let dataset = get("dataset")?;
        for (kind, value) in [("run_name", &run_name), ("dataset", &dataset)] {
            if !is_safe_name(value) {
                bail!(
                    "run_index.tsv line {}: {} {:?} must match [A-Za-z0-9._-]+",
                    line_no,
                    kind,
                    value
                );
            }
        }
        let commit_urls = get("code_commit")?
            .split('+')
            .map(|short| {
                commits.get(short).cloned().ok_or_else(|| {
                    anyhow!(
                        "run_index.tsv line {}: commit {:?} has no #bsllmner2-{} entity in ro-crate-metadata.json",
                        line_no,
                        short,
                        short
                    )
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        runs.push(Run {
            run_name,
            dataset,
            model: get("model")?,
            first_start: get("first_start")?,
            last_end: get("last_end")?,
            commit_urls,
            result_file: get("result_file")?,
        });
    }
    Ok(runs)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p insdc-rdf-bsllmner`
Expected: 9 passed.

- [ ] **Step 6: Lint and commit**

```bash
cargo fmt --all && cargo clippy --workspace -- -D warnings
git add crates/bsllmner tests/fixtures/bsllmner Cargo.lock
git commit -m "feat(bsllmner): read release and run metadata from the RO-Crate

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Result entries → annotation records

**Files:**
- Create: `crates/bsllmner/src/model.rs`, `crates/bsllmner/src/annotate.rs`
- Modify: `crates/bsllmner/src/lib.rs`

**Interfaces:**
- Consumes: `meta::{Release, Run}` (Task 2), `insdc_rdf_core::iri::{encode_fragment, is_http_iri}` (Task 1)
- Produces (in `insdc_rdf_bsllmner::model`):
  - `ResultFile { entries: Vec<Entry> }`
  - `Entry { extract: Extract, results: BTreeMap<String, Vec<ResultItem>> }`
  - `Extract { accession: String }`
  - `ResultItem { value, term_id, term_uri, label: String, exact_match: bool }`
  - `Annotation { iri: String, field: String, values: Vec<String>, term_uri: String, exact_match: bool }`
  - `TermOut { term_uri: String, declare_type: bool, label: String }`
  - `Record { biosample_iri: String, run_iri: String, annotations: Vec<Annotation>, terms: Vec<TermOut> }`
- Produces (in `insdc_rdf_bsllmner::annotate`):
  - `is_valid_accession(&str) -> bool`
  - `annotation_iri(accession, release_id, dataset, field, term_id: &str) -> String`
  - `TermRegistry` with `new()`, `register(&mut self, term_uri: &str, label: &str) -> Option<TermOut>` and `term_count(&self) -> usize`
  - `build_record(entry: &Entry, run: &Run, release: &Release, terms: &mut TermRegistry) -> Result<(Record, Vec<String>), String>`. `Err` holds a log line for a skipped entry; the `Vec` holds log lines for dropped items.

- [ ] **Step 1: Write the model types with a failing test**

`crates/bsllmner/src/model.rs`:

```rust
//! Types read from bsllmner-mk2 result files, and the records written from them.

use std::collections::BTreeMap;

use serde::Deserialize;

/// One `results/select_<run>.json`. Only the fields the converter uses are declared;
/// serde skips the rest (candidate lists, timings, run metadata) without keeping them.
#[derive(Debug, Deserialize)]
pub struct ResultFile {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub extract: Extract,
    /// Field name -> selected terms. A field that was extracted but not mapped is absent.
    #[serde(default)]
    pub results: BTreeMap<String, Vec<ResultItem>>,
}

#[derive(Debug, Deserialize)]
pub struct Extract {
    pub accession: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResultItem {
    pub value: String,
    pub term_id: String,
    pub term_uri: String,
    pub label: String,
    pub exact_match: bool,
}

/// One annotation node: all items of an entry and field that share a term.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub iri: String,
    pub field: String,
    pub values: Vec<String>,
    pub term_uri: String,
    pub exact_match: bool,
}

/// Term statements still to be written: the type the first time a term is seen,
/// and a label the first time each (term, label) pair is seen.
#[derive(Debug, Clone, PartialEq)]
pub struct TermOut {
    pub term_uri: String,
    pub declare_type: bool,
    pub label: String,
}

/// Everything written for one entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub biosample_iri: String,
    pub run_iri: String,
    pub annotations: Vec<Annotation>,
    pub terms: Vec<TermOut>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_file_ignores_other_keys() {
        let f: ResultFile = serde_json::from_str(
            r#"{"entries": [{"extract": {"accession": "SAMN1", "extracted": {"x": null}, "raw_output": "{}"},
                             "search_results": {"x": {}}, "text2term_results": {}, "select_timings": {},
                             "ambiguous_fields": {}, "results": {"x": []}}],
                "run_metadata": {"run_name": "r"}, "evaluation": null, "performance": {}, "errors": []}"#,
        )
        .unwrap();
        assert_eq!(f.entries.len(), 1);
        assert_eq!(f.entries[0].extract.accession, "SAMN1");
        assert!(f.entries[0].results["x"].is_empty());
    }

    #[test]
    fn test_entry_without_results_key() {
        let e: Entry = serde_json::from_str(r#"{"extract": {"accession": "SAMN1"}}"#).unwrap();
        assert!(e.results.is_empty());
    }
}
```

Change `crates/bsllmner/src/lib.rs` to:

```rust
pub mod annotate;
pub mod meta;
pub mod model;
```

Create `crates/bsllmner/src/annotate.rs` with the tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Release {
        Release {
            release_id: "rel-1".into(),
            name: "n".into(),
            date_published: "2026-09-26".into(),
            license: "https://creativecommons.org/licenses/by/4.0/".into(),
        }
    }

    fn run(dataset: &str) -> Run {
        Run {
            run_name: format!("{}_r1", dataset),
            dataset: dataset.into(),
            model: "m".into(),
            first_start: "s".into(),
            last_end: "e".into(),
            commit_urls: vec![],
            result_file: "f.json".into(),
        }
    }

    fn entry(json: &str) -> Entry {
        serde_json::from_str(json).unwrap()
    }

    const HEPG2: &str = r#"{
      "extract": {"accession": "SAMD00270091", "extracted": {"cell_line": "HepG2"}},
      "results": {
        "cell_line": [{"value": "HepG2", "term_id": "CVCL:0027",
          "term_uri": "http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027", "label": "Hep-G2",
          "exact_match": true, "reasoning": "Exact match on oboInOwl:hasRelatedSynonym"}],
        "drug": []
      }
    }"#;

    const CELLOSAURUS_0027: &str = "http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027";

    #[test]
    fn test_accession_validation() {
        for ok in ["SAMN00000002", "SAMD00270091", "SAMEA6161248"] {
            assert!(is_valid_accession(ok), "{}", ok);
        }
        for bad in ["", "SAM", "SAMN", "SAMX0001", "SAMN12a", "BADACC123", "SAMEAB123", " SAMN1"] {
            assert!(!is_valid_accession(bad), "{}", bad);
        }
    }

    #[test]
    fn test_annotation_iri() {
        assert_eq!(
            annotation_iri(
                "SAMD00270091",
                "2026-06_mistral-small3.1-24b-v2",
                "rnaseq_human_5y",
                "cell_line",
                "CVCL:0027"
            ),
            "http://ddbj.nig.ac.jp/biosample/SAMD00270091#bsllmner/2026-06_mistral-small3.1-24b-v2/rnaseq_human_5y/cell_line/CVCL_0027"
        );
    }

    #[test]
    fn test_annotation_iri_percent_encodes_term_id() {
        let iri = annotation_iri("SAMN1", "r", "d", "f", "X:a b#c%β");
        assert!(iri.ends_with("#bsllmner/r/d/f/X_a%20b%23c%25%CE%B2"), "{}", iri);
        assert!(insdc_rdf_core::iri::is_http_iri(&iri));
    }

    #[test]
    fn test_build_record_scalar_field() {
        let mut reg = TermRegistry::new();
        let (rec, dropped) =
            build_record(&entry(HEPG2), &run("rnaseq_human_5y"), &release(), &mut reg).unwrap();
        assert!(dropped.is_empty());
        assert_eq!(rec.biosample_iri, "http://identifiers.org/biosample/SAMD00270091");
        assert_eq!(
            rec.run_iri,
            "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/rel-1/#run-rnaseq_human_5y_r1"
        );
        assert_eq!(
            rec.annotations,
            vec![Annotation {
                iri: "http://ddbj.nig.ac.jp/biosample/SAMD00270091#bsllmner/rel-1/rnaseq_human_5y/cell_line/CVCL_0027".into(),
                field: "cell_line".into(),
                values: vec!["HepG2".into()],
                term_uri: CELLOSAURUS_0027.into(),
                exact_match: true,
            }]
        );
        assert_eq!(
            rec.terms,
            vec![TermOut {
                term_uri: CELLOSAURUS_0027.into(),
                declare_type: true,
                label: "Hep-G2".into()
            }]
        );
    }

    #[test]
    fn test_build_record_merges_items_with_same_term() {
        let e = entry(
            r#"{"extract": {"accession": "SAMN13836220"}, "results": {"knockout_gene": [
            {"value": "REV-ERB a", "term_id": "NCBIGene:353187", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_353187", "label": "Nr1d1", "exact_match": false, "reasoning": null},
            {"value": "REV-ERB b", "term_id": "NCBIGene:353187", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_353187", "label": "Nr1d1", "exact_match": true, "reasoning": null},
            {"value": "REV-ERB a", "term_id": "NCBIGene:353187", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_353187", "label": "Nr1d1", "exact_match": false, "reasoning": null}
        ]}}"#,
        );
        let mut reg = TermRegistry::new();
        let (rec, _) = build_record(&e, &run("rnaseq_mouse"), &release(), &mut reg).unwrap();
        assert_eq!(rec.annotations.len(), 1);
        assert_eq!(
            rec.annotations[0].values,
            vec!["REV-ERB a".to_string(), "REV-ERB b".to_string()]
        );
        assert!(rec.annotations[0].exact_match);
        assert_eq!(rec.terms.len(), 1);
    }

    #[test]
    fn test_same_term_in_two_fields_gives_two_nodes() {
        let e = entry(
            r#"{"extract": {"accession": "SAMN1"}, "results": {
            "chip_antigen": [{"value": "CTCF", "term_id": "NCBIGene:10664", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_10664", "label": "CTCF", "exact_match": true, "reasoning": null}],
            "knockdown_gene": [{"value": "CTCF", "term_id": "NCBIGene:10664", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_10664", "label": "CTCF", "exact_match": true, "reasoning": null}]
        }}"#,
        );
        let mut reg = TermRegistry::new();
        let (rec, _) = build_record(&e, &run("d"), &release(), &mut reg).unwrap();
        assert_eq!(rec.annotations.len(), 2);
        assert_ne!(rec.annotations[0].iri, rec.annotations[1].iri);
        assert_eq!(rec.terms.len(), 1);
    }

    #[test]
    fn test_datasets_give_distinct_iris() {
        let mut reg = TermRegistry::new();
        let (a, _) = build_record(&entry(HEPG2), &run("chipatlas_hg38"), &release(), &mut reg).unwrap();
        let (b, _) = build_record(&entry(HEPG2), &run("rnaseq_human_5y"), &release(), &mut reg).unwrap();
        assert_ne!(a.annotations[0].iri, b.annotations[0].iri);
        assert_ne!(a.run_iri, b.run_iri);
        assert!(b.terms.is_empty(), "term already written by the first record");
    }

    #[test]
    fn test_empty_or_missing_results_give_no_annotations() {
        let mut reg = TermRegistry::new();
        for json in [
            r#"{"extract": {"accession": "SAMEA6161248", "extracted": {}}, "results": {}}"#,
            r#"{"extract": {"accession": "SAMEA6161248"}}"#,
        ] {
            let (rec, dropped) = build_record(&entry(json), &run("d"), &release(), &mut reg).unwrap();
            assert!(rec.annotations.is_empty() && rec.terms.is_empty() && dropped.is_empty());
        }
    }

    #[test]
    fn test_malformed_accession_registers_nothing() {
        let mut reg = TermRegistry::new();
        let bad = HEPG2.replace("SAMD00270091", "BADACC123");
        let err = build_record(&entry(&bad), &run("d"), &release(), &mut reg).unwrap_err();
        assert!(err.contains("BADACC123"), "{}", err);
        assert_eq!(reg.term_count(), 0);
        let (rec, _) = build_record(&entry(HEPG2), &run("d"), &release(), &mut reg).unwrap();
        assert!(rec.terms[0].declare_type);
    }

    #[test]
    fn test_unusable_term_iri_is_dropped_and_logged() {
        let e = entry(
            r#"{"extract": {"accession": "SAMN00000003"}, "results": {"disease": [
            {"value": "x", "term_id": "X:1", "term_uri": "not an iri", "label": "x", "exact_match": false, "reasoning": null},
            {"value": "cancer", "term_id": "MONDO:0004992", "term_uri": "http://purl.obolibrary.org/obo/MONDO_0004992", "label": "cancer", "exact_match": true, "reasoning": null}
        ]}}"#,
        );
        let mut reg = TermRegistry::new();
        let (rec, dropped) = build_record(&e, &run("d"), &release(), &mut reg).unwrap();
        assert_eq!(rec.annotations.len(), 1);
        assert_eq!(dropped.len(), 1);
        assert!(dropped[0].contains("not an iri"), "{}", dropped[0]);
        assert_eq!(reg.term_count(), 1);
    }

    #[test]
    fn test_registry_emits_new_label_without_type() {
        let mut reg = TermRegistry::new();
        assert!(reg.register("http://t/1", "a").unwrap().declare_type);
        assert_eq!(reg.register("http://t/1", "a"), None);
        assert_eq!(
            reg.register("http://t/1", "b"),
            Some(TermOut {
                term_uri: "http://t/1".into(),
                declare_type: false,
                label: "b".into()
            })
        );
        assert_eq!(reg.term_count(), 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p insdc-rdf-bsllmner`
Expected: the model tests pass; `annotate` fails to compile (`TermRegistry`, `build_record`, … not found).

- [ ] **Step 3: Implement `annotate.rs`**

Prepend to `crates/bsllmner/src/annotate.rs`:

```rust
//! Turns one result entry into the annotation nodes and term statements to write.

use std::collections::HashSet;

use insdc_rdf_core::iri::{encode_fragment, is_http_iri};
use insdc_rdf_core::prefix::{DDBJ_BIOSAMPLE, IDORG_BIOSAMPLE};

use crate::meta::{Release, Run};
use crate::model::{Annotation, Entry, Record, TermOut};

/// True for `SAM` + N/D/E + an optional uppercase letter + digits (SAMN…, SAMD…, SAMEA…).
pub fn is_valid_accession(acc: &str) -> bool {
    let Some(rest) = acc.strip_prefix("SAM") else {
        return false;
    };
    let mut chars = rest.chars().peekable();
    if !matches!(chars.next(), Some('N' | 'D' | 'E')) {
        return false;
    }
    if chars.peek().is_some_and(|c| c.is_ascii_uppercase()) {
        chars.next();
    }
    let digits: String = chars.collect();
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// `http://ddbj.nig.ac.jp/biosample/{acc}#bsllmner/{release}/{dataset}/{field}/{term_local}`,
/// where `term_local` is the term id with `:` replaced by `_`.
pub fn annotation_iri(
    accession: &str,
    release_id: &str,
    dataset: &str,
    field: &str,
    term_id: &str,
) -> String {
    let fragment = format!(
        "bsllmner/{}/{}/{}/{}",
        release_id,
        dataset,
        field,
        term_id.replace(':', "_")
    );
    format!("{}{}#{}", DDBJ_BIOSAMPLE, accession, encode_fragment(&fragment))
}

/// Remembers, across the whole conversion, which terms and (term, label) pairs were written.
#[derive(Debug, Default)]
pub struct TermRegistry {
    terms: HashSet<String>,
    labels: HashSet<(String, String)>,
}

impl TermRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns what still has to be written for this term and label, or None if nothing does.
    pub fn register(&mut self, term_uri: &str, label: &str) -> Option<TermOut> {
        let declare_type = self.terms.insert(term_uri.to_string());
        let new_label = self
            .labels
            .insert((term_uri.to_string(), label.to_string()));
        (declare_type || new_label).then(|| TermOut {
            term_uri: term_uri.to_string(),
            declare_type,
            label: label.to_string(),
        })
    }

    pub fn term_count(&self) -> usize {
        self.terms.len()
    }
}

/// Builds the record for one entry of `run`.
///
/// Returns `Err(log line)` for a malformed accession; nothing is registered then, so a
/// term first seen in a skipped entry is still written where it is next used. The `Vec`
/// holds log lines for items dropped because their term IRI is unusable.
pub fn build_record(
    entry: &Entry,
    run: &Run,
    release: &Release,
    terms: &mut TermRegistry,
) -> Result<(Record, Vec<String>), String> {
    let acc = &entry.extract.accession;
    if !is_valid_accession(acc) {
        return Err(format!(
            "{}: skipped entry with malformed accession {:?}",
            run.run_name, acc
        ));
    }

    let mut annotations: Vec<Annotation> = Vec::new();
    let mut term_outs = Vec::new();
    let mut dropped = Vec::new();
    for (field, items) in &entry.results {
        let field_start = annotations.len();
        for item in items {
            if !is_http_iri(&item.term_uri) {
                dropped.push(format!(
                    "{}: {} {}: dropped item with unusable term IRI {:?}",
                    run.run_name, acc, field, item.term_uri
                ));
                continue;
            }
            match annotations[field_start..]
                .iter_mut()
                .find(|a| a.term_uri == item.term_uri)
            {
                Some(a) => {
                    if !a.values.contains(&item.value) {
                        a.values.push(item.value.clone());
                    }
                    a.exact_match |= item.exact_match;
                }
                None => annotations.push(Annotation {
                    iri: annotation_iri(acc, &release.release_id, &run.dataset, field, &item.term_id),
                    field: field.clone(),
                    values: vec![item.value.clone()],
                    term_uri: item.term_uri.clone(),
                    exact_match: item.exact_match,
                }),
            }
            if let Some(t) = terms.register(&item.term_uri, &item.label) {
                term_outs.push(t);
            }
        }
    }

    let record = Record {
        biosample_iri: format!("{}{}", IDORG_BIOSAMPLE, acc),
        run_iri: release.run_iri(&run.run_name),
        annotations,
        terms: term_outs,
    };
    Ok((record, dropped))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p insdc-rdf-bsllmner`
Expected: 9 (meta) + 2 (model) + 11 (annotate) = 22 passed.

- [ ] **Step 5: Lint and commit**

```bash
cargo fmt --all && cargo clippy --workspace -- -D warnings
git add crates/bsllmner
git commit -m "feat(bsllmner): build annotation records from result entries

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Serializers (N-Triples, Turtle, JSON-LD)

**Files:**
- Create: `crates/bsllmner/src/serializer/mod.rs`, `…/ntriples.rs`, `…/turtle.rs`, `…/jsonld.rs`
- Modify: `crates/bsllmner/src/lib.rs` (add `pub mod serializer;`)

**Interfaces:**
- Consumes: `meta::{CrateMeta, CITATION}`, `model::Record`, `insdc_rdf_core::prefix::*` (including `PROV`), `insdc_rdf_core::escape::{escape_ntriples_string, escape_turtle_string}`
- Produces:
  - `trait Serializer { write_header, write_provenance(&self, &mut W, &CrateMeta), write_record(&self, &mut W, &Record), write_footer }`, every method returning `std::io::Result<()>`
  - `NTriplesSerializer::new()`, `TurtleSerializer::new()`, `JsonLdSerializer::new()`
  - For `JsonLdSerializer`, `write_header` writes `[` and resets the comma state; `write_footer` writes `\n]\n`.

- [ ] **Step 1: Write the trait and the failing tests**

`crates/bsllmner/src/serializer/mod.rs`:

```rust
pub mod jsonld;
pub mod ntriples;
pub mod turtle;

use std::io::Write;

use crate::meta::CrateMeta;
use crate::model::Record;

pub trait Serializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
    fn write_provenance<W: Write>(&self, writer: &mut W, meta: &CrateMeta)
        -> std::io::Result<()>;
    fn write_record<W: Write>(&self, writer: &mut W, record: &Record) -> std::io::Result<()>;
    fn write_footer<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
}

#[cfg(test)]
pub(crate) mod test_data {
    use crate::meta::{CrateMeta, Release, Run};
    use crate::model::{Annotation, Record, TermOut};

    pub const NODE: &str =
        "http://ddbj.nig.ac.jp/biosample/SAMN1#bsllmner/rel-1/d1/knockout_gene/NCBIGene_353187";
    pub const TERM: &str = "http://purl.obolibrary.org/obo/NCBIGene_353187";
    pub const RUN1: &str =
        "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/rel-1/#run-r1";
    pub const REL: &str = "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/rel-1/";

    pub fn meta() -> CrateMeta {
        CrateMeta {
            release: Release {
                release_id: "rel-1".into(),
                name: "Release \"one\"".into(),
                date_published: "2026-09-26".into(),
                license: "https://creativecommons.org/licenses/by/4.0/".into(),
            },
            runs: vec![Run {
                run_name: "r1".into(),
                dataset: "d1".into(),
                model: "mistral-small3.1:24b".into(),
                first_start: "2026-05-30T17:20:09+00:00".into(),
                last_end: "2026-05-30T22:16:22+00:00".into(),
                commit_urls: vec!["https://example.org/c1".into(), "https://example.org/c2".into()],
                result_file: "f.json".into(),
            }],
        }
    }

    pub fn record_with(values: Vec<&str>, label: &str) -> Record {
        Record {
            biosample_iri: "http://identifiers.org/biosample/SAMN1".into(),
            run_iri: RUN1.into(),
            annotations: vec![Annotation {
                iri: NODE.into(),
                field: "knockout_gene".into(),
                values: values.into_iter().map(String::from).collect(),
                term_uri: TERM.into(),
                exact_match: true,
            }],
            terms: vec![TermOut { term_uri: TERM.into(), declare_type: true, label: label.into() }],
        }
    }

    pub fn record() -> Record {
        record_with(vec!["REV-ERB a", "REV-ERB b"], "Nr1d1")
    }

    pub fn render<S: super::Serializer>(ser: &S, meta: Option<&CrateMeta>, records: &[Record]) -> String {
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        if let Some(m) = meta {
            ser.write_provenance(&mut buf, m).unwrap();
        }
        for r in records {
            ser.write_record(&mut buf, r).unwrap();
        }
        ser.write_footer(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }
}
```

`crates/bsllmner/src/serializer/ntriples.rs`, tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::serializer::test_data::*;

    #[test]
    fn test_record_triples() {
        let out = render(&NTriplesSerializer::new(), None, &[record()]);
        let expected = format!(
            "<http://identifiers.org/biosample/SAMN1> <http://schema.org/additionalProperty> <{n}> .\n\
             <{n}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://schema.org/PropertyValue> .\n\
             <{n}> <http://schema.org/name> \"knockout_gene\" .\n\
             <{n}> <http://schema.org/propertyID> \"knockout_gene\" .\n\
             <{n}> <http://schema.org/value> \"REV-ERB a\" .\n\
             <{n}> <http://schema.org/value> \"REV-ERB b\" .\n\
             <{n}> <http://schema.org/valueReference> <{t}> .\n\
             <{n}> <http://ddbj.nig.ac.jp/ontologies/biosample/exactMatch> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n\
             <{n}> <http://www.w3.org/ns/prov#wasGeneratedBy> <{r}> .\n\
             <{t}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://schema.org/DefinedTerm> .\n\
             <{t}> <http://www.w3.org/2000/01/rdf-schema#label> \"Nr1d1\" .\n",
            n = NODE,
            t = TERM,
            r = RUN1
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn test_label_only_term() {
        let mut rec = record();
        rec.terms[0].declare_type = false;
        let out = render(&NTriplesSerializer::new(), None, &[rec]);
        assert!(!out.contains("DefinedTerm"));
        assert!(out.contains(&format!("<{}> <http://www.w3.org/2000/01/rdf-schema#label> \"Nr1d1\" .", TERM)));
    }

    #[test]
    fn test_provenance_triples() {
        let out = render(&NTriplesSerializer::new(), Some(&meta()), &[]);
        assert!(out.contains(&format!("<{}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://schema.org/Dataset> .", REL)));
        assert!(out.contains(&format!("<{}> <http://schema.org/name> \"Release \\\"one\\\"\" .", REL)));
        assert!(out.contains(&format!("<{}> <http://schema.org/version> \"rel-1\" .", REL)));
        assert!(out.contains(&format!("<{}> <http://schema.org/datePublished> \"2026-09-26\"^^<http://www.w3.org/2001/XMLSchema#date> .", REL)));
        assert!(out.contains(&format!("<{}> <http://schema.org/license> <https://creativecommons.org/licenses/by/4.0/> .", REL)));
        assert!(out.contains(&format!("<{}> <http://schema.org/citation> <https://doi.org/10.1101/2025.02.17.638570> .", REL)));
        assert!(out.contains(&format!("<{}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/prov#Activity> .", RUN1)));
        assert!(out.contains(&format!("<{}> <http://www.w3.org/2000/01/rdf-schema#label> \"r1\" .", RUN1)));
        assert!(out.contains(&format!("<{}> <http://www.w3.org/ns/prov#startedAtTime> \"2026-05-30T17:20:09+00:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .", RUN1)));
        assert!(out.contains(&format!("<{}> <http://www.w3.org/ns/prov#endedAtTime> \"2026-05-30T22:16:22+00:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .", RUN1)));
        assert_eq!(out.matches("<http://www.w3.org/ns/prov#wasAssociatedWith>").count(), 2);
        assert!(out.contains(&format!("<{}> <http://ddbj.nig.ac.jp/ontologies/biosample/llmModel> \"mistral-small3.1:24b\" .", RUN1)));
        assert!(out.contains(&format!("<{}> <http://schema.org/isPartOf> <{}> .", RUN1, REL)));
    }

    #[test]
    fn test_record_without_annotations_writes_nothing() {
        let mut rec = record();
        rec.annotations.clear();
        rec.terms.clear();
        assert_eq!(render(&NTriplesSerializer::new(), None, &[rec]), "");
    }
}
```

`crates/bsllmner/src/serializer/turtle.rs`, tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::serializer::test_data::*;

    #[test]
    fn test_turtle_record() {
        let out = render(&TurtleSerializer::new(), Some(&meta()), &[record()]);
        assert!(out.starts_with("@prefix schema: <http://schema.org/> ."));
        assert!(out.contains("@prefix prov: <http://www.w3.org/ns/prov#> ."));
        assert!(out.contains(&format!("<http://identifiers.org/biosample/SAMN1> schema:additionalProperty <{}> .", NODE)));
        assert!(out.contains(&format!("<{}> a schema:PropertyValue ;", NODE)));
        assert!(out.contains("    schema:value \"REV-ERB a\", \"REV-ERB b\" ;"));
        assert!(out.contains(&format!("    schema:valueReference <{}> ;", TERM)));
        assert!(out.contains("    biosample_ont:exactMatch true ;"));
        assert!(out.contains(&format!("    prov:wasGeneratedBy <{}> .", RUN1)));
        assert!(out.contains(&format!("<{}> a schema:DefinedTerm ;\n    rdfs:label \"Nr1d1\" .", TERM)));
        assert!(out.contains(&format!("<{}> a schema:Dataset ;", REL)));
        assert!(out.contains("    schema:datePublished \"2026-09-26\"^^xsd:date ;"));
        assert!(out.contains("    prov:wasAssociatedWith <https://example.org/c1>, <https://example.org/c2> ;"));
    }
}
```

`crates/bsllmner/src/serializer/jsonld.rs`, tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::serializer::test_data::*;

    #[test]
    fn test_jsonld_is_array_of_nodes() {
        let out = render(&JsonLdSerializer::new(), Some(&meta()), &[record()]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let nodes = v.as_array().unwrap();
        // release, 1 run, 1 BioSample, 1 term
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0]["@id"], REL);
        assert_eq!(nodes[0]["@type"], "schema:Dataset");
        assert_eq!(nodes[1]["@type"], "prov:Activity");
        assert_eq!(nodes[1]["prov:wasAssociatedWith"].as_array().unwrap().len(), 2);
        let pv = &nodes[2]["schema:additionalProperty"][0];
        assert_eq!(nodes[2]["@id"], "http://identifiers.org/biosample/SAMN1");
        assert_eq!(pv["@id"], NODE);
        assert_eq!(pv["schema:value"], serde_json::json!(["REV-ERB a", "REV-ERB b"]));
        assert_eq!(pv["schema:valueReference"]["@id"], TERM);
        assert_eq!(pv["biosample_ont:exactMatch"], true);
        assert_eq!(nodes[3]["@type"], "schema:DefinedTerm");
        assert_eq!(nodes[3]["rdfs:label"], "Nr1d1");
        assert!(nodes.iter().all(|n| n.get("@context").is_some()));
    }

    #[test]
    fn test_jsonld_empty_chunk_is_valid() {
        let out = render(&JsonLdSerializer::new(), None, &[]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.as_array().unwrap().is_empty());
    }
}
```

Add a cross-format escaping test to `serializer/mod.rs`, inside a new `#[cfg(test)] mod tests` after `test_data`:

```rust
#[cfg(test)]
mod tests {
    use super::jsonld::JsonLdSerializer;
    use super::ntriples::NTriplesSerializer;
    use super::turtle::TurtleSerializer;
    use super::test_data::*;

    #[test]
    fn test_escaping_in_all_formats() {
        let nasty = "say \"hi\" \\ back\nnew\ttab β-catenin";
        let rec = record_with(vec![nasty], nasty);

        let nt = render(&NTriplesSerializer::new(), Some(&meta()), &[rec.clone()]);
        for line in nt.lines() {
            assert!(line.ends_with(" ."), "unterminated: {}", line);
        }
        assert!(nt.contains("\"say \\\"hi\\\" \\\\ back\\nnew\\ttab β-catenin\""));

        let ttl = render(&TurtleSerializer::new(), Some(&meta()), &[rec.clone()]);
        assert!(ttl.contains("\"say \\\"hi\\\" \\\\ back\\nnew\\ttab β-catenin\""));
        assert!(!ttl.contains("back\nnew"));

        let js = render(&JsonLdSerializer::new(), Some(&meta()), &[rec]);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v[2]["schema:additionalProperty"][0]["schema:value"][0], nasty);
        assert_eq!(v[3]["rdfs:label"], nasty);
    }
}
```

Add `pub mod serializer;` to `crates/bsllmner/src/lib.rs`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p insdc-rdf-bsllmner serializer`
Expected: compile errors (`NTriplesSerializer`, `TurtleSerializer`, `JsonLdSerializer` not found).

- [ ] **Step 3: Implement N-Triples**

Prepend to `ntriples.rs`:

```rust
use std::io::Write;

use insdc_rdf_core::escape::escape_ntriples_string as esc;
use insdc_rdf_core::prefix::*;

use super::Serializer;
use crate::meta::{CrateMeta, CITATION};
use crate::model::Record;

#[derive(Debug, Clone, Default)]
pub struct NTriplesSerializer;

impl NTriplesSerializer {
    pub fn new() -> Self {
        NTriplesSerializer
    }
}

impl Serializer for NTriplesSerializer {
    fn write_header<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    fn write_provenance<W: Write>(&self, w: &mut W, meta: &CrateMeta) -> std::io::Result<()> {
        let rel = &meta.release;
        let r = rel.iri();
        writeln!(w, "<{}> <{}> <{}Dataset> .", r, RDF_TYPE, SCHEMA)?;
        writeln!(w, "<{}> <{}name> \"{}\" .", r, SCHEMA, esc(&rel.name))?;
        writeln!(w, "<{}> <{}version> \"{}\" .", r, SCHEMA, esc(&rel.release_id))?;
        writeln!(
            w,
            "<{}> <{}datePublished> \"{}\"^^<{}date> .",
            r,
            SCHEMA,
            esc(&rel.date_published),
            XSD
        )?;
        writeln!(w, "<{}> <{}license> <{}> .", r, SCHEMA, rel.license)?;
        writeln!(w, "<{}> <{}citation> <{}> .", r, SCHEMA, CITATION)?;
        for run in &meta.runs {
            let s = rel.run_iri(&run.run_name);
            writeln!(w, "<{}> <{}> <{}Activity> .", s, RDF_TYPE, PROV)?;
            writeln!(w, "<{}> <{}label> \"{}\" .", s, RDFS, esc(&run.run_name))?;
            writeln!(
                w,
                "<{}> <{}startedAtTime> \"{}\"^^<{}dateTime> .",
                s,
                PROV,
                esc(&run.first_start),
                XSD
            )?;
            writeln!(
                w,
                "<{}> <{}endedAtTime> \"{}\"^^<{}dateTime> .",
                s,
                PROV,
                esc(&run.last_end),
                XSD
            )?;
            for url in &run.commit_urls {
                writeln!(w, "<{}> <{}wasAssociatedWith> <{}> .", s, PROV, url)?;
            }
            writeln!(
                w,
                "<{}> <{}llmModel> \"{}\" .",
                s,
                DDBJ_BIOSAMPLE_ONT,
                esc(&run.model)
            )?;
            writeln!(w, "<{}> <{}isPartOf> <{}> .", s, SCHEMA, r)?;
        }
        Ok(())
    }

    fn write_record<W: Write>(&self, w: &mut W, rec: &Record) -> std::io::Result<()> {
        for a in &rec.annotations {
            writeln!(
                w,
                "<{}> <{}additionalProperty> <{}> .",
                rec.biosample_iri, SCHEMA, a.iri
            )?;
            writeln!(w, "<{}> <{}> <{}PropertyValue> .", a.iri, RDF_TYPE, SCHEMA)?;
            writeln!(w, "<{}> <{}name> \"{}\" .", a.iri, SCHEMA, esc(&a.field))?;
            writeln!(w, "<{}> <{}propertyID> \"{}\" .", a.iri, SCHEMA, esc(&a.field))?;
            for v in &a.values {
                writeln!(w, "<{}> <{}value> \"{}\" .", a.iri, SCHEMA, esc(v))?;
            }
            writeln!(w, "<{}> <{}valueReference> <{}> .", a.iri, SCHEMA, a.term_uri)?;
            writeln!(
                w,
                "<{}> <{}exactMatch> \"{}\"^^<{}boolean> .",
                a.iri, DDBJ_BIOSAMPLE_ONT, a.exact_match, XSD
            )?;
            writeln!(w, "<{}> <{}wasGeneratedBy> <{}> .", a.iri, PROV, rec.run_iri)?;
        }
        for t in &rec.terms {
            if t.declare_type {
                writeln!(w, "<{}> <{}> <{}DefinedTerm> .", t.term_uri, RDF_TYPE, SCHEMA)?;
            }
            writeln!(w, "<{}> <{}label> \"{}\" .", t.term_uri, RDFS, esc(&t.label))?;
        }
        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}
```

- [ ] **Step 4: Implement Turtle**

Prepend to `turtle.rs`:

```rust
use std::io::Write;

use insdc_rdf_core::escape::escape_turtle_string as esc;
use insdc_rdf_core::prefix::*;

use super::Serializer;
use crate::meta::{CrateMeta, CITATION};
use crate::model::Record;

#[derive(Debug, Clone, Default)]
pub struct TurtleSerializer;

impl TurtleSerializer {
    pub fn new() -> Self {
        TurtleSerializer
    }
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", esc(s))
}

/// Writes `<subject> po1 ;\n    po2 ;\n    poN .`
fn write_block<W: Write>(w: &mut W, subject: &str, po: &[String]) -> std::io::Result<()> {
    writeln!(w, "<{}> {} .", subject, po.join(" ;\n    "))
}

impl Serializer for TurtleSerializer {
    fn write_header<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        writeln!(w, "@prefix schema: <{}> .", SCHEMA)?;
        writeln!(w, "@prefix rdfs: <{}> .", RDFS)?;
        writeln!(w, "@prefix xsd: <{}> .", XSD)?;
        writeln!(w, "@prefix prov: <{}> .", PROV)?;
        writeln!(w, "@prefix biosample_ont: <{}> .", DDBJ_BIOSAMPLE_ONT)?;
        writeln!(w)
    }

    fn write_provenance<W: Write>(&self, w: &mut W, meta: &CrateMeta) -> std::io::Result<()> {
        let rel = &meta.release;
        write_block(
            w,
            &rel.iri(),
            &[
                "a schema:Dataset".to_string(),
                format!("schema:name {}", quoted(&rel.name)),
                format!("schema:version {}", quoted(&rel.release_id)),
                format!("schema:datePublished {}^^xsd:date", quoted(&rel.date_published)),
                format!("schema:license <{}>", rel.license),
                format!("schema:citation <{}>", CITATION),
            ],
        )?;
        for run in &meta.runs {
            let mut po = vec![
                "a prov:Activity".to_string(),
                format!("rdfs:label {}", quoted(&run.run_name)),
                format!("prov:startedAtTime {}^^xsd:dateTime", quoted(&run.first_start)),
                format!("prov:endedAtTime {}^^xsd:dateTime", quoted(&run.last_end)),
            ];
            if !run.commit_urls.is_empty() {
                let urls: Vec<String> = run.commit_urls.iter().map(|u| format!("<{}>", u)).collect();
                po.push(format!("prov:wasAssociatedWith {}", urls.join(", ")));
            }
            po.push(format!("biosample_ont:llmModel {}", quoted(&run.model)));
            po.push(format!("schema:isPartOf <{}>", rel.iri()));
            write_block(w, &rel.run_iri(&run.run_name), &po)?;
        }
        writeln!(w)
    }

    fn write_record<W: Write>(&self, w: &mut W, rec: &Record) -> std::io::Result<()> {
        if !rec.annotations.is_empty() {
            let nodes: Vec<String> = rec.annotations.iter().map(|a| format!("<{}>", a.iri)).collect();
            writeln!(
                w,
                "<{}> schema:additionalProperty {} .",
                rec.biosample_iri,
                nodes.join(", ")
            )?;
        }
        for a in &rec.annotations {
            let values: Vec<String> = a.values.iter().map(|v| quoted(v)).collect();
            write_block(
                w,
                &a.iri,
                &[
                    "a schema:PropertyValue".to_string(),
                    format!("schema:name {}", quoted(&a.field)),
                    format!("schema:propertyID {}", quoted(&a.field)),
                    format!("schema:value {}", values.join(", ")),
                    format!("schema:valueReference <{}>", a.term_uri),
                    format!("biosample_ont:exactMatch {}", a.exact_match),
                    format!("prov:wasGeneratedBy <{}>", rec.run_iri),
                ],
            )?;
        }
        for t in &rec.terms {
            let mut po = Vec::new();
            if t.declare_type {
                po.push("a schema:DefinedTerm".to_string());
            }
            po.push(format!("rdfs:label {}", quoted(&t.label)));
            write_block(w, &t.term_uri, &po)?;
        }
        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}
```

- [ ] **Step 5: Implement JSON-LD**

Prepend to `jsonld.rs`:

```rust
use std::cell::Cell;
use std::io::Write;

use insdc_rdf_core::prefix::*;
use serde_json::{json, Map, Value};

use super::Serializer;
use crate::meta::{CrateMeta, CITATION};
use crate::model::Record;

fn context() -> Value {
    json!({
        "schema": SCHEMA,
        "rdfs": RDFS,
        "xsd": XSD,
        "prov": PROV,
        "biosample_ont": DDBJ_BIOSAMPLE_ONT,
    })
}

fn id(iri: &str) -> Value {
    json!({ "@id": iri })
}

fn typed(value: &str, datatype: &str) -> Value {
    json!({ "@value": value, "@type": datatype })
}

/// Writes each chunk as one JSON array of node objects, each with its own `@context`.
#[derive(Debug)]
pub struct JsonLdSerializer {
    first: Cell<bool>,
}

impl Default for JsonLdSerializer {
    fn default() -> Self {
        JsonLdSerializer { first: Cell::new(true) }
    }
}

impl JsonLdSerializer {
    pub fn new() -> Self {
        Self::default()
    }

    fn write_node<W: Write>(&self, w: &mut W, node: Value) -> std::io::Result<()> {
        if !self.first.replace(false) {
            write!(w, ",")?;
        }
        let s = serde_json::to_string_pretty(&node).map_err(std::io::Error::other)?;
        write!(w, "\n{}", s)
    }
}

impl Serializer for JsonLdSerializer {
    fn write_header<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        self.first.set(true);
        write!(w, "[")
    }

    fn write_provenance<W: Write>(&self, w: &mut W, meta: &CrateMeta) -> std::io::Result<()> {
        let rel = &meta.release;
        self.write_node(
            w,
            json!({
                "@context": context(),
                "@id": rel.iri(),
                "@type": "schema:Dataset",
                "schema:name": rel.name,
                "schema:version": rel.release_id,
                "schema:datePublished": typed(&rel.date_published, "xsd:date"),
                "schema:license": id(&rel.license),
                "schema:citation": id(CITATION),
            }),
        )?;
        for run in &meta.runs {
            let software: Vec<Value> = run.commit_urls.iter().map(|u| id(u)).collect();
            self.write_node(
                w,
                json!({
                    "@context": context(),
                    "@id": rel.run_iri(&run.run_name),
                    "@type": "prov:Activity",
                    "rdfs:label": run.run_name,
                    "prov:startedAtTime": typed(&run.first_start, "xsd:dateTime"),
                    "prov:endedAtTime": typed(&run.last_end, "xsd:dateTime"),
                    "prov:wasAssociatedWith": software,
                    "biosample_ont:llmModel": run.model,
                    "schema:isPartOf": id(&rel.iri()),
                }),
            )?;
        }
        Ok(())
    }

    fn write_record<W: Write>(&self, w: &mut W, rec: &Record) -> std::io::Result<()> {
        if !rec.annotations.is_empty() {
            let nodes: Vec<Value> = rec
                .annotations
                .iter()
                .map(|a| {
                    json!({
                        "@id": a.iri,
                        "@type": "schema:PropertyValue",
                        "schema:name": a.field,
                        "schema:propertyID": a.field,
                        "schema:value": a.values,
                        "schema:valueReference": id(&a.term_uri),
                        "biosample_ont:exactMatch": a.exact_match,
                        "prov:wasGeneratedBy": id(&rec.run_iri),
                    })
                })
                .collect();
            self.write_node(
                w,
                json!({
                    "@context": context(),
                    "@id": rec.biosample_iri,
                    "schema:additionalProperty": nodes,
                }),
            )?;
        }
        for t in &rec.terms {
            let mut node = Map::new();
            node.insert("@context".into(), context());
            node.insert("@id".into(), Value::from(t.term_uri.as_str()));
            if t.declare_type {
                node.insert("@type".into(), Value::from("schema:DefinedTerm"));
            }
            node.insert("rdfs:label".into(), Value::from(t.label.as_str()));
            self.write_node(w, Value::Object(node))?;
        }
        Ok(())
    }

    fn write_footer<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        writeln!(w, "\n]")
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p insdc-rdf-bsllmner`
Expected: 22 + 4 (ntriples) + 1 (turtle) + 2 (jsonld) + 1 (escaping) = 30 passed.

- [ ] **Step 7: Lint and commit**

```bash
cargo fmt --all && cargo clippy --workspace -- -D warnings
git add crates/bsllmner
git commit -m "feat(bsllmner): serialize provenance and annotations as N-Triples, Turtle and JSON-LD

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Chunk writer, `run_convert`, CLI, end-to-end test

**Files:**
- Create: `crates/bsllmner/src/chunk.rs`
- Create: `tests/fixtures/bsllmner/2026-06_test-release/results/select_chipatlas_hg38_part1.json`, `…/results/select_rnaseq_human_5y_2021-08.json`
- Modify: `crates/bsllmner/src/lib.rs`, `src/main.rs`, `Cargo.toml` (root)

**Interfaces:**
- Consumes: everything from Tasks 2–4, plus `insdc_rdf_core::{progress::Progress, manifest::Manifest}`
- Produces:
  - `insdc_rdf_bsllmner::run_convert(input: &Path, output_dir: &Path, chunk_size: usize) -> anyhow::Result<()>`
  - `ChunkWriter::new(output_dir: &Path, chunk_size: usize, progress: Progress, provenance: CrateMeta) -> std::io::Result<ChunkWriter>`, with `add_record(Record)`, `record_skip()` and `finish(self)`
  - CLI `--source bsllmner`

- [ ] **Step 1: Create the fixture result files**

`tests/fixtures/bsllmner/2026-06_test-release/results/select_chipatlas_hg38_part1.json`:

```json
{
  "entries": [
    {
      "extract": {"accession": "SAMD00004141", "extracted": {"cell_line": "Hela", "chip_antigen": ["REV-ERB a", "REV-ERB b", "POLR2A"]}, "raw_output": "{}", "llm_timing": {"total_duration": 1}},
      "search_results": {"cell_line": {"Hela": [{"term_uri": "http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0030", "label": "HeLa"}]}},
      "text2term_results": {},
      "select_timings": {},
      "results": {
        "cell_line": [{"value": "Hela", "term_id": "CVCL:0030", "term_uri": "http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0030", "label": "HeLa", "exact_match": true, "reasoning": "Exact match on oboInOwl:hasRelatedSynonym"}],
        "chip_antigen": [
          {"value": "REV-ERB a", "term_id": "NCBIGene:9572", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_9572", "label": "NR1D1", "exact_match": false, "reasoning": null},
          {"value": "REV-ERB b", "term_id": "NCBIGene:9572", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_9572", "label": "NR1D1", "exact_match": false, "reasoning": null},
          {"value": "POLR2A", "term_id": "NCBIGene:5430", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_5430", "label": "POLR2A", "exact_match": true, "reasoning": "Exact match on rdfs:label"}
        ],
        "tissue": []
      },
      "ambiguous_fields": {}
    },
    {
      "extract": {"accession": "SAMN00000001", "extracted": {"tissue": "liver"}},
      "results": {"tissue": [{"value": "liver", "term_id": "UBERON:0002107", "term_uri": "http://purl.obolibrary.org/obo/UBERON_0002107", "label": "liver", "exact_match": true, "reasoning": "Exact match on rdfs:label"}]},
      "ambiguous_fields": {}
    },
    {
      "extract": {"accession": "BADACC123", "extracted": {"disease": "cancer"}},
      "results": {"disease": [{"value": "cancer", "term_id": "MONDO:0004992", "term_uri": "http://purl.obolibrary.org/obo/MONDO_0004992", "label": "cancer", "exact_match": true, "reasoning": "Exact match on rdfs:label"}]},
      "ambiguous_fields": {}
    },
    {
      "extract": {"accession": "SAMEA6161248", "extracted": {}, "raw_output": null},
      "results": {},
      "ambiguous_fields": {}
    }
  ],
  "run_metadata": {"run_name": "chipatlas_hg38_part1", "status": "completed"},
  "evaluation": null,
  "performance": {"total_input_entries": 4},
  "errors": []
}
```

`tests/fixtures/bsllmner/2026-06_test-release/results/select_rnaseq_human_5y_2021-08.json`:

```json
{
  "entries": [
    {
      "extract": {"accession": "SAMD00270091", "extracted": {"cell_line": "HepG2", "tissue": "liver", "disease": "liver cancer \"stage II\"", "drug": ["PGE2"]}},
      "results": {
        "cell_line": [{"value": "HepG2", "term_id": "CVCL:0027", "term_uri": "http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027", "label": "Hep-G2", "exact_match": true, "reasoning": "Exact match on oboInOwl:hasRelatedSynonym"}],
        "tissue": [{"value": "liver", "term_id": "UBERON:0002107", "term_uri": "http://purl.obolibrary.org/obo/UBERON_0002107", "label": "liver", "exact_match": true, "reasoning": "Exact match on rdfs:label"}],
        "disease": [{"value": "liver cancer \"stage II\"", "term_id": "MONDO:0002691", "term_uri": "http://purl.obolibrary.org/obo/MONDO_0002691", "label": "liver cancer", "exact_match": false, "reasoning": "text2term score: 0.98"}]
      },
      "ambiguous_fields": {}
    },
    {
      "extract": {"accession": "SAMN00000001", "extracted": {"tissue": "liver", "cell_type": "hepatocytes"}},
      "results": {
        "tissue": [{"value": "liver", "term_id": "UBERON:0002107", "term_uri": "http://purl.obolibrary.org/obo/UBERON_0002107", "label": "liver", "exact_match": true, "reasoning": "Exact match on rdfs:label"}],
        "cell_type": [{"value": "hepatocytes", "term_id": "CL:0000182", "term_uri": "http://purl.obolibrary.org/obo/CL_0000182", "label": "hepatocyte", "exact_match": false, "reasoning": "text2term score: 0.91"}]
      },
      "ambiguous_fields": {}
    },
    {
      "extract": {"accession": "SAMN00000003", "extracted": {"disease": "cancer"}},
      "results": {"disease": [
        {"value": "x", "term_id": "X:1", "term_uri": "not an iri", "label": "x", "exact_match": false, "reasoning": null},
        {"value": "cancer", "term_id": "MONDO:0004992", "term_uri": "http://purl.obolibrary.org/obo/MONDO_0004992", "label": "cancer", "exact_match": true, "reasoning": "Exact match on rdfs:label"}
      ]},
      "ambiguous_fields": {}
    }
  ],
  "run_metadata": {"run_name": "rnaseq_human_5y_2021-08", "status": "completed"},
  "evaluation": null,
  "performance": {"total_input_entries": 3},
  "errors": []
}
```

The `drug` field is extracted but absent from `results`, as in the real data. Expected from this fixture:
- 6 records processed and 1 skipped (`BADACC123`).
- 10 annotation nodes: SAMD00004141 → 3, SAMN00000001 (chipatlas) → 1, SAMD00270091 → 3, SAMN00000001 (rnaseq) → 2, SAMN00000003 → 1.
- 8 distinct terms and 2 runs. With chunk size 2 the output is 3 chunks.

- [ ] **Step 2: Write the failing end-to-end tests**

Change `crates/bsllmner/src/lib.rs` to:

```rust
pub mod annotate;
pub mod chunk;
pub mod meta;
pub mod model;
pub mod serializer;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/bsllmner/2026-06_test-release"
        ))
    }

    fn read_all(dir: &Path, sub: &str, ext: &str) -> String {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir.join(sub))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == ext))
            .collect();
        paths.sort();
        paths.iter().map(|p| std::fs::read_to_string(p).unwrap()).collect()
    }

    fn copy_dir(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            let to = dst.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copy_dir(&e.path(), &to);
            } else {
                std::fs::copy(e.path(), to).unwrap();
            }
        }
    }

    #[test]
    fn test_end_to_end() {
        let tmp = tempdir().unwrap();
        let out = tmp.path().join("out");
        run_convert(&fixture_dir(), &out, 2).unwrap();

        for sub in ["ttl/chunk_0002.ttl", "jsonld/chunk_0002.jsonld", "nt/chunk_0002.nt"] {
            assert!(out.join(sub).exists(), "{}", sub);
        }
        assert!(!out.join("nt/chunk_0003.nt").exists());

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["total_records"], 6);
        assert_eq!(manifest["records_skipped"], 1);
        assert_eq!(manifest["total_chunks"], 3);

        let nt = read_all(&out, "nt", "nt");
        let count = |needle: &str| nt.lines().filter(|l| l.contains(needle)).count();
        assert_eq!(count("<http://schema.org/PropertyValue> ."), 10);
        assert_eq!(count("<http://schema.org/DefinedTerm> ."), 8);
        assert_eq!(count("<http://www.w3.org/ns/prov#Activity> ."), 2);
        assert_eq!(count("<http://schema.org/Dataset> ."), 1);
        assert_eq!(count("<http://www.w3.org/ns/prov#wasAssociatedWith>"), 3);
        // A term first seen in a skipped entry is still declared, once.
        assert_eq!(
            count("<http://purl.obolibrary.org/obo/MONDO_0004992> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"),
            1
        );
        assert_eq!(
            count("<http://purl.obolibrary.org/obo/UBERON_0002107> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"),
            1
        );
        // Provenance only in the first chunk.
        let chunk0 = std::fs::read_to_string(out.join("nt/chunk_0000.nt")).unwrap();
        assert_eq!(chunk0.matches("prov#Activity> .").count(), 2);
        // One accession in two datasets keeps two node sets.
        assert_eq!(count("#bsllmner/2026-06_test-release/chipatlas_hg38/tissue/UBERON_0002107> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"), 1);
        assert_eq!(count("#bsllmner/2026-06_test-release/rnaseq_human_5y/tissue/UBERON_0002107> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"), 2);
        // Merged node keeps both values.
        assert_eq!(count("chip_antigen/NCBIGene_9572> <http://schema.org/value>"), 2);
        // Escaped quote in a value.
        assert!(nt.contains("\"liver cancer \\\"stage II\\\"\""));

        let log = std::fs::read_to_string(out.join("errors.log")).unwrap();
        assert!(log.contains("BADACC123"), "{}", log);
        assert!(log.contains("not an iri"), "{}", log);

        let results = insdc_rdf_biosample::validate::validate_directory(&out);
        assert!(results.iter().all(|r| r.errors.is_empty()), "{:?}", results);

        for i in 0..3 {
            let js = std::fs::read_to_string(out.join(format!("jsonld/chunk_{:04}.jsonld", i))).unwrap();
            let v: serde_json::Value = serde_json::from_str(&js).unwrap();
            assert!(v.is_array());
        }
    }

    #[test]
    fn test_refuses_output_dir_with_chunks() {
        let tmp = tempdir().unwrap();
        let out = tmp.path().join("out");
        run_convert(&fixture_dir(), &out, 100).unwrap();
        let err = run_convert(&fixture_dir(), &out, 100).unwrap_err();
        assert!(err.to_string().contains("already holds converted chunks"), "{}", err);
    }

    #[test]
    fn test_missing_result_file_is_named() {
        let tmp = tempdir().unwrap();
        let crate_dir = tmp.path().join("2026-06_test-release");
        copy_dir(&fixture_dir(), &crate_dir);
        std::fs::remove_file(crate_dir.join("results/select_rnaseq_human_5y_2021-08.json")).unwrap();
        let err = run_convert(&crate_dir, &tmp.path().join("out"), 100).unwrap_err();
        assert!(format!("{:#}", err).contains("select_rnaseq_human_5y_2021-08.json"), "{:#}", err);
    }

    #[test]
    fn test_input_must_be_a_directory() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("x.json");
        std::fs::write(&file, "{}").unwrap();
        assert!(run_convert(&file, &tmp.path().join("out"), 100).is_err());
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p insdc-rdf-bsllmner tests::`
Expected: compile errors (`chunk` module and `run_convert` not found).

- [ ] **Step 4: Implement `chunk.rs`**

```rust
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use insdc_rdf_core::manifest::Manifest;
use insdc_rdf_core::progress::Progress;

use crate::meta::CrateMeta;
use crate::model::Record;
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
    buffer: Vec<Record>,
    /// Written at the start of the first chunk, then cleared.
    provenance: Option<CrateMeta>,
    progress: Progress,
    progress_path: PathBuf,
}

fn write_chunk_file<S: Serializer>(
    ser: &S,
    path: &Path,
    provenance: Option<&CrateMeta>,
    records: &[Record],
) -> std::io::Result<()> {
    let mut w = BufWriter::new(File::create(path)?);
    ser.write_header(&mut w)?;
    if let Some(meta) = provenance {
        ser.write_provenance(&mut w, meta)?;
    }
    for record in records {
        ser.write_record(&mut w, record)?;
    }
    ser.write_footer(&mut w)?;
    w.flush()
}

impl ChunkWriter {
    pub fn new(
        output_dir: &Path,
        chunk_size: usize,
        progress: Progress,
        provenance: CrateMeta,
    ) -> std::io::Result<Self> {
        for sub in ["ttl", "jsonld", "nt"] {
            fs::create_dir_all(output_dir.join(sub))?;
        }
        Ok(ChunkWriter {
            output_dir: output_dir.to_path_buf(),
            chunk_size,
            turtle_ser: TurtleSerializer::new(),
            jsonld_ser: JsonLdSerializer::new(),
            ntriples_ser: NTriplesSerializer::new(),
            buffer: Vec::with_capacity(chunk_size),
            provenance: Some(provenance),
            progress,
            progress_path: output_dir.join("progress.json"),
        })
    }

    pub fn add_record(&mut self, record: Record) -> std::io::Result<()> {
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
        // Also flush when nothing was buffered but provenance was never written.
        if !self.buffer.is_empty() || self.provenance.is_some() {
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
        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(std::io::Error::other)?;
        fs::write(self.output_dir.join("manifest.json"), manifest_json)?;
        self.progress.save(&self.progress_path)?;
        Ok(())
    }

    fn flush_chunk(&mut self) -> std::io::Result<()> {
        let name = format!("chunk_{:04}", self.progress.chunks_completed);
        let provenance = self.provenance.as_ref();
        write_chunk_file(
            &self.turtle_ser,
            &self.output_dir.join("ttl").join(format!("{}.ttl", name)),
            provenance,
            &self.buffer,
        )?;
        write_chunk_file(
            &self.jsonld_ser,
            &self.output_dir.join("jsonld").join(format!("{}.jsonld", name)),
            provenance,
            &self.buffer,
        )?;
        write_chunk_file(
            &self.ntriples_ser,
            &self.output_dir.join("nt").join(format!("{}.nt", name)),
            provenance,
            &self.buffer,
        )?;

        self.provenance = None;
        self.buffer.clear();
        self.progress.chunks_completed += 1;
        self.progress.save(&self.progress_path)?;
        Ok(())
    }
}
```

- [ ] **Step 5: Implement `run_convert`**

Insert into `crates/bsllmner/src/lib.rs`, after the `pub mod` lines and before `#[cfg(test)]`:

```rust
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{bail, Context};
use md5::Digest;

use annotate::TermRegistry;
use chunk::ChunkWriter;
use insdc_rdf_core::progress::Progress;
use model::ResultFile;

/// Converts an unpacked bsllmner-mk2 RO-Crate directory to chunked RDF.
pub fn run_convert(input: &Path, output_dir: &Path, chunk_size: usize) -> anyhow::Result<()> {
    if !input.is_dir() {
        bail!(
            "--input for bsllmner must be the unpacked RO-Crate directory, got {:?}",
            input
        );
    }
    if has_existing_chunks(output_dir) {
        bail!(
            "{:?} already holds converted chunks; convert into an empty directory",
            output_dir
        );
    }
    let meta = meta::load_crate(input)?;

    fs::create_dir_all(output_dir)?;
    let error_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_dir.join("errors.log"))?;
    let mut error_log = BufWriter::new(error_log_file);

    let checksums = input.join("provenance").join("checksums.sha256");
    let source_size = fs::metadata(&checksums)
        .with_context(|| format!("{:?}", checksums))?
        .len();
    let source_md5 = compute_md5(&checksums)?;

    eprintln!(
        "Converting bsllmner release {} ({} runs) {:?} -> {:?} (chunk size: {})",
        meta.release.release_id,
        meta.runs.len(),
        input,
        output_dir,
        chunk_size
    );

    let progress = Progress::new(&input.to_string_lossy(), source_size, &source_md5);
    let release = meta.release.clone();
    let runs = meta.runs.clone();
    let mut chunk_writer = ChunkWriter::new(output_dir, chunk_size, progress, meta)?;
    let mut terms = TermRegistry::new();
    let mut records: u64 = 0;
    let mut skipped: u64 = 0;
    let mut annotations: u64 = 0;

    for run in &runs {
        let path = input.join("results").join(&run.result_file);
        let bytes = fs::read(&path)
            .with_context(|| format!("reading result file {:?} of run {}", path, run.run_name))?;
        let file: ResultFile =
            serde_json::from_slice(&bytes).with_context(|| format!("parsing {:?}", path))?;
        drop(bytes);
        for entry in &file.entries {
            match annotate::build_record(entry, run, &release, &mut terms) {
                Ok((record, dropped)) => {
                    for line in dropped {
                        writeln!(error_log, "{}", line)?;
                    }
                    annotations += record.annotations.len() as u64;
                    chunk_writer.add_record(record)?;
                    records += 1;
                }
                Err(line) => {
                    writeln!(error_log, "{}", line)?;
                    chunk_writer.record_skip();
                    skipped += 1;
                }
            }
        }
        eprintln!("  {}: {} entries", run.run_name, file.entries.len());
    }

    chunk_writer.finish()?;
    error_log.flush()?;

    eprintln!("\nConversion complete:");
    eprintln!("  Records processed: {}", records);
    eprintln!("  Records skipped:   {}", skipped);
    eprintln!("  Annotation nodes:  {}", annotations);
    eprintln!("  Distinct terms:    {}", terms.term_count());
    eprintln!("  Runs:              {}", runs.len());
    eprintln!("  Output:            {:?}", output_dir);
    Ok(())
}

fn has_existing_chunks(output_dir: &Path) -> bool {
    ["ttl", "jsonld", "nt"].iter().any(|sub| {
        fs::read_dir(output_dir.join(sub))
            .map(|mut it| it.next().is_some())
            .unwrap_or(false)
    })
}

fn compute_md5(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = md5::Md5::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}
```

- [ ] **Step 6: Wire the CLI**

Root `Cargo.toml`:
- Add `insdc-rdf-bsllmner = { path = "crates/bsllmner" }` after the `insdc-rdf-sra-experiment` dependency.
- Change `description` to `"Convert INSDC sequence archive metadata to RDF (BioSample, SRA, BioProject, SRA Experiment, bsllmner-mk2 annotations)"`.

`src/main.rs`:
- Change the doc comment above `input` from `/// Path to input file` to `/// Path to input file (for bsllmner: the unpacked RO-Crate directory)`.
- Add `Bsllmner,` after `SraExperiment,` in `enum SourceType`.
- Add a match arm after the `SourceType::SraExperiment` arm:

```rust
            SourceType::Bsllmner => {
                insdc_rdf_bsllmner::run_convert(&input, &output_dir, chunk_size)
            }
```

- [ ] **Step 7: Run all tests and a CLI smoke run**

Run: `cargo test --workspace`
Expected: all pass, with 34 in `insdc-rdf-bsllmner` (30 + 4 new).

Run:
```bash
rm -rf /tmp/claude-1001/claude-1001/-home-inutano-repos-insdc-rdf/cd9db146-76bd-47a6-ac93-459a2a6aa628/scratchpad/bsl-smoke
cargo run -q -- convert --source bsllmner --input tests/fixtures/bsllmner/2026-06_test-release \
  --output-dir /tmp/claude-1001/claude-1001/-home-inutano-repos-insdc-rdf/cd9db146-76bd-47a6-ac93-459a2a6aa628/scratchpad/bsl-smoke
cargo run -q -- validate /tmp/claude-1001/claude-1001/-home-inutano-repos-insdc-rdf/cd9db146-76bd-47a6-ac93-459a2a6aa628/scratchpad/bsl-smoke
```
Expected:
- Conversion prints `Records processed: 6`, `Records skipped:   1`, `Annotation nodes:  10`, `Distinct terms:    8`, `Runs:              2`.
- Validation ends with `0 errors`.

- [ ] **Step 8: Lint and commit**

```bash
cargo fmt --all && cargo clippy --workspace -- -D warnings
git add Cargo.toml Cargo.lock src/main.rs crates/bsllmner tests/fixtures/bsllmner
git commit -m "feat: add bsllmner source to insdc-rdf convert

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Ontology files → N-Triples script

**Files:**
- Create: `scripts/bsllmner_ontology_to_nt.py`, `tests/fixtures/bsllmner/ontology/sample.owl`

**Interfaces:**
- Produces: `python3 scripts/bsllmner_ontology_to_nt.py <ontology-dir> <out-dir>`, which writes `<out-dir>/nt/<stem>.nt` per `.owl` and `<out-dir>/manifest.json` = `{"source_dir", "triples": {file: n}, "total_triples"}`.

- [ ] **Step 1: Create the fixture**

`tests/fixtures/bsllmner/ontology/sample.owl`:

```xml
<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:oboInOwl="http://www.geneontology.org/formats/oboInOwl#">
  <owl:Class rdf:about="http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027">
    <rdfs:label>Hep-G2</rdfs:label>
    <oboInOwl:hasRelatedSynonym>HepG2</oboInOwl:hasRelatedSynonym>
  </owl:Class>
  <owl:Class rdf:about="http://purl.obolibrary.org/obo/UBERON_0002107">
    <rdfs:label>liver</rdfs:label>
    <oboInOwl:hasRelatedSynonym>iecur "jecur"</oboInOwl:hasRelatedSynonym>
  </owl:Class>
</rdf:RDF>
```

- [ ] **Step 2: Run the not-yet-existing script to see it fail**

Run: `python3 scripts/bsllmner_ontology_to_nt.py tests/fixtures/bsllmner/ontology /tmp/claude-1001/claude-1001/-home-inutano-repos-insdc-rdf/cd9db146-76bd-47a6-ac93-459a2a6aa628/scratchpad/onto-smoke`
Expected: `can't open file … No such file or directory`.

- [ ] **Step 3: Write the script**

`scripts/bsllmner_ontology_to_nt.py`:

```python
#!/usr/bin/env python3
"""Convert the ontology files shipped in a bsllmner-mk2 RO-Crate to N-Triples.

The files are converted unchanged, one .nt per .owl, into <out-dir>/nt/, so
that `insdc-rdf validate <out-dir>` and the QLever index build treat them like
any other source. They are third-party files with their own licenses; see
SOURCES.md in the bsllmner release bucket. Requires rdflib.

Usage: bsllmner_ontology_to_nt.py <crate>/ontology <out-dir>
"""
import json
import sys
from pathlib import Path

from rdflib import Graph


def main(argv):
    if len(argv) != 3:
        sys.exit(__doc__)
    src, out = Path(argv[1]), Path(argv[2])
    owls = sorted(src.glob("*.owl"))
    if not owls:
        sys.exit(f"no .owl files in {src}")
    nt_dir = out / "nt"
    if nt_dir.exists() and any(nt_dir.iterdir()):
        sys.exit(f"{nt_dir} is not empty; convert into an empty directory")
    nt_dir.mkdir(parents=True, exist_ok=True)

    counts = {}
    for owl in owls:
        graph = Graph()
        graph.parse(owl, format="xml")
        graph.serialize(destination=nt_dir / f"{owl.stem}.nt", format="nt", encoding="utf-8")
        counts[owl.name] = len(graph)
        print(f"{owl.name}: {len(graph)} triples", file=sys.stderr)

    manifest = {
        "source_dir": str(src.resolve()),
        "triples": counts,
        "total_triples": sum(counts.values()),
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main(sys.argv)
```

- [ ] **Step 4: Run it on the fixture and validate**

```bash
S=/tmp/claude-1001/claude-1001/-home-inutano-repos-insdc-rdf/cd9db146-76bd-47a6-ac93-459a2a6aa628/scratchpad/onto-smoke
rm -rf $S
python3 scripts/bsllmner_ontology_to_nt.py tests/fixtures/bsllmner/ontology $S
cat $S/manifest.json
grep -c . $S/nt/sample.nt
grep -F 'iecur \"jecur\"' $S/nt/sample.nt
cargo run -q -- validate $S
python3 scripts/bsllmner_ontology_to_nt.py tests/fixtures/bsllmner/ontology $S; echo "exit=$?"
```
Expected:
- The first run prints `sample.owl: 6 triples`, and the manifest has `"total_triples": 6`.
- `grep -c` prints `6`, and the escaped synonym line is found.
- validate reports `0 errors`.
- The second run exits non-zero with `… is not empty`.

- [ ] **Step 5: Commit**

```bash
chmod +x scripts/bsllmner_ontology_to_nt.py
git add scripts/bsllmner_ontology_to_nt.py tests/fixtures/bsllmner/ontology
git commit -m "feat: add script converting bsllmner ontology files to N-Triples

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: rdf-config schema

**Files:**
- Create: `config/bsllmner/{model,prefix,metadata,endpoint,description,sparql}.yaml`, `config/bsllmner/shape.shex`, `config/bsllmner/schema.svg`
- Modify: `config/biosample/model.yaml`, `config/biosample/prefix.yaml`, `config/biosample/sparql.yaml`, `config/biosample/shape.shex`, `config/biosample/schema.svg`

rdf-config runs from the local checkout:

```bash
RC() { (cd ~/repos/rdf-config && bundle exec ruby bin/rdf-config "$@"); }
```

- [ ] **Step 1: Write `config/bsllmner/`**

`config/bsllmner/prefix.yaml`:

```yaml
biosample: <http://ddbj.nig.ac.jp/biosample/>
biosample_ont: <http://ddbj.nig.ac.jp/ontologies/biosample/>
idorg_biosample: <http://identifiers.org/biosample/>
schema: <http://schema.org/>
prov: <http://www.w3.org/ns/prov#>
rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
rdfs: <http://www.w3.org/2000/01/rdf-schema#>
xsd: <http://www.w3.org/2001/XMLSchema#>
obo: <http://purl.obolibrary.org/obo/>
```

`config/bsllmner/model.yaml`:

```yaml
- BioSample idorg_biosample:SAMD00270091:
  - a: biosample_ont:BioSampleRecord
  - schema:additionalProperty+:
    - annotation: Annotation

- Annotation <http://ddbj.nig.ac.jp/biosample/SAMD00270091#bsllmner/2026-06_mistral-small3.1-24b-v2/rnaseq_human_5y/cell_line/CVCL_0027>:
  - a: schema:PropertyValue
  - schema:name:
    - annotation_name: "cell_line"
  - schema:propertyID:
    - annotation_field: "cell_line"
  - schema:value+:
    - annotation_value: "HepG2"
  - schema:valueReference:
    - annotation_term: Term
  - biosample_ont:exactMatch:
    - exact_match: true
  - prov:wasGeneratedBy:
    - annotation_run: Run

- Term <http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027>:
  - a: schema:DefinedTerm
  - rdfs:label+:
    - term_label: "Hep-G2"

- Run <https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2/#run-rnaseq_human_5y_2021-08>:
  - a: prov:Activity
  - rdfs:label:
    - run_name: "rnaseq_human_5y_2021-08"
  - prov:startedAtTime:
    - run_started: "2026-05-30T17:20:09+00:00"^^xsd:dateTime
  - prov:endedAtTime:
    - run_ended: "2026-05-30T22:16:22+00:00"^^xsd:dateTime
  - prov:wasAssociatedWith+:
    - run_software: <https://github.com/dbcls/bsllmner-mk2/commit/9a3828811f1ab4bac85615e0e1b2efcc6603265f>
  - biosample_ont:llmModel:
    - run_model: "mistral-small3.1:24b"
  - schema:isPartOf:
    - run_release: Release

- Release <https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2/>:
  - a: schema:Dataset
  - schema:name:
    - release_name: "Ontology-mapped named entities from ChIP-Atlas and RNA-Seq BioSample records"
  - schema:version:
    - release_version: "2026-06_mistral-small3.1-24b-v2"
  - schema:datePublished:
    - release_date: "2026-09-26"^^xsd:date
  - schema:license:
    - release_license: <https://creativecommons.org/licenses/by/4.0/>
  - schema:citation:
    - release_citation: <https://doi.org/10.1101/2025.02.17.638570>
```

`config/bsllmner/metadata.yaml`:

```yaml
title: BioSample bsllmner-mk2 annotations RDF
website: https://biosampleplus.s3.ap-northeast-1.amazonaws.com/index.html
description: Ontology terms that bsllmner-mk2 extracted from BioSample records with a large language model, attached to the records as schema:PropertyValue nodes
tags: [BioSample, ontology annotation, LLM]
provider: Database Center for Life Science
creators:
  - name: Shuya Ikeda
    affiliation: Database Center for Life Science
  - name: Hirotaka Suetake
    affiliation: Sator Inc.
  - name: Tazro Ohta
    affiliation: Database Center for Life Science
version: "2026-06_mistral-small3.1-24b-v2"
issued: 2026-09-26
licenses:
  - Attribution 4.0 International (CC BY 4.0)
```

`config/bsllmner/endpoint.yaml`:

```yaml
endpoint: http://localhost:7001
```

`config/bsllmner/description.yaml`:

```yaml
dataset:
  name: BioSample bsllmner-mk2 annotations RDF
  description: Ontology-mapped named entities (cell line, cell type, tissue, disease, drug, knockout/knockdown/overexpressed gene, ChIP antigen) for ChIP-Atlas and human/mouse RNA-Seq BioSamples, with run-level provenance
```

`config/bsllmner/sparql.yaml`:

```yaml
annotations_of_sample:
  description: Get the bsllmner annotations of a BioSample
  variables: [BioSample, annotation_field, annotation_value, annotation_term, term_label]

samples_by_term:
  description: List BioSamples annotated with an ontology term
  variables: [BioSample, annotation_field, annotation_term]

annotation_provenance:
  description: Get the run and release that produced an annotation
  variables: [Annotation, run_name, run_model, release_version]
```

- [ ] **Step 2: Check rdf-config parses the model, then generate ShEx and SVG**

```bash
RC() { (cd ~/repos/rdf-config && bundle exec ruby bin/rdf-config "$@"); }
C=$PWD/config/bsllmner
RC --config $C --senbero
RC --config $C --shex > $C/shape.shex
RC --config $C --schema > $C/schema.svg
head -20 $C/shape.shex
```
Expected:
- `--senbero` prints the class tree with Annotation, Term, Run and Release.
- `shape.shex` starts with `PREFIX` lines and has `<AnnotationShape>`.
- `schema.svg` starts with `<svg` or `<?xml`.

If rdf-config rejects a typed literal (`"…"^^xsd:dateTime`), replace it with a plain example (`2026-05-30T17:20:09+00:00` without quotes and datatype) and re-run. Check accepted forms in `~/repos/rdf-config/doc/spec.md` under "Object".

- [ ] **Step 3: Remove the BioSamplePlus placeholders from `config/biosample/`**

In `config/biosample/model.yaml`:
- Under `schema:additionalProperty*:`, delete the line `    - assigned_sample_attribute: AnnotatedSampleType`.
- In the `OriginalSampleProperty` block, delete the `schema:valueReference+:` item (2 lines) and the `prov:wasAttributedTo:` item (2 lines).
- Delete the whole `- AnnotatedSampleType …` block at the end of the file.
- Add this comment at the top of the file:

```yaml
# Ontology annotations from bsllmner-mk2 are modelled in config/bsllmner/.
```

In `config/biosample/prefix.yaml`, delete the now-unused `cellosaurus:` and `prov:` lines. First check with `grep -n 'cellosaurus:\|prov:' config/biosample/model.yaml`, which must print nothing but the comment.

In `config/biosample/sparql.yaml`, delete the `annotated_attributes:` query (3 lines plus the blank line after it).

Regenerate:

```bash
RC() { (cd ~/repos/rdf-config && bundle exec ruby bin/rdf-config "$@"); }
C=$PWD/config/biosample
RC --config $C --shex > $C/shape.shex
RC --config $C --schema > $C/schema.svg
git diff --stat config/biosample
```
Expected: `shape.shex` no longer mentions `AnnotatedSampleType` or `valueReference`, and `schema.svg` is regenerated.

- [ ] **Step 4: Commit**

```bash
git add config/bsllmner config/biosample
git commit -m "docs(config): add bsllmner rdf-config schema, drop BioSamplePlus placeholders

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Convert the full release and its ontologies

Data production only. No code changes or commits, unless a defect is found; in that case fix it in the owning task's files, with a test.

**Files:** none in the repo. Outputs: `/data3/insdc-rdf-202610/bsllmner/` and `/data3/insdc-rdf-202610/bsllmner-ontology/`.

- [ ] **Step 1: Build the release binary**

Run: `cargo build --release && ./target/release/insdc-rdf --version`
Expected: `insdc-rdf v0.3.0-…`.

- [ ] **Step 2: Convert the release**

```bash
OUT=/data3/insdc-rdf-202610
/usr/bin/time -v ./target/release/insdc-rdf convert --source bsllmner \
  --input /data1/work/bsllmner/2026-06_mistral-small3.1-24b-v2 \
  --output-dir $OUT/bsllmner --chunk-size 100000 \
  2> $OUT/logs/convert-bsllmner.log
tail -n 40 $OUT/logs/convert-bsllmner.log | grep -E 'Records|Annotation|Distinct|Runs|Maximum resident|Elapsed|Exit status'
wc -c < $OUT/bsllmner/errors.log
```
Expected, from the spec:
- `Records processed: 4189039`, `Records skipped:   0`, `Annotation nodes:  6746471`, `Distinct terms:    38737`, `Runs:              311`.
- `Exit status: 0`, Maximum resident set size below 8,000,000 kB, and `errors.log` of 0 bytes.

**If any number differs, stop.** Do not edit the expected values. Report the difference and investigate with superpowers:systematic-debugging, comparing against the profiling scripts in the scratchpad `profile/` directory.

- [ ] **Step 3: Validate and count from the N-Triples**

```bash
OUT=/data3/insdc-rdf-202610
./target/release/insdc-rdf validate $OUT/bsllmner 2>&1 | tail -1
cat $OUT/bsllmner/nt/*.nt | grep -c '<http://schema.org/PropertyValue> \.$'
cat $OUT/bsllmner/nt/*.nt | grep -c '<http://schema.org/DefinedTerm> \.$'
cat $OUT/bsllmner/nt/*.nt | grep -c '<http://www.w3.org/ns/prov#Activity> \.$'
cat $OUT/bsllmner/nt/*.nt | wc -l
```
Expected:
- validate reports `0 errors`.
- The counts are 6746471, 38737 and 311.
- Record the total triple count for the README.

- [ ] **Step 4: Convert the ontology files**

```bash
OUT=/data3/insdc-rdf-202610
/usr/bin/time -v python3 scripts/bsllmner_ontology_to_nt.py \
  /data1/work/bsllmner/2026-06_mistral-small3.1-24b-v2/ontology $OUT/bsllmner-ontology \
  2> $OUT/logs/convert-bsllmner-ontology.log
grep -E 'triples|Maximum resident|Exit status' $OUT/logs/convert-bsllmner-ontology.log
./target/release/insdc-rdf validate $OUT/bsllmner-ontology 2>&1 | tail -1
```
Expected:
- 10 lines `<file>.owl: N triples` with N > 0, and `Exit status: 0`.
- validate reports `0 errors`.
- Record `total_triples` from `$OUT/bsllmner-ontology/manifest.json` for the README.

---

### Task 9: QLever index build, trial server and validation

**Files:**
- Create: `scripts/qlever_rebuild_index.sh`, `scripts/validate_bsllmner_qlever.sh`

**Interfaces:**
- Produces:
  - `scripts/qlever_rebuild_index.sh <index-dir> <port> <nt-dir>...`. Env overrides: `QLEVER_NAME` (default `insdc-rdf`), `QLEVER_CONTAINER` (default `qlever-insdc-<port>`), `QLEVER_IMAGE`.
  - `scripts/validate_bsllmner_qlever.sh [endpoint] [release-id]`

- [ ] **Step 1: Write the index script**

`scripts/qlever_rebuild_index.sh`:

```bash
#!/bin/bash
# Build a QLever index for insdc-rdf from directories of N-Triples, then start a server.
#
# Usage: qlever_rebuild_index.sh <index-dir> <port> <nt-dir>...
#   index-dir  directory for the index (created; must not hold an index already)
#   port       port for qlever-server
#   nt-dir     one or more directories of *.nt files, mounted read-only
# Env: QLEVER_NAME (default insdc-rdf), QLEVER_CONTAINER (default qlever-insdc-<port>),
#      QLEVER_IMAGE (default docker.io/adfreiburg/qlever)
#
# Server flags (see the endpoint notes): --entrypoint bash with -u uid:gid, and -s is the
# query timeout (-t would enable the text index).
set -euo pipefail

if [ $# -lt 3 ]; then
  sed -n '2,12p' "$0"; exit 1
fi
INDEX_DIR=$(realpath -m "$1"); PORT=$2; shift 2
NAME=${QLEVER_NAME:-insdc-rdf}
CONTAINER=${QLEVER_CONTAINER:-qlever-insdc-$PORT}
IMAGE=${QLEVER_IMAGE:-docker.io/adfreiburg/qlever}

mkdir -p "$INDEX_DIR"
if compgen -G "$INDEX_DIR/$NAME.index.*" > /dev/null; then
  echo "$INDEX_DIR already holds an index named $NAME" >&2; exit 1
fi
echo '{"num-triples-per-batch": 1000000}' > "$INDEX_DIR/$NAME.settings.json"

mounts=(); files=(); i=0
for d in "$@"; do
  compgen -G "$d/*.nt" > /dev/null || { echo "no .nt files in $d" >&2; exit 1; }
  mounts+=(-v "$(realpath "$d"):/nt$i:ro"); files+=("/nt$i/*.nt"); i=$((i + 1))
done

echo "$(date -u +%FT%TZ) index build start: $INDEX_DIR <- $*"
docker run --rm -u "$(id -u):$(id -g)" -v "$INDEX_DIR:/index" "${mounts[@]}" \
  -w /index --init --entrypoint bash "$IMAGE" -c \
  "qlever-index -i $NAME -s $NAME.settings.json --vocabulary-type on-disk-compressed \
     -f <(cat ${files[*]}) -g - -F nt -p false --stxxl-memory 10G > $NAME.index-log.txt 2>&1"
echo "$(date -u +%FT%TZ) index build done"

docker rm -f "$CONTAINER" 2>/dev/null || true
docker run -d --name "$CONTAINER" --restart unless-stopped -u "$(id -u):$(id -g)" \
  -p "$PORT:$PORT" -v "$INDEX_DIR:/index" -w /index --entrypoint bash "$IMAGE" -c \
  "qlever-server -i $NAME -p $PORT -m 20G -s 300s > $NAME.server-log.txt 2>&1"
echo "QLever server $CONTAINER started on port $PORT"
```

- [ ] **Step 2: Smoke-test the script on the fixture output**

```bash
S=/tmp/claude-1001/claude-1001/-home-inutano-repos-insdc-rdf/cd9db146-76bd-47a6-ac93-459a2a6aa628/scratchpad
chmod +x scripts/qlever_rebuild_index.sh
QLEVER_CONTAINER=qlever-smoke bash scripts/qlever_rebuild_index.sh $S/qlever-smoke 7099 $S/bsl-smoke/nt $S/onto-smoke/nt
sleep 10
curl -s http://localhost:7099 --data-urlencode 'query=SELECT (COUNT(*) AS ?n) WHERE { ?s ?p ?o }' --data-urlencode action=tsv_export
bash scripts/qlever_rebuild_index.sh $S/qlever-smoke 7099 $S/bsl-smoke/nt; echo "exit=$?"
docker rm -f qlever-smoke
```
Expected:
- The count equals `cat $S/bsl-smoke/nt/*.nt $S/onto-smoke/nt/*.nt | sort -u | wc -l`.
- The second run exits 1 with `already holds an index`.

- [ ] **Step 3: Write the validation script**

`scripts/validate_bsllmner_qlever.sh`:

```bash
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
```

Commit both scripts:

```bash
chmod +x scripts/validate_bsllmner_qlever.sh
git add scripts/qlever_rebuild_index.sh scripts/validate_bsllmner_qlever.sh
git commit -m "feat: add QLever index rebuild and bsllmner validation scripts

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 4: Compute expected SRA counts while the disk is idle**

```bash
OUT=/data3/insdc-rdf-202610; IN=/data1/work/insdc-rdf-202610/input
awk -F'\t' 'NR>1{c[$7]++} END{for(k in c) print k"\t"c[k]}' $IN/SRA_Accessions.tab | sort > $OUT/logs/expected-sra-types.tsv
cat $OUT/logs/expected-sra-types.tsv
{ awk -F'\t' 'NR>1 && $7=="EXPERIMENT"{print "<http://identifiers.org/insdc.sra/"$1">"}' $IN/SRA_Accessions.tab
  cat $OUT/sra-experiment/nt/*.nt | grep '<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ddbj.nig.ac.jp/ontologies/dra/Experiment> \.$' | cut -d' ' -f1
} | sort -u -S 8G | wc -l > $OUT/logs/expected-experiment-union.txt
cat $OUT/logs/expected-experiment-union.txt
for s in bioproject biosample sra sra-experiment bsllmner bsllmner-ontology; do echo "$s $(cat $OUT/$s/nt/*.nt | wc -l)"; done | tee $OUT/logs/triples-per-source.txt
```
Expected: a count per `Type` (RUN, EXPERIMENT, SAMPLE, STUDY, SUBMISSION, ANALYSIS), one union number, and triple counts per source. Keep all three files for Step 7 and the README.

- [ ] **Step 5: Check disk space, then start the build in the background**

```bash
df -h /data1 /data3
```
- Proceed on `/data1` only if it has ≥ 350 GB available.
- Otherwise use `/data3/qlever-insdc-202610` as the index dir in the command below, and say so in the report.

```bash
OUT=/data3/insdc-rdf-202610
QLEVER_CONTAINER=qlever-insdc-202610 nohup bash scripts/qlever_rebuild_index.sh \
  /data1/work/qlever-insdc-202610 7011 \
  $OUT/bioproject/nt $OUT/biosample/nt $OUT/sra/nt $OUT/sra-experiment/nt \
  $OUT/bsllmner/nt $OUT/bsllmner-ontology/nt \
  > $OUT/logs/qlever-index-202610.log 2>&1 &
```

Monitor without polling tightly. Every hour or so, check:

```bash
tail -n 3 /data1/work/qlever-insdc-202610/insdc-rdf.index-log.txt; df -h /data1 | tail -1
```

Expected:
- The previous build took about 22 h on HDD, so expect many hours.
- `qlever-index-202610.log` contains `index build done` followed by `QLever server qlever-insdc-202610 started on port 7011`.
- `grep -c -i error /data1/work/qlever-insdc-202610/insdc-rdf.index-log.txt` prints `0`.
- If free space on the index disk drops below 30 GB, stop the container (`docker ps` → `docker stop <id>`) and report.

- [ ] **Step 6: Run the validation queries on the trial server**

```bash
sleep 15
bash scripts/validate_sparql_qlever.sh http://localhost:7011 | tee /data3/insdc-rdf-202610/logs/validate-7011.txt
bash scripts/validate_bsllmner_qlever.sh http://localhost:7011 | tee /data3/insdc-rdf-202610/logs/validate-bsllmner-7011.txt
```

- [ ] **Step 7: Compare against expectations**

| Check | Expected |
|---|---|
| `biosample_ont:BioSampleRecord` | 60,144,760 |
| `bioproject_ont:BioProjectRecord` | 1,124,118 |
| `dra_ont:Run` / `Sample` / `Study` / `Submission` / `Analysis` | the RUN / SAMPLE / STUDY / SUBMISSION / ANALYSIS lines of `expected-sra-types.tsv` |
| `dra_ont:Experiment` | `expected-experiment-union.txt` |
| bsllmner runs / annotation nodes / DefinedTerms | 311 / 6,746,471 / 38,737 |
| Ontology join | > 0 |
| Missing BioSamples | reported as is |

Any mismatch (other than "missing BioSamples"): stop and report. Do not proceed to Task 11.

---

### Task 10: README

**Files:**
- Modify: `README.md`. This is on the `bsllmner-rdf` branch; the main checkout's uncommitted README edit is not part of it. Do not touch the QLever `docker run` snippet those uncommitted edits change.

- [ ] **Step 1: Edit the README**

Make these changes, using the numbers recorded in Tasks 8–9 (`logs/convert-*.log`, `triples-per-source.txt`, `validate-*.txt`):

1. **Intro** — "for four data sources" becomes "for four INSDC data sources, plus ontology annotations from bsllmner-mk2". Add a bullet:
   `- **bsllmner-mk2 annotations** — ontology terms extracted from BioSample records by an LLM, from a [BioSample Plus](https://biosampleplus.s3.ap-northeast-1.amazonaws.com/index.html) RO-Crate release`
2. **Summary table** — replace the rows with the 2026-10 numbers:
   - Records: from the manifests.
   - Triples: `triples-per-source.txt`, rounded like the existing rows.
   - Conversion time: "Elapsed (wall clock)" in `/data3/insdc-rdf-202610/logs/convert-<source>.log`.
   - Add a bsllmner row.
   - Note under the table: "NCBI dumps of 2026-10-07 (SRA Experiment: 2026-09-13); bsllmner release 2026-06_mistral-small3.1-24b-v2".
3. **Usage** — add:

```bash
# bsllmner-mk2 annotations (unpacked RO-Crate release directory)
curl -O https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2.tar.gz
tar xzf 2026-06_mistral-small3.1-24b-v2.tar.gz
insdc-rdf convert --source bsllmner --input 2026-06_mistral-small3.1-24b-v2 --output-dir output/bsllmner
```

   In the options table, `--source` lists `bsllmner`, and `--input` reads "Path to input file (for `bsllmner`: the unpacked RO-Crate directory)". Add a sentence: the bsllmner converter refuses an output directory that already holds chunks.
4. **RDF schemas → new "### bsllmner-mk2 annotations" subsection** containing:
   - the `config/bsllmner/schema.svg` image, in the same `<a><img></a>` form as the others
   - the annotation and provenance Turtle examples from the spec's "RDF model" section
   - one sentence each on: IRI pattern; merging of values per term; datasets kept apart; `schema:propertyID` distinguishing these nodes from original attributes
5. **New section "### Ontologies for bsllmner annotations"**, after the schema subsection:
   - The crate's `ontology/*.owl` are not part of the converter output.
   - Loading them with the annotations is recommended: `python3 scripts/bsllmner_ontology_to_nt.py <crate>/ontology output/bsllmner-ontology`, then index `output/bsllmner-ontology/nt` alongside.
   - They provide labels, synonyms and definitions, not hierarchy (no `rdfs:subClassOf`).
   - Licenses: CC BY 4.0, except Uberon CC BY 3.0, NCBI Gene public domain, and the EFO classes in the CL subsets Apache-2.0.
6. **New section "### License and citation (bsllmner data)"** — CC-BY-4.0. Cite https://doi.org/10.1101/2025.02.17.638570 and the release id.
7. **New section "### Known limitations (bsllmner)"** — copy the three bullets from the spec's "Known limitations".
8. **Validation with QLever → Setup** — add after the existing snippet: "`scripts/qlever_rebuild_index.sh <index-dir> <port> <nt-dir>...` wraps index build and server start; `scripts/validate_bsllmner_qlever.sh <endpoint>` checks the annotation counts."
9. **Record counts table** — replace it with the 2026-10 numbers from `validate-7011.txt`. Add `schema:DefinedTerm` and `prov:Activity` rows. Update the sentence "All counts match …" to say what was compared.
10. **Project structure** — add `    bsllmner/       bsllmner-mk2 RO-Crate parser + serializers` and `    sra-experiment/ SRA experiment XML parser + serializers` if it is missing.
11. **Roadmap** — replace the `bsllmner-mk2 — LLM-based ontology annotation layer …` item with:
    `- [x] **bsllmner-mk2** — ontology annotations as `schema:PropertyValue` nodes (`--source bsllmner`, `config/bsllmner/`)`

- [ ] **Step 2: Check rendering basics**

Run: `grep -n 'bsllmner' README.md | head -40 && grep -c '<img' README.md`
Expected: each item above shows up, and the image count is 5.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: document bsllmner source and the 2026-10 refresh

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Switch port 7001 to the new index (gated on the user)

**Files:** none. This changes the live endpoint.

- [ ] **Step 1: Ask the user**

Report the Task 9 Step 7 table and ask for an explicit OK before switching. **Do not continue without it.**

- [ ] **Step 2: Switch**

```bash
docker rm -f qlever-insdc-202610
docker rm -f qlever-insdc
docker run -d --name qlever-insdc --restart unless-stopped -u "$(id -u):$(id -g)" \
  -p 7001:7001 -v /data1/work/qlever-insdc-202610:/index -w /index --entrypoint bash \
  docker.io/adfreiburg/qlever -c \
  'qlever-server -i insdc-rdf -p 7001 -m 20G -s 300s > insdc-rdf.server-log.txt 2>&1'
sleep 15
curl -s http://100.88.253.33:7001 --data-urlencode 'query=SELECT (COUNT(*) AS ?n) WHERE { ?s ?p ?o }' --data-urlencode action=tsv_export
```
Use `/data3/qlever-insdc-202610` if Task 9 built there. Expected: the new total triple count, reachable over Tailscale.

Rollback, if needed:

```bash
docker rm -f qlever-insdc
docker run -d --name qlever-insdc --restart unless-stopped -u "$(id -u):$(id -g)" \
  -p 7001:7001 -v /data2/qlever-insdc:/index -w /index --entrypoint bash \
  docker.io/adfreiburg/qlever -c \
  'qlever-server -i insdc-rdf -p 7001 -m 20G -s 300s > insdc-rdf.server-log.txt 2>&1'
```

- [ ] **Step 3: Leave old data in place**

Do not delete `/data2/qlever-insdc`, the April outputs, or `/data1/work/insdc-rdf-202610/input`. Tell the user what could be deleted and how much space each would free (`du -sh`), and let them decide.

---

## Amendment (2026-10-08): publish the RDF to S3

Spec section: "Publishing the RDF to S3". The user decided on the bucket (`biosampleplus`), two releases, all three formats, gzip per chunk, RO-Crate metadata and checksums, and new entries in `index.json`. Task 11's gate and Task 14's upload gate are put to the user in one message.

Extra global constraints for Tasks 12–14:
- Python scripts run on the system **Python 3.8**: stdlib only, with no 3.9+ syntax or APIs (no `list[str]` at runtime, no `str.removeprefix`, no `|` dict merge). Tests use pytest 8 (`python3 -m pytest scripts/tests -q`).
- Nothing is uploaded and no bucket object is changed before Task 14 Step 4 has the user's OK.

### Task 12: RDF release packager

**Files:**
- Create: `scripts/package_rdf_release.py`
- Create: `scripts/tests/test_package_rdf_release.py`
- Create: `releases/2026-10_insdc-rdf.json`
- Create: `releases/2026-06_mistral-small3.1-24b-v2_rdf.json`
- Create: `scripts/upload_rdf_release.sh`
- Modify: `README.md` (new section "Published releases", before the roadmap)

**Interfaces:**
- Produces:
  - `python3 scripts/package_rdf_release.py SPEC OUT_DIR --data-root DIR [--jobs N] [--level L]`.
  - Library function `package(spec: dict, out_dir: str, data_root: str, repo_root: str, jobs: int = 4, level: int = 6) -> dict`, which returns the index entry.
  - It writes `OUT_DIR/<id>/`, `OUT_DIR/<id>.index-entry.json` and, if `spec["tarball"]`, `OUT_DIR/<id>.tar.gz`.
  - `scripts/upload_rdf_release.sh [--dryrun] OUT_DIR RELEASE_ID`.
- Consumes:
  - Each source's converter output under `<data_root>/<source.dir>/`: `nt/`, `ttl/` and `jsonld/` chunk files, plus `manifest.json` (`total_chunks`, `total_records`, `completed_at`) and `progress.json` (`started_at`).
  - The repo's `config/<source>/` directory.

**Spec file format.** Write both files exactly as below.

`releases/2026-10_insdc-rdf.json`:

```json
{
  "release_id": "2026-10_insdc-rdf",
  "bucket": "biosampleplus",
  "base_url": "https://biosampleplus.s3.ap-northeast-1.amazonaws.com",
  "name": "INSDC BioProject, BioSample and SRA metadata as RDF",
  "description": "RDF conversion of the NCBI BioProject, BioSample and SRA metadata dumps of 2026-10-07 (SRA experiment metadata of 2026-09-13), produced with insdc-rdf: 60,144,760 BioSample, 1,124,118 BioProject, 143,187,558 SRA accession and 41,593,302 SRA experiment records as 5,709,467,473 triples, in N-Triples, Turtle and JSON-LD. The bsllmner-mk2 annotation RDF attaches to the BioSample IRIs of this release.",
  "date_published": "2026-10-08",
  "license": {"id": "https://creativecommons.org/licenses/by/4.0/", "name": "Creative Commons Attribution 4.0 International"},
  "authors": [
    {"id": "#tazro-ohta", "name": "Tazro Ohta", "affiliation": ["#dbcls", "#chiba-ai-med", "#chiba-iaar"]},
    {"id": "#hirotaka-suetake", "name": "Hirotaka Suetake", "affiliation": ["#sator"]}
  ],
  "organizations": [
    {"id": "#dbcls", "name": "Database Division for Life Science (DBCLS), BioData Science Initiative, National Institute of Genetics, Research Organization of Information and Systems"},
    {"id": "#chiba-ai-med", "name": "Department of Artificial Intelligence Medicine, Graduate School of Medicine, Chiba University"},
    {"id": "#chiba-iaar", "name": "Institute for Advanced Academic Research, Chiba University"},
    {"id": "#sator", "name": "Sator Inc."}
  ],
  "software": {
    "name": "insdc-rdf",
    "description": "Converts INSDC metadata dumps (BioSample, BioProject, SRA) and bsllmner-mk2 releases to RDF.",
    "version": "v0.3.0",
    "commit": "c3ba3b06db21690760316ca9148299e9fe317c6c",
    "repository": "https://github.com/inutano/insdc-rdf"
  },
  "inputs": [
    {"id": "https://ftp.ncbi.nlm.nih.gov/bioproject/bioproject.xml", "type": "File", "name": "bioproject.xml", "content_size": 4135547517, "date_modified": "2026-10-07T16:28:28Z", "md5": "9a946342a8f6266eebadca80e9577069"},
    {"id": "https://ftp.ncbi.nlm.nih.gov/biosample/biosample_set.xml.gz", "type": "File", "name": "biosample_set.xml.gz", "content_size": 4932583454, "date_modified": "2026-10-07T16:18:15Z", "md5": "fe05127ff2390c2ee357a73371d31fc7"},
    {"id": "https://ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/SRA_Accessions.tab", "type": "File", "name": "SRA_Accessions.tab", "content_size": 35180332411, "date_modified": "2026-10-07T21:06:16Z", "md5": "56e13836b3e088dd6dc0b9f27661ff4a"},
    {"id": "https://ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/NCBI_SRA_Metadata_Full_20260913.tar.gz", "type": "File", "name": "NCBI_SRA_Metadata_Full_20260913.tar.gz", "content_size": 18233005318, "date_modified": "2026-09-14T18:35:45Z", "md5": "e127c60e8aa90f589f2e2fcaeda075a8"}
  ],
  "sources": [
    {"name": "bioproject", "title": "BioProject", "dir": "bioproject", "schema_dir": "config/bioproject", "inputs": ["https://ftp.ncbi.nlm.nih.gov/bioproject/bioproject.xml"], "expected_triples": 5772603},
    {"name": "biosample", "title": "BioSample", "dir": "biosample", "schema_dir": "config/biosample", "inputs": ["https://ftp.ncbi.nlm.nih.gov/biosample/biosample_set.xml.gz"], "expected_triples": 4031734256},
    {"name": "sra", "title": "SRA accessions", "dir": "sra", "schema_dir": "config/sra", "inputs": ["https://ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/SRA_Accessions.tab"], "expected_triples": 1014701708},
    {"name": "sra-experiment", "title": "SRA experiments", "dir": "sra-experiment", "schema_dir": "config/sra-experiment", "inputs": ["https://ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/NCBI_SRA_Metadata_Full_20260913.tar.gz"], "expected_triples": 657258906}
  ],
  "tarball": false,
  "derived_from": [
    "https://ftp.ncbi.nlm.nih.gov/bioproject/bioproject.xml",
    "https://ftp.ncbi.nlm.nih.gov/biosample/biosample_set.xml.gz",
    "https://ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/SRA_Accessions.tab",
    "https://ftp.ncbi.nlm.nih.gov/sra/reports/Metadata/NCBI_SRA_Metadata_Full_20260913.tar.gz"
  ],
  "readme": {
    "intro": [
      "This release is the RDF form of the NCBI BioProject, BioSample and SRA metadata dumps, converted with [insdc-rdf](https://github.com/inutano/insdc-rdf). The BioSample model follows the curated schema at <https://github.com/inutano/biosample_jsonld>.",
      "The bsllmner-mk2 annotation RDF, release `2026-06_mistral-small3.1-24b-v2_rdf` in this bucket, attaches ontology-mapped annotations to the BioSample IRIs of this release. Load both into one graph to query them together."
    ],
    "notes": [
      "`dra_ont:Experiment` subjects come from both `sra` (accession list) and `sra-experiment` (experiment XML); load both for the full set.",
      "Every triple is in the default graph. The chunks of one source can be loaded in any order."
    ]
  }
}
```

`releases/2026-06_mistral-small3.1-24b-v2_rdf.json`:

```json
{
  "release_id": "2026-06_mistral-small3.1-24b-v2_rdf",
  "bucket": "biosampleplus",
  "base_url": "https://biosampleplus.s3.ap-northeast-1.amazonaws.com",
  "name": "Ontology-mapped named entities from ChIP-Atlas and RNA-Seq BioSample records, as RDF",
  "description": "RDF conversion of BioSample Plus release 2026-06_mistral-small3.1-24b-v2, produced with insdc-rdf: 6,746,471 ontology-mapped annotations from 4,189,039 result entries, using 38,737 distinct terms, as schema:PropertyValue nodes on BioSample IRIs, with the 311 bsllmner-mk2 runs as prov:Activity provenance. 47,307,387 triples in N-Triples, Turtle and JSON-LD.",
  "date_published": "2026-10-08",
  "license": {"id": "https://creativecommons.org/licenses/by/4.0/", "name": "Creative Commons Attribution 4.0 International"},
  "authors": [
    {"id": "#shuya-ikeda", "name": "Shuya Ikeda", "affiliation": ["#dbcls"]},
    {"id": "#hirotaka-suetake", "name": "Hirotaka Suetake", "affiliation": ["#sator"]},
    {"id": "#tazro-ohta", "name": "Tazro Ohta", "affiliation": ["#dbcls", "#chiba-ai-med", "#chiba-iaar"]}
  ],
  "organizations": [
    {"id": "#dbcls", "name": "Database Division for Life Science (DBCLS), BioData Science Initiative, National Institute of Genetics, Research Organization of Information and Systems"},
    {"id": "#sator", "name": "Sator Inc."},
    {"id": "#chiba-ai-med", "name": "Department of Artificial Intelligence Medicine, Graduate School of Medicine, Chiba University"},
    {"id": "#chiba-iaar", "name": "Institute for Advanced Academic Research, Chiba University"}
  ],
  "software": {
    "name": "insdc-rdf",
    "description": "Converts INSDC metadata dumps (BioSample, BioProject, SRA) and bsllmner-mk2 releases to RDF.",
    "version": "v0.3.0-14-g6ea33dc",
    "commit": "6ea33dc1df3cc198b4f33fec1719999ee224768d",
    "repository": "https://github.com/inutano/insdc-rdf"
  },
  "inputs": [
    {"id": "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2/", "type": "Dataset", "name": "BioSample Plus release 2026-06_mistral-small3.1-24b-v2"}
  ],
  "sources": [
    {"name": "bsllmner", "title": "bsllmner-mk2 annotations", "dir": "bsllmner", "schema_dir": "config/bsllmner", "inputs": ["https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2/"], "expected_triples": 47307387}
  ],
  "tarball": true,
  "derived_from": ["2026-06_mistral-small3.1-24b-v2"],
  "readme": {
    "intro": [
      "This release is the RDF form of BioSample Plus release [`2026-06_mistral-small3.1-24b-v2`](https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_mistral-small3.1-24b-v2/), converted with [insdc-rdf](https://github.com/inutano/insdc-rdf). Each ontology term bsllmner-mk2 assigned to a BioSample field becomes a `schema:PropertyValue` attached to the BioSample with `schema:additionalProperty`, and each bsllmner-mk2 run becomes a `prov:Activity`.",
      "The BioSample IRIs are those of the INSDC RDF release `2026-10_insdc-rdf` in this bucket. Load both into one graph to query annotations together with the original BioSample attributes."
    ],
    "notes": [
      "Annotation nodes carry `schema:propertyID` (the field, for example `cell_line`) and no `schema:name`, so queries on the submitters' attributes (`schema:name`) are unaffected by the annotations.",
      "Term IRIs are written exactly as bsllmner-mk2 produced them, without normalization. The terms carry only `rdf:type schema:DefinedTerm` and `rdfs:label`.",
      "The ontology files the terms come from are not part of this release. Load the OWL files under `ontology/` of the source crate alongside it to query term hierarchies.",
      "Cite the bsllmner-mk2 publication, <https://doi.org/10.1101/2025.02.17.638570>, and this release ID."
    ]
  }
}
```

**`package()` behaviour.** Work in a staging directory, `OUT_DIR/<id>.partial/`, and rename it to `OUT_DIR/<id>/` only at the very end.

Before writing anything, check the following. Any failure exits non-zero with a message naming what is wrong:
- `OUT_DIR/<id>`, `OUT_DIR/<id>.partial` and, if `tarball` is set, `OUT_DIR/<id>.tar.gz` do not already exist.
- For each source:
  - `manifest.json` and `progress.json` exist.
  - Every one of `nt`, `ttl` and `jsonld` is present as a directory.
  - Each of the three directories holds exactly `manifest.total_chunks` regular files.
  - All three directories hold the same chunk stems (`chunk_0000`, …).
  - `schema_dir` exists under `repo_root`.

Then:

1. **Compress the chunks.** Gzip every chunk to `<id>.partial/<source>/<fmt>/<file>.gz` in parallel, with `concurrent.futures.ProcessPoolExecutor(max_workers=jobs)`.
   - Each worker uses `gzip.GzipFile(filename="", mode="wb", fileobj=out, compresslevel=level, mtime=0)`. Pass `filename=""` explicitly so that no FNAME field is written.
   - It streams in 8 MiB blocks, counts `b"\n"` in the input when the format is `nt`, and returns the size and sha256 of the `.gz` it wrote.
   - A source's triple count is the sum of its nt line counts.
   - If `expected_triples` is set and differs from the count, exit non-zero with `"<source>: counted N triples, expected M"`. Leave `.partial`, and say in the message that it can be removed.
2. **Copy provenance and schema files.**
   - Each source's `manifest.json` goes to `provenance/<source>.manifest.json`, byte for byte.
   - Each `schema_dir`'s regular files go to `schema/<source>/` (no subdirectories).
3. **Write `provenance/triples.tsv`:** a header `source\ttriples`, then one line per source in spec order.
4. **Write `README.md`.** This is UTF-8, generated from the spec and the counts, with these sections in order:
   - `# <name>`.
   - A line with `` Release `<id>` · published <date_published> · [<license.name>](<license.id>) ``.
   - The `readme.intro` paragraphs.
   - `## Contents`: a table with the columns Source (title), Records (`manifest.total_records`), Triples, Chunks and the gzipped size of each of nt, ttl and jsonld (human-readable, KB/MB/GB/TB with powers of 1024, one decimal place). Numbers use thousands separators (`f"{n:,}"`). After the table comes a total line, then the release layout tree from the spec.
   - `## Download`:
     - Say that no AWS account is needed.
     - If there is a tarball: `curl -O <base_url>/releases/<id>.tar.gz` and `tar xzf <id>.tar.gz`.
     - Always: one `aws s3 sync --no-sign-request s3://<bucket>/releases/<id>/ ./<id>/ --exclude "*/ttl/*" --exclude "*/jsonld/*"` example, labelled "N-Triples only", and a sentence on swapping the excludes for the other formats.
     - Always: a single-file `curl -O` example for the first source's `nt/chunk_0000.nt.gz`.
   - `## Verify`: `cd <id>` and `sha256sum -c --ignore-missing provenance/checksums.sha256`, with a sentence that `--ignore-missing` lets a partial download, such as one format only, be checked.
   - `## Load into a triplestore`:
     - Every chunk is a complete RDF document.
     - Example: `zcat */nt/*.nt.gz | <loader reading N-Triples on stdin>`.
     - Point to `scripts/qlever_rebuild_index.sh` in the insdc-rdf repository for the QLever recipe used to validate this release.
   - `## Schema`: `schema/<source>/` holds the rdf-config model, the ShEx shape (`shape.shex`), the diagram (`schema.svg`) and example SPARQL (`sparql.yaml`).
   - `## Notes`: the `readme.notes` paragraphs, as a bullet list.
   - `## Provenance`:
     - A table of the inputs: Input (a markdown link with `name` as the text and `id` as the URL), Last modified, Size (bytes with separators) and MD5. For an input with no `md5`/`content_size`/`date_modified`, show `—`.
     - Then: "Converted with insdc-rdf `<version>` ([`<commit[:7]>`](<repository>/commit/<commit>))".
     - Then the per-source conversion start and end times (from `progress.json` `started_at` and `manifest.json` `completed_at`).
   - `## License and citation`: the license line, then a suggested citation of the form `<name>, release `<id>`. <base_url>/releases/<id>/`.
   - `## Contact`: `https://github.com/inutano/insdc-rdf/issues`.
5. **Write `provenance/checksums.sha256`.** It lists `<sha256>  <relpath>` (two spaces), sorted by relpath, for every regular file in the release except `ro-crate-metadata.json` and `provenance/checksums.sha256` itself. Reuse the worker hashes for the `.gz` files.
6. **Write `ro-crate-metadata.json`.** RO-Crate 1.1: `"@context": "https://w3id.org/ro/crate/1.1/context"`, indent 2, UTF-8, `ensure_ascii=False`. Its `@graph` holds, in this order:
   - The metadata descriptor (`conformsTo` `https://w3id.org/ro/crate/1.1`, `about` `./`).
   - The root `./` `Dataset`, with:
     - `name`, `description`, `datePublished` and `license` (`{"@id": license.id}`).
     - `author`: the author ids.
     - `hasPart`: each `<source>/`, then `schema/`, `provenance/` and `README.md`.
     - `isBasedOn`: the input ids.
     - `mentions`: the `#convert-<source>` ids.
   - One `Dataset` per directory, each with `name` and `hasPart`: `<source>/` lists its three format dirs; `<source>/<fmt>/` lists its files; `schema/` lists its source dirs, and `schema/<source>/` lists its files; `provenance/` lists its files.
   - One `File` per file in the release except `ro-crate-metadata.json`. Each has `name` (the basename), `contentSize` (a decimal **string**), `encodingFormat` and `sha256`.
     - Gzipped chunks: `["application/n-triples", "application/gzip"]`, `["text/turtle", "application/gzip"]` or `["application/ld+json", "application/gzip"]`.
     - Other files by extension: `.md` `text/markdown`, `.json` `application/json`, `.tsv` `text/tab-separated-values`, `.sha256` `text/plain`, `.shex` `text/shex`, `.yaml` `application/yaml`, `.svg` `image/svg+xml`. For any other extension, `application/octet-stream`.
   - One entity per input: `@id` the input id, `@type` the input type, plus `name`, `contentSize` (a string, when `content_size` is given) and `dateModified` (when `date_modified` is given).
   - The software: `@id` `#<name>-<commit[:7]>`, `@type` `SoftwareApplication`, plus `name`, `description`, `version`, `softwareVersion` (the full commit) and `url` (`<repository>/commit/<commit>`).
   - One `CreateAction` per source: `@id` `#convert-<source>`, `name` `insdc-rdf convert --source <source>`, `instrument` (the software id), `object` (the source's input ids), `result` (`<source>/`), `startTime`, `endTime` and `actionStatus` `http://schema.org/CompletedActionStatus`.
   - The `Person` entities, each with `affiliation` as a list of `{"@id": ...}`.
   - The `Organization` entities.
   - The license: `@id` license.id, `@type` `CreativeWork`, `name` license.name.
7. **Rename** `.partial` to `<id>`.
8. **Tarball.** If `tarball` is set, write `OUT_DIR/<id>.tar.gz` with `tarfile.open(..., "w:gz")`, adding the release directory under the archive name `<id>`. Members must start with `<id>/`, as in the crate tarball.
9. **Index entry.** Write `OUT_DIR/<id>.index-entry.json` (indent 2) and return it as a dict, with keys in this order:
   - `release_id`, `kind` (`"rdf"`) and `prefix` (`releases/<id>/`).
   - `tarball` (`releases/<id>.tar.gz` or `null`).
   - `name`, `description`, `date_published`, `license` (license.id), `authors` (the names), `ro_crate_profile` (`https://w3id.org/ro/crate/1.1`), `formats` (`["nt","ttl","jsonld"]`), `triple_count` (the sum over sources) and `derived_from`.
   - `file_count`: every regular file under `<id>/`, including `ro-crate-metadata.json`.
   - `total_bytes`: the sum of their sizes.
   - `tarball_bytes`: only when there is a tarball.

`main()` parses the CLI. `repo_root` is the parent of the `scripts/` directory containing the script. `main()` prints a summary (files, bytes, triples per source) and exits 0, or prints the error to stderr and exits 1.

**`scripts/upload_rdf_release.sh [--dryrun] OUT_DIR RELEASE_ID`.**
- Use `set -euo pipefail`. The bucket comes from `BUCKET` (default `biosampleplus`), and the profile from the usual `AWS_PROFILE`.
- **Refusals:**
  - Refuse if `aws s3 ls "s3://$BUCKET/releases/$ID/"` lists anything. Note the trailing slash: without it, `…v2` would match `…v2_rdf`.
  - Refuse if there is a local tarball and `aws s3 ls "s3://$BUCKET/releases/$ID.tar.gz"` finds it remotely.
  - Refuse if `OUT_DIR/$ID/ro-crate-metadata.json` is missing.
- **Upload** with `aws s3 cp --recursive "$OUT_DIR/$ID/" "s3://$BUCKET/releases/$ID/" --exclude "*" --include <pattern> --content-type <type> --cache-control "public, max-age=86400" --no-progress`, once per group:

  | Pattern | Content-Type |
  |---|---|
  | `*.gz` | `application/gzip` |
  | `*.json` | `application/json` |
  | `*.md` | `text/markdown; charset=utf-8` |
  | `*.sha256`, `*.tsv`, `*.shex`, `*.yaml` | `text/plain; charset=utf-8` |
  | `*.svg` | `image/svg+xml` |

- **Before uploading,** fail if any local file under `OUT_DIR/$ID/` matches none of the patterns. Check with `find` against the same extension list.
- **Tarball:** upload the tarball last, if there is one, with `application/gzip` and the same Cache-Control.
- **After uploading** (skipped with `--dryrun`), compare the local file count and total bytes with `aws s3 ls --recursive --summarize "s3://$BUCKET/releases/$ID/"`, and exit 1 on a mismatch.
- **`--dryrun`** passes `--dryrun` to every `aws s3 cp` and still runs the refusal checks.
- **Not in this script:** the catalog files (`index.json` etc.). Those are uploaded in Task 14.

**README section "Published releases"** (insert before the roadmap). Keep it short:
- The two releases, with their IDs and a link to `https://biosampleplus.s3.ap-northeast-1.amazonaws.com/index.html`.
- One sentence each on what they contain.
- How to produce a release: `python3 scripts/package_rdf_release.py releases/<id>.json <out-dir> --data-root <converter output root>`, then `bash scripts/upload_rdf_release.sh <out-dir> <id>`.
- That a new NCBI dump becomes a new `releases/YYYY-MM_insdc-rdf.json` and a new release, because releases are immutable.

- [ ] **Step 1: Write the failing tests** in `scripts/tests/test_package_rdf_release.py`.

  Import the module by path, using `importlib.util.spec_from_file_location` on `scripts/package_rdf_release.py`. A fixture builder creates a temporary `data_root` with two sources:
  - Source `alpha`: 2 chunks. The nt files have 3 and 2 lines, with an escaped quote and a non-ASCII character. There are matching ttl and jsonld files. `manifest.json` has `{"total_chunks": 2, "total_records": 5, "completed_at": "2026-10-08T01:00:00+00:00"}`, and `progress.json` has `started_at`.
  - Source `beta`: 1 chunk, 4 nt lines.

  It also creates a temporary `repo_root` with `config/alpha/{model.yaml,shape.shex,schema.svg}` and `config/beta/{model.yaml,shape.shex}`, and a spec dict modelled on the files above: two authors, one organization, one File input with md5 and one Dataset input without. Tests:

  1. `test_chunks_round_trip`: every input chunk has a `.gz` at `<id>/<source>/<fmt>/<file>.gz` whose decompressed bytes equal the input.
  2. `test_gzip_is_deterministic`: two `package()` runs into different `OUT_DIR`s give byte-identical `.gz` files. Each `.gz` has header bytes 4–7 (mtime) equal to zero and FLG (byte 3) without the FNAME bit (0x08).
  3. `test_checksums_verify`: `sha256sum -c provenance/checksums.sha256`, run with `cwd=<id>`, exits 0. The list excludes `ro-crate-metadata.json` and `provenance/checksums.sha256`, and includes `README.md`, `provenance/triples.tsv`, `provenance/alpha.manifest.json` and `schema/alpha/shape.shex`.
  4. `test_ro_crate_metadata`:
     - Every file under `<id>/` except `ro-crate-metadata.json` has exactly one `File` entity, with `contentSize` (a string) and `sha256` matching the disk.
     - A `.nt.gz` entity has `encodingFormat == ["application/n-triples", "application/gzip"]`.
     - Root `hasPart` equals `[{"@id": "alpha/"}, {"@id": "beta/"}, {"@id": "schema/"}, {"@id": "provenance/"}, {"@id": "README.md"}]`.
     - `#convert-alpha` has `startTime`/`endTime` from progress/manifest and `result` `{"@id": "alpha/"}`.
     - The software `url` ends with the commit.
     - Every `{"@id": ...}` reference in the graph that starts with `#` or is a relative path resolves to an entity in the graph.
  5. `test_triple_counts`: `provenance/triples.tsv` is `source\ttriples\nalpha\t5\nbeta\t4\n`, and the returned entry has `triple_count == 9`.
  6. `test_expected_triples_mismatch_aborts`: with `expected_triples: 6` on alpha, `package()` raises `SystemExit` or a custom error whose message contains `alpha`, `5` and `6`, and no `<id>/` directory exists.
  7. `test_refuses_existing_output`: an existing `<id>/` raises, and so does an existing `<id>.partial/`, and neither is modified.
  8. `test_chunk_count_mismatch_aborts`: deleting `alpha/ttl/chunk_0001.ttl` raises an error naming `alpha` and `ttl`, before any `.partial` is created.
  9. `test_tarball`: with `tarball: true`:
     - `<id>.tar.gz` exists, every member name starts with `<id>/`, and it contains `<id>/ro-crate-metadata.json`.
     - The entry's `tarball == "releases/<id>.tar.gz"`, and its `tarball_bytes` equals the file size.

     With `tarball: false`: no tarball, `entry["tarball"] is None`, and there is no `tarball_bytes` key.
  10. `test_index_entry`:
      - Keys in the order listed above.
      - `kind == "rdf"` and `prefix == "releases/<id>/"`.
      - `file_count` equals the number of regular files under `<id>/`, and `total_bytes` equals their size sum.
      - `<id>.index-entry.json` equals the returned dict.
  11. `test_readme`: `README.md` contains the following:
      - `` `<id>` ``.
      - `9` in the Contents total.
      - `aws s3 sync --no-sign-request s3://<bucket>/releases/<id>/`.
      - `sha256sum -c --ignore-missing provenance/checksums.sha256`.
      - The md5 of the File input, and `—` for the Dataset input's size.
      - Every `readme.notes` string.
  12. `test_cli`: `subprocess.run([sys.executable, "scripts/package_rdf_release.py", spec_path, out, "--data-root", root, "--jobs", "2"])` exits 0, and an injected mismatch exits 1 with the message on stderr. Point `repo_root` at the fixture by copying the script into `<tmp_repo>/scripts/`.
  13. `test_committed_specs`: both `releases/*.json` files in the real repo load and have every required key. Each source's `schema_dir` exists in the real repo, each source's `inputs` ids are among `inputs[].id`, the `release_id` matches `^[0-9]{4}-[0-9]{2}_[A-Za-z0-9._-]+$`, and `software.commit` is 40 hex characters and an ancestor of HEAD (`git merge-base --is-ancestor`).

- [ ] **Step 2: Run the tests and see them fail**

  Run: `python3 -m pytest scripts/tests -q`
  Expected: errors importing the missing module, or the tests failing.

- [ ] **Step 3: Write `scripts/package_rdf_release.py`, the two spec files and `scripts/upload_rdf_release.sh`**

- [ ] **Step 4: Run the tests and see them pass**

  Run: `python3 -m pytest scripts/tests -q && bash -n scripts/upload_rdf_release.sh`
  Expected: 13 passed, and no syntax error.

- [ ] **Step 5: Smoke-test on real data, without writing to /data**

  ```bash
  SP=<scratchpad>
  python3 scripts/package_rdf_release.py releases/2026-06_mistral-small3.1-24b-v2_rdf.json $SP/pkg-smoke --data-root $SP/pkg-smoke-root --jobs 4
  ```

  Here `pkg-smoke-root/bsllmner` is a copy of the fixture conversion: run `./target/debug/insdc-rdf convert --source bsllmner --input tests/fixtures/bsllmner/2026-06_test-release --output-dir $SP/pkg-smoke-root/bsllmner`. With the real spec this must fail with `bsllmner: counted N triples, expected 47307387`. That proves the guard works. Then run it again with a copy of the spec whose `expected_triples` is removed, and check `sha256sum -c` in the output.

- [ ] **Step 6: Commit**

  ```bash
  git add scripts/package_rdf_release.py scripts/tests/test_package_rdf_release.py scripts/upload_rdf_release.sh releases/ README.md
  git commit -m "feat: package and upload RDF releases for the biosampleplus bucket

  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
  ```

### Task 13: Catalog files for the bucket

The bucket's `index.json`, `index.html`, `README.md` and `SOURCES.md` are not in any repository on this workstation. The originals were downloaded from the bucket into a scratch git repository (`<scratchpad>/bucket-site/`, first commit = the live files). Edit them there and commit there. Only `scripts/catalog_add_release.py` and its test are committed to the insdc-rdf branch.

**Files:**
- Create: `scripts/catalog_add_release.py`, `scripts/tests/test_catalog_add_release.py` (insdc-rdf repo)
- Modify: `<scratchpad>/bucket-site/{index.html,README.md,SOURCES.md}` (scratch repo)

**`catalog_add_release.py INDEX_JSON ENTRY_JSON... [--updated YYYY-MM-DD]`** rewrites `INDEX_JSON` in place:
- Each entry is appended to `releases`, in argument order. It refuses (exits 1, file untouched) if any `release_id` is already present.
- It sets `updated` (default: today's UTC date).
- Every existing key and entry stays as it is, in its order. Output uses indent 2, `ensure_ascii=False` and a trailing newline.

Tests:
- Appending two entries.
- Refusing a duplicate (file unchanged byte for byte).
- Leaving existing entries unchanged.
- The `--updated` flag.

**`index.html` changes.** All text goes in through `textContent` or `createTextNode`, never `innerHTML`.
- `renderRelease(r, all)` takes the full release list.
- When `r.kind === "rdf"`:
  - Metadata: Published; Triples (`num(r.triple_count)`); Formats (`r.formats.join(" · ")`); Files; Size (`bytes(r.total_bytes)`); and Archive only if `r.tarball_bytes != null`.
  - If `r.tarball`: the same archive button as the crates, then a command block of `curl -O …`, `tar xzf <id>.tar.gz` and `cd <id> && sha256sum -c provenance/checksums.sha256`.
  - Without a tarball: no button, and a command block of `aws s3 sync --no-sign-request s3://biosampleplus/<prefix> ./<id>/ --exclude "*/ttl/*" --exclude "*/jsonld/*"`, then `cd <id> && sha256sum -c --ignore-missing provenance/checksums.sha256`. The command block is preceded by the comment line `# N-Triples only; drop the excludes for all formats`.
  - The `aws s3 ls` browse block, as for crates.
  - Links: `ro-crate-metadata.json`, Release README and `checksums.sha256`. No `run_index.tsv`.
  - A "Derived from:" line before the links. An item equal to a `release_id` in `all` links to `#release-<id>`. An http(s) URL item links to itself, with its last path segment as the text. Separate items with ", ".
- Crate entries (no `kind`, or `kind !== "rdf"`) render exactly as now.
- Header lede: add one sentence, "RDF releases carry these annotations, and the INSDC BioSample, BioProject and SRA metadata they attach to, as gzipped N-Triples, Turtle and JSON-LD for loading into a triplestore."
- Add a `github.com/inutano/insdc-rdf` badge.
- "About this data": scope the tarball note to crates, and add a note that RDF chunks are gzipped one by one, so syncing only the format you need is the fast path.
- Footer: add that RDF releases are produced by [insdc-rdf](https://github.com/inutano/insdc-rdf).

**`README.md` changes.**
- Contents tree: add the RDF release layout (`<source>/<fmt>/chunk_NNNN.<fmt>.gz`, `schema/`, `provenance/{<source>.manifest.json,triples.tsv,checksums.sha256}`, `README.md`, `ro-crate-metadata.json`), stating that RDF releases have `kind: "rdf"` in `index.json`.
- Download: an "RDF releases" paragraph with the per-format sync and `sha256sum -c --ignore-missing`.
- Software: add insdc-rdf.

**`SOURCES.md` changes.**
- Add a BioProject row to "Input metadata" (NCBI / NLM, "As above").
- Add a section "## RDF releases":
  - They are produced by insdc-rdf from the NCBI dumps listed in each release's README and `ro-crate-metadata.json`.
  - The annotation RDF references ontology term IRIs and labels but does not redistribute ontology files; those stay in the crates under `ontology/`.
- Software: add `insdc-rdf (https://github.com/inutano/insdc-rdf)`. Do not state a license for it; the repository declares none. This is flagged to the user.
- Do not change the ontology table. Gaps there are reported to the user, not fixed here.

- [ ] **Step 1:** Write the failing test for `catalog_add_release.py`, run it and see it fail. Write the script, run the test and see it pass. Commit it to the insdc-rdf branch: `feat: add script that appends release entries to the bucket catalog`.
- [ ] **Step 2:** Make the `index.html`, `README.md` and `SOURCES.md` edits in the scratch repo.
- [ ] **Step 3:** Render-check `index.html` against a test catalog:
  - Build a test `index.json` in a temporary copy: the live one plus the two entries from `<scratchpad>/pkg-smoke` or hand-written ones with the same keys (one with a tarball, one without).
  - Serve it with `python3 -m http.server`, and capture it with `firefox --headless --screenshot <scratchpad>/catalog-render.png --window-size 1000,3000 http://127.0.0.1:<port>/index.html`.
  - Check the PNG shows four cards, newest first: `2026-10_insdc-rdf`, `…v2_rdf`, `…v2` and the superseded one. The INSDC card must have no archive button and must show the sync command. The crate cards must be unchanged.
  - Also check the no-JS-error path in node: extract the script, stub `document`/`fetch` minimally, and assert that `renderRelease` runs for all four entries without throwing.
- [ ] **Step 4:** Commit the edits in the scratch repo (`git -C <scratchpad>/bucket-site commit -am "Catalog: RDF releases"`).

### Task 14: Package, verify and upload (controller)

- [ ] **Step 1: Package.** Wait until the QLever index build has left its parse phase (its index log shows past "Triples parsed"), so the two jobs do not compete for `/data3` reads. Then set `date_published` in both spec files to today's date (UTC), and commit (`chore: set release dates`). Then run:

  ```bash
  OUT=/data3/insdc-rdf-202610
  python3 scripts/package_rdf_release.py releases/2026-06_mistral-small3.1-24b-v2_rdf.json $OUT/release-staging --data-root $OUT --jobs 8
  python3 scripts/package_rdf_release.py releases/2026-10_insdc-rdf.json $OUT/release-staging --data-root $OUT --jobs 8
  ```

  Expected:
  - Exit 0 for both runs.
  - Triples: 47,307,387 and 5,709,467,473. The packager checks `expected_triples` itself.
  - Sizes: about 0.5 GB and 60–70 GB.
- [ ] **Step 2: Verify.**
  - `(cd $OUT/release-staging/<id> && sha256sum -c --quiet provenance/checksums.sha256)` for both releases.
  - `gzip -t` on 5 random chunks per source.
  - `tar tzf <id>.tar.gz | head`.
  - Check that `ro-crate-metadata.json` loads with `python3 -m json.tool`.
- [ ] **Step 3: Build the catalog.**
  - Copy `<scratchpad>/bucket-site/` files to `$OUT/release-staging/bucket/`.
  - Run `python3 scripts/catalog_add_release.py $OUT/release-staging/bucket/index.json $OUT/release-staging/2026-06_mistral-small3.1-24b-v2_rdf.index-entry.json $OUT/release-staging/2026-10_insdc-rdf.index-entry.json`.
  - Render-check as in Task 13 Step 3, with the real entries.
  - `bash scripts/upload_rdf_release.sh --dryrun $OUT/release-staging <id>` for both releases.
- [ ] **Step 4: Gate.**
  - The final review is clean, and the branch has been pushed or merged per the user's choice in finishing-a-development-branch, so the commit URLs resolve.
  - Then put the Task 11 question and the upload question to the user together. The upload question shows the release sizes, file counts, catalog diff, render screenshot and the rulings to confirm (licenses, authors).
  - **No upload without an explicit OK.**
- [ ] **Step 5: Upload.**
  - `bash scripts/upload_rdf_release.sh $OUT/release-staging 2026-06_mistral-small3.1-24b-v2_rdf`, then the same for `2026-10_insdc-rdf`.
  - Then the catalog:

  ```bash
  B=s3://biosampleplus; S=$OUT/release-staging/bucket
  aws s3 cp $S/index.json  $B/index.json  --content-type application/json --cache-control "public, max-age=300"
  aws s3 cp $S/index.html  $B/index.html  --content-type "text/html; charset=utf-8" --cache-control "public, max-age=300"
  aws s3 cp $S/README.md   $B/README.md   --content-type "text/markdown; charset=utf-8" --cache-control "public, max-age=3600"
  aws s3 cp $S/SOURCES.md  $B/SOURCES.md  --content-type "text/markdown; charset=utf-8" --cache-control "public, max-age=3600"
  ```

- [ ] **Step 6: Check the live bucket.**
  - `curl -sI` a chunk, the tarball and `index.json`; check Content-Type and Cache-Control, and that there is no Content-Encoding.
  - Download one chunk anonymously and `sha256sum` it against `checksums.sha256`.
  - Load `index.html` with the headless screenshot.
- [ ] **Step 7:** Tell the user:
  - Where the edited catalog sources are: `$OUT/release-staging/bucket/`, plus the scratch repo's diff.
  - That `release-staging` can be deleted after the upload, and its size.
