pub mod chunk;
pub mod model;
pub mod parser;
pub mod serializer;
pub mod validate;

use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use md5::Digest;

use chunk::ChunkWriter;
use insdc_rdf_core::progress::Progress;
use parser::BioSampleParser;

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
        "Converting {:?} -> {:?} (chunk size: {})",
        input, output_dir, chunk_size
    );
    eprintln!("  File size: {} bytes, MD5: {}", source_size, source_md5);

    let progress = Progress::new(&input_str, source_size, &source_md5);
    let mut chunk_writer = ChunkWriter::new(output_dir, chunk_size, progress)?;

    let is_gzipped = input.to_string_lossy().ends_with(".gz");

    let mut records_processed: u64 = 0;
    let mut records_skipped: u64 = 0;

    if is_gzipped {
        let file = File::open(input)?;
        let gz = flate2::read::GzDecoder::new(file);
        let reader = BufReader::new(gz);
        let mut parser = BioSampleParser::new(reader);
        process_records(
            &mut parser,
            &mut chunk_writer,
            &mut error_log,
            &mut records_processed,
            &mut records_skipped,
        )?;
    } else {
        let file = File::open(input)?;
        let reader = BufReader::new(file);
        let mut parser = BioSampleParser::new(reader);
        process_records(
            &mut parser,
            &mut chunk_writer,
            &mut error_log,
            &mut records_processed,
            &mut records_skipped,
        )?;
    }

    chunk_writer.finish()?;

    eprintln!("\nConversion complete:");
    eprintln!("  Records processed: {}", records_processed);
    eprintln!("  Records skipped:   {}", records_skipped);
    eprintln!("  Output:            {:?}", output_dir);

    Ok(())
}

fn process_records<R: std::io::BufRead, W: Write>(
    parser: &mut BioSampleParser<R>,
    chunk_writer: &mut ChunkWriter,
    error_log: &mut W,
    records_processed: &mut u64,
    records_skipped: &mut u64,
) -> anyhow::Result<()> {
    loop {
        match parser.next_record() {
            Ok(Some(record)) => {
                chunk_writer.add_record(record)?;
                *records_processed += 1;
                if (*records_processed).is_multiple_of(100_000) {
                    eprintln!("  Progress: {} records processed", records_processed);
                }
            }
            Ok(None) => break,
            Err(e) => {
                writeln!(error_log, "{}", e)?;
                chunk_writer.record_skip();
                *records_skipped += 1;
            }
        }
    }
    Ok(())
}

fn compute_md5(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = md5::Md5::new();
    std::io::copy(&mut file, &mut hasher)?;
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}
