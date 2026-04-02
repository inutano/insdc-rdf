use std::io::BufRead;

use insdc_rdf_core::error::ConvertError;
use crate::model::{SraAccessionRecord, SraType};

/// Column indices in SRA_Accessions.tab (0-based).
const COL_ACCESSION: usize = 0;
const COL_SUBMISSION: usize = 1;
const COL_STATUS: usize = 2;
const COL_UPDATED: usize = 3;
const COL_PUBLISHED: usize = 4;
const COL_TYPE: usize = 6;
const COL_CENTER: usize = 7;
const COL_EXPERIMENT: usize = 10;
const COL_SAMPLE: usize = 11;
const COL_STUDY: usize = 12;
const COL_BIOSAMPLE: usize = 17;
const COL_BIOPROJECT: usize = 18;

const EXPECTED_COLUMNS: usize = 20;

/// A streaming parser for SRA_Accessions.tab (TSV format).
///
/// Reads one line at a time. Skips the header row and rows where
/// Status != "live". Call `next_record()` in a loop until it returns
/// `Ok(None)` (end of input).
pub struct SraAccessionParser<R: BufRead> {
    reader: R,
    line_buf: String,
    line_number: u64,
    header_validated: bool,
}

impl<R: BufRead> SraAccessionParser<R> {
    pub fn new(reader: R) -> Self {
        SraAccessionParser {
            reader,
            line_buf: String::new(),
            line_number: 0,
            header_validated: false,
        }
    }

    pub fn next_record(&mut self) -> Result<Option<SraAccessionRecord>, ConvertError> {
        loop {
            self.line_buf.clear();
            let bytes_read = self.reader.read_line(&mut self.line_buf)
                .map_err(ConvertError::Io)?;
            if bytes_read == 0 {
                return Ok(None); // EOF
            }
            self.line_number += 1;

            let line = self.line_buf.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }

            // Skip/validate header
            if !self.header_validated {
                if line.starts_with("Accession\t") {
                    self.header_validated = true;
                    continue;
                }
                // No header found — treat as data
                self.header_validated = true;
            }

            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < EXPECTED_COLUMNS {
                return Err(ConvertError::TsvParse {
                    line: self.line_number,
                    message: format!("expected {} columns, got {}", EXPECTED_COLUMNS, fields.len()),
                });
            }

            // Skip non-live records
            if fields[COL_STATUS] != "live" {
                continue;
            }

            let accession = fields[COL_ACCESSION];
            let type_str = fields[COL_TYPE];

            let sra_type = match SraType::parse(type_str) {
                Some(t) => t,
                None => {
                    return Err(ConvertError::TsvParse {
                        line: self.line_number,
                        message: format!("unknown SRA type: {}", type_str),
                    });
                }
            };

            return Ok(Some(SraAccessionRecord {
                accession: accession.to_string(),
                sra_type,
                submission: nonempty(fields[COL_SUBMISSION]),
                updated: nonempty(fields[COL_UPDATED]),
                published: nonempty(fields[COL_PUBLISHED]),
                center: nonempty(fields[COL_CENTER]),
                experiment: nonempty(fields[COL_EXPERIMENT]),
                sample: nonempty(fields[COL_SAMPLE]),
                study: nonempty(fields[COL_STUDY]),
                biosample: nonempty(fields[COL_BIOSAMPLE]),
                bioproject: nonempty(fields[COL_BIOPROJECT]),
            }));
        }
    }
}

/// Convert a TSV field to Option<String>, treating "-" as empty.
fn nonempty(s: &str) -> Option<String> {
    if s.is_empty() || s == "-" {
        None
    } else {
        Some(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse_all(tsv: &str) -> (Vec<SraAccessionRecord>, Vec<ConvertError>) {
        let reader = std::io::BufReader::new(Cursor::new(tsv.as_bytes().to_vec()));
        let mut parser = SraAccessionParser::new(reader);
        let mut records = Vec::new();
        let mut errors = Vec::new();
        loop {
            match parser.next_record() {
                Ok(Some(rec)) => records.push(rec),
                Ok(None) => break,
                Err(e) => errors.push(e),
            }
        }
        (records, errors)
    }

    fn sample_tsv() -> String {
        std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/sra_accessions_sample.tab")
        ).unwrap()
    }

    #[test]
    fn test_parse_fixture() {
        let (records, errors) = parse_all(&sample_tsv());
        assert_eq!(errors.len(), 0);
        // 7 data rows, but 1 is suppressed → 6 live records
        assert_eq!(records.len(), 6);
    }

    #[test]
    fn test_filters_non_live() {
        let (records, _) = parse_all(&sample_tsv());
        // SRR999999 has Status=suppressed, should be skipped
        assert!(!records.iter().any(|r| r.accession == "SRR999999"));
    }

    #[test]
    fn test_run_record_fields() {
        let (records, _) = parse_all(&sample_tsv());
        let run = records.iter().find(|r| r.accession == "DRR000001").unwrap();
        assert_eq!(run.sra_type, SraType::Run);
        assert_eq!(run.experiment.as_deref(), Some("DRX000001"));
        assert_eq!(run.sample.as_deref(), Some("DRS000001"));
        assert_eq!(run.study.as_deref(), Some("DRP000001"));
        assert_eq!(run.biosample.as_deref(), Some("SAMD00016353"));
        assert_eq!(run.bioproject.as_deref(), Some("PRJDA38027"));
    }

    #[test]
    fn test_sample_record_empty_fields() {
        let (records, _) = parse_all(&sample_tsv());
        let sample = records.iter().find(|r| r.accession == "DRS000001").unwrap();
        assert_eq!(sample.sra_type, SraType::Sample);
        assert_eq!(sample.experiment, None);
        assert_eq!(sample.study, None);
        assert_eq!(sample.biosample.as_deref(), Some("SAMD00016353"));
        assert_eq!(sample.bioproject, None);
    }

    #[test]
    fn test_types_parsed() {
        let (records, _) = parse_all(&sample_tsv());
        let types: Vec<&SraType> = records.iter().map(|r| &r.sra_type).collect();
        assert!(types.contains(&&SraType::Submission));
        assert!(types.contains(&&SraType::Study));
        assert!(types.contains(&&SraType::Run));
        assert!(types.contains(&&SraType::Sample));
        assert!(types.contains(&&SraType::Experiment));
    }
}
