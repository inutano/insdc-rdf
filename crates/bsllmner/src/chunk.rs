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
        let manifest_json =
            serde_json::to_string_pretty(&manifest).map_err(std::io::Error::other)?;
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
            &self
                .output_dir
                .join("jsonld")
                .join(format!("{}.jsonld", name)),
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
