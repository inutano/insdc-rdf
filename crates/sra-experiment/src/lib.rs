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

    let mut records_processed: u64 = 0;
    let mut records_skipped: u64 = 0;
    let mut io_error: Option<std::io::Error> = None;

    parser::process_tar_archive(file, |result| {
        if io_error.is_some() {
            return; // Stop processing if we had an I/O error
        }
        match result {
            Ok(record) => {
                if let Err(e) = chunk_writer.add_record(record) {
                    io_error = Some(e);
                    return;
                }
                records_processed += 1;
                if records_processed.is_multiple_of(1_000_000) {
                    eprintln!("  Progress: {} records processed", records_processed);
                }
            }
            Err(e) => {
                let _ = writeln!(error_log, "{}", e);
                chunk_writer.record_skip();
                records_skipped += 1;
            }
        }
    })?;

    if let Some(e) = io_error {
        return Err(e.into());
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fixture_experiment_xml() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/experiment_sample.xml"
        ))
        .unwrap()
    }

    fn append_fixture<W: std::io::Write>(builder: &mut tar::Builder<W>, xml: &[u8]) {
        let mut header = tar::Header::new_gnu();
        header.set_size(xml.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "SRA000001/SRA000001.experiment.xml", xml)
            .unwrap();
    }

    fn build_test_tar_gz() -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let xml = fixture_experiment_xml();
        let enc = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(enc);
        append_fixture(&mut builder, &xml);
        let enc = builder.into_inner().unwrap();
        enc.finish().unwrap()
    }

    fn build_test_plain_tar() -> Vec<u8> {
        let xml = fixture_experiment_xml();
        let mut builder = tar::Builder::new(Vec::new());
        append_fixture(&mut builder, &xml);
        builder.into_inner().unwrap()
    }

    fn assert_end_to_end(archive_bytes: &[u8], filename: &str) {
        let dir = tempdir().unwrap();
        let input_path = dir.path().join(filename);
        std::fs::write(&input_path, archive_bytes).unwrap();

        let output_dir = dir.path().join("output");
        run_convert(&input_path, &output_dir, 100).unwrap();

        // Verify output structure
        assert!(output_dir.join("ttl/chunk_0000.ttl").exists());
        assert!(output_dir.join("jsonld/chunk_0000.jsonld").exists());
        assert!(output_dir.join("nt/chunk_0000.nt").exists());
        assert!(output_dir.join("manifest.json").exists());
        assert!(output_dir.join("progress.json").exists());

        // Verify manifest
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(output_dir.join("manifest.json")).unwrap(),
        )
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

    #[test]
    fn test_end_to_end_tar_gz() {
        assert_end_to_end(&build_test_tar_gz(), "test.tar.gz");
    }

    #[test]
    fn test_end_to_end_plain_tar() {
        assert_end_to_end(&build_test_plain_tar(), "test.tar");
    }
}
