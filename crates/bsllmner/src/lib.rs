pub mod annotate;
pub mod chunk;
pub mod meta;
pub mod model;
pub mod serializer;

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
        paths
            .iter()
            .map(|p| std::fs::read_to_string(p).unwrap())
            .collect()
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

        for sub in [
            "ttl/chunk_0002.ttl",
            "jsonld/chunk_0002.jsonld",
            "nt/chunk_0002.nt",
        ] {
            assert!(out.join(sub).exists(), "{}", sub);
        }
        assert!(!out.join("nt/chunk_0003.nt").exists());

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out.join("manifest.json")).unwrap())
                .unwrap();
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
        // Release title only: annotation nodes carry no schema:name.
        assert_eq!(count("<http://schema.org/name>"), 1);
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
        assert_eq!(
            count("chip_antigen/NCBIGene_9572> <http://schema.org/value>"),
            2
        );
        // Escaped quote in a value.
        assert!(nt.contains("\"liver cancer \\\"stage II\\\"\""));

        let log = std::fs::read_to_string(out.join("errors.log")).unwrap();
        assert!(log.contains("BADACC123"), "{}", log);
        assert!(log.contains("not an iri"), "{}", log);

        let results = insdc_rdf_biosample::validate::validate_directory(&out);
        assert!(results.iter().all(|r| r.errors.is_empty()), "{:?}", results);

        for i in 0..3 {
            let js =
                std::fs::read_to_string(out.join(format!("jsonld/chunk_{:04}.jsonld", i))).unwrap();
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
        assert!(
            err.to_string().contains("already holds converted chunks"),
            "{}",
            err
        );
    }

    #[test]
    fn test_missing_result_file_is_named() {
        let tmp = tempdir().unwrap();
        let crate_dir = tmp.path().join("2026-06_test-release");
        copy_dir(&fixture_dir(), &crate_dir);
        std::fs::remove_file(crate_dir.join("results/select_rnaseq_human_5y_2021-08.json"))
            .unwrap();
        let err = run_convert(&crate_dir, &tmp.path().join("out"), 100).unwrap_err();
        assert!(
            format!("{:#}", err).contains("select_rnaseq_human_5y_2021-08.json"),
            "{:#}",
            err
        );
    }

    #[test]
    fn test_input_must_be_a_directory() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("x.json");
        std::fs::write(&file, "{}").unwrap();
        assert!(run_convert(&file, &tmp.path().join("out"), 100).is_err());
    }
}
