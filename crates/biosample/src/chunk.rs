use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use insdc_rdf_core::manifest::Manifest;
use insdc_rdf_core::progress::Progress;

use crate::model::BioSampleRecord;
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
    buffer: Vec<BioSampleRecord>,
    progress: Progress,
    progress_path: PathBuf,
}

impl ChunkWriter {
    pub fn new(output_dir: &Path, chunk_size: usize, progress: Progress) -> std::io::Result<Self> {
        fs::create_dir_all(output_dir.join("ttl"))?;
        fs::create_dir_all(output_dir.join("jsonld"))?;
        fs::create_dir_all(output_dir.join("nt"))?;

        let progress_path = output_dir.join("progress.json");

        Ok(ChunkWriter {
            output_dir: output_dir.to_path_buf(),
            chunk_size,
            turtle_ser: TurtleSerializer::new(),
            jsonld_ser: JsonLdSerializer::new(),
            ntriples_ser: NTriplesSerializer::new(),
            buffer: Vec::with_capacity(chunk_size),
            progress,
            progress_path,
        })
    }

    pub fn add_record(&mut self, record: BioSampleRecord) -> std::io::Result<()> {
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

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(std::io::Error::other)?;
        fs::write(self.output_dir.join("manifest.json"), manifest_json)?;
        self.progress.save(&self.progress_path)?;

        Ok(())
    }

    fn flush_chunk(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let chunk_num = self.progress.chunks_completed;
        let chunk_name = format!("chunk_{:04}", chunk_num);

        // TTL
        {
            let file = File::create(self.output_dir.join("ttl").join(format!("{}.ttl", chunk_name)))?;
            let mut writer = BufWriter::new(file);
            self.turtle_ser.write_header(&mut writer)?;
            for record in &self.buffer {
                self.turtle_ser.write_record(&mut writer, record)?;
            }
            self.turtle_ser.write_footer(&mut writer)?;
        }

        // JSON-LD
        {
            let file = File::create(self.output_dir.join("jsonld").join(format!("{}.jsonld", chunk_name)))?;
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
            let file = File::create(self.output_dir.join("nt").join(format!("{}.nt", chunk_name)))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Attribute, BioSampleRecord};
    use tempfile::tempdir;

    fn make_record(acc: &str) -> BioSampleRecord {
        BioSampleRecord {
            accession: acc.to_string(),
            submission_date: Some("2024-01-01T00:00:00Z".to_string()),
            last_update: None,
            publication_date: None,
            title: Some(format!("Sample {}", acc)),
            attributes: vec![Attribute {
                attribute_name: "organism".to_string(),
                harmonized_name: Some("organism".to_string()),
                display_name: None,
                value: Some("Homo sapiens".to_string()),
            }],
        }
    }

    #[test]
    fn test_creates_output_directories() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out");
        let progress = Progress::new("test", 0, "deadbeef");
        ChunkWriter::new(&out, 10, progress).unwrap();
        assert!(out.join("ttl").exists());
        assert!(out.join("jsonld").exists());
        assert!(out.join("nt").exists());
    }

    #[test]
    fn test_flushes_at_chunk_size() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out");
        let progress = Progress::new("test", 0, "deadbeef");
        let mut writer = ChunkWriter::new(&out, 3, progress).unwrap();

        for i in 0..3 {
            writer.add_record(make_record(&format!("SAMD{:08}", i))).unwrap();
        }
        assert!(out.join("ttl").join("chunk_0000.ttl").exists());
        writer.finish().unwrap();
        assert!(out.join("manifest.json").exists());
    }

    #[test]
    fn test_multiple_chunks() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out");
        let progress = Progress::new("test", 0, "deadbeef");
        let mut writer = ChunkWriter::new(&out, 3, progress).unwrap();

        for i in 0..7 {
            writer.add_record(make_record(&format!("SAMD{:08}", i))).unwrap();
        }
        writer.finish().unwrap();

        assert!(out.join("ttl").join("chunk_0000.ttl").exists());
        assert!(out.join("ttl").join("chunk_0001.ttl").exists());
        assert!(out.join("ttl").join("chunk_0002.ttl").exists());

        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(out.join("manifest.json")).unwrap()
        ).unwrap();
        assert_eq!(manifest["total_chunks"], 3);
        assert_eq!(manifest["total_records"], 7);
    }
}
