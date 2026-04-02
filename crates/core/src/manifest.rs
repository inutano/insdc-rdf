use serde::Serialize;

/// Summary written to manifest.json when conversion finishes.
#[derive(Debug, Serialize)]
pub struct Manifest {
    pub source_file: String,
    pub source_md5: String,
    pub total_chunks: u32,
    pub total_records: u64,
    pub records_skipped: u64,
    pub completed_at: String,
}
