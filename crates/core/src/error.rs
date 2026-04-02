use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConvertError {
    #[error("XML parse error at byte {offset}: {message}")]
    XmlParse { offset: u64, message: String },

    #[error("Record missing accession at byte {offset}")]
    MissingAccession { offset: u64 },

    #[error("TSV parse error at line {line}: {message}")]
    TsvParse { line: u64, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
