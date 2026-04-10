use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Cursor, Read};
use tar::Archive;

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

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
                let offset = reader.buffer_position();
                let accession = extract_attr(e, b"accession");
                match accession {
                    Some(acc) => match parse_experiment_body(&mut reader, &mut buf, acc) {
                        Ok(record) => records.push(record),
                        Err(err) => errors.push(err),
                    },
                    None => {
                        errors.push(ConvertError::MissingAccession { offset });
                        skip_to_end(&mut reader, &mut buf, b"EXPERIMENT");
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                errors.push(ConvertError::XmlParse {
                    offset: reader.buffer_position(),
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

    let mut depth: u32 = 1;
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
                    "TITLE"
                    | "DESIGN_DESCRIPTION"
                    | "LIBRARY_NAME"
                    | "LIBRARY_STRATEGY"
                    | "LIBRARY_SOURCE"
                    | "LIBRARY_SELECTION"
                    | "LIBRARY_CONSTRUCTION_PROTOCOL"
                    | "INSTRUMENT_MODEL" => {
                        current_tag = Some(tag);
                    }
                    "PLATFORM" => {
                        in_platform = true;
                    }
                    "PAIRED" => {
                        let length =
                            extract_attr(e, b"NOMINAL_LENGTH").and_then(|s| s.parse::<f64>().ok());
                        let sdev =
                            extract_attr(e, b"NOMINAL_SDEV").and_then(|s| s.parse::<f64>().ok());
                        rec.library_layout = Some(LibraryLayout::Paired {
                            nominal_length: length,
                            nominal_sdev: sdev,
                        });
                    }
                    "SINGLE" => {
                        rec.library_layout = Some(LibraryLayout::Single);
                    }
                    _ => {
                        if in_platform && platform_tag.is_none() && tag != "INSTRUMENT_MODEL" {
                            platform_tag = Some(tag);
                        }
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag.as_str() {
                    "PAIRED" => {
                        let length =
                            extract_attr(e, b"NOMINAL_LENGTH").and_then(|s| s.parse::<f64>().ok());
                        let sdev =
                            extract_attr(e, b"NOMINAL_SDEV").and_then(|s| s.parse::<f64>().ok());
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
                        if in_platform && platform_tag.is_none() && tag != "INSTRUMENT_MODEL" {
                            platform_tag = Some(tag);
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if let Some(ref tag) = current_tag {
                    let text = e
                        .unescape()
                        .map_err(|err| ConvertError::XmlParse {
                            offset: reader.buffer_position(),
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
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ConvertError::XmlParse {
                    offset: reader.buffer_position(),
                    message: e.to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(rec)
}

fn extract_attr(e: &quick_xml::events::BytesStart, name: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == name {
            return attr.unescape_value().ok().map(|v| v.to_string());
        }
    }
    None
}

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

/// Process a tar archive (optionally gzip-compressed), yielding experiment records
/// and errors through a callback. The stream is auto-detected by reading the first
/// two bytes: if they are the gzip magic `1f 8b`, the input is piped through
/// `flate2::GzDecoder`; otherwise it is treated as an uncompressed tar archive.
///
/// Reads one tar entry at a time — memory usage is proportional to the largest
/// single experiment XML file, not the entire archive.
pub fn process_tar_archive<R: Read>(
    mut reader: R,
    on_result: impl FnMut(Result<SraExperimentRecord, ConvertError>),
) -> std::io::Result<()> {
    let mut prefix = [0u8; 2];
    let mut filled = 0;
    while filled < prefix.len() {
        let n = reader.read(&mut prefix[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    let is_gzipped = filled == prefix.len() && prefix == GZIP_MAGIC;

    // Replay the bytes we peeked in front of the rest of the stream so that
    // downstream readers see the original byte sequence.
    let replay = Cursor::new(prefix[..filled].to_vec());
    let stream = replay.chain(reader);

    if is_gzipped {
        let gz = GzDecoder::new(stream);
        process_tar_entries(Archive::new(gz), on_result)
    } else {
        process_tar_entries(Archive::new(stream), on_result)
    }
}

fn process_tar_entries<R: Read>(
    mut archive: Archive<R>,
    mut on_result: impl FnMut(Result<SraExperimentRecord, ConvertError>),
) -> std::io::Result<()> {
    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?.to_string_lossy().to_string();

        if path.ends_with(".experiment.xml") {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;

            let (records, errors) = parse_experiment_xml(&bytes);
            for error in errors {
                on_result(Err(error));
            }
            for record in records {
                on_result(Ok(record));
            }
        }
    }

    Ok(())
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
        assert_eq!(r.instrument_model.as_deref(), Some("Illumina NovaSeq 6000"));
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
        let r = &records[1];
        assert_eq!(r.library_name, None);
        assert_eq!(r.library_construction_protocol, None);
    }

    #[test]
    fn test_empty_design_description() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[1];
        assert_eq!(r.design_description, None);
    }

    #[test]
    fn test_different_platform() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[2];
        assert_eq!(r.platform.as_deref(), Some("ABI_SOLID"));
        assert_eq!(r.instrument_model.as_deref(), Some("AB SOLiD System 3.0"));
    }

    #[test]
    fn test_paired_without_attributes() {
        let (records, _) = parse_experiment_xml(&fixture_xml());
        let r = &records[2];
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

    // --- Tar.gz tests ---

    fn build_tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

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
        let xml = fixture_xml();
        let tar_bytes = build_tar_gz(&[
            ("SRA000001/SRA000001.experiment.xml", &xml),
            ("SRA000001/SRA000001.run.xml", b"<RUN_SET/>"),
            ("SRA000002/SRA000002.submission.xml", b"<SUBMISSION/>"),
        ]);

        let cursor = std::io::Cursor::new(tar_bytes);
        let mut records = Vec::new();
        process_tar_archive(cursor, |result| {
            if let Ok(rec) = result {
                records.push(rec);
            }
        })
        .unwrap();

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
        let mut count = 0;
        process_tar_archive(cursor, |_| count += 1).unwrap();
        assert_eq!(count, 0);
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
        let mut records = Vec::new();
        process_tar_archive(cursor, |result| {
            if let Ok(rec) = result {
                records.push(rec);
            }
        })
        .unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].accession, "SRX100001");
        assert_eq!(records[1].accession, "ERX200001");
    }

    #[test]
    fn test_tar_gz_propagates_errors() {
        // XML with one valid experiment and one missing accession
        let xml = br#"<?xml version="1.0"?>
<EXPERIMENT_SET>
  <EXPERIMENT accession="SRX000001">
    <DESIGN><LIBRARY_DESCRIPTOR>
      <LIBRARY_STRATEGY>WGS</LIBRARY_STRATEGY><LIBRARY_SOURCE>GENOMIC</LIBRARY_SOURCE>
      <LIBRARY_SELECTION>RANDOM</LIBRARY_SELECTION><LIBRARY_LAYOUT><SINGLE/></LIBRARY_LAYOUT>
    </LIBRARY_DESCRIPTOR></DESIGN><PLATFORM><ILLUMINA><INSTRUMENT_MODEL>HiSeq</INSTRUMENT_MODEL></ILLUMINA></PLATFORM>
  </EXPERIMENT>
  <EXPERIMENT>
    <TITLE>No accession</TITLE>
  </EXPERIMENT>
</EXPERIMENT_SET>"#;

        let tar_bytes = build_tar_gz(&[("SRA000001/SRA000001.experiment.xml", xml.as_slice())]);

        let cursor = std::io::Cursor::new(tar_bytes);
        let mut records = Vec::new();
        let mut errors = Vec::new();
        process_tar_archive(cursor, |result| match result {
            Ok(rec) => records.push(rec),
            Err(e) => errors.push(e),
        })
        .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].accession, "SRX000001");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("missing accession"));
    }

    // --- Plain tar tests ---

    fn build_plain_tar(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());

        for (path, data) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *data).unwrap();
        }

        builder.into_inner().unwrap()
    }

    #[test]
    fn test_plain_tar_parser() {
        let xml = fixture_xml();
        let tar_bytes = build_plain_tar(&[
            ("SRA000001/SRA000001.experiment.xml", &xml),
            ("SRA000001/SRA000001.run.xml", b"<RUN_SET/>"),
            ("SRA000002/SRA000002.submission.xml", b"<SUBMISSION/>"),
        ]);

        // Sanity check: a plain tar must not start with the gzip magic so
        // the auto-detection actually exercises the non-gzipped branch.
        assert_ne!(&tar_bytes[..2], &GZIP_MAGIC);

        let cursor = std::io::Cursor::new(tar_bytes);
        let mut records = Vec::new();
        process_tar_archive(cursor, |result| {
            if let Ok(rec) = result {
                records.push(rec);
            }
        })
        .unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].accession, "SRX000001");
        assert_eq!(records[1].accession, "DRX000002");
        assert_eq!(records[2].accession, "ERX000003");
    }

    #[test]
    fn test_plain_tar_propagates_errors() {
        let xml = br#"<?xml version="1.0"?>
<EXPERIMENT_SET>
  <EXPERIMENT accession="SRX000001">
    <DESIGN><LIBRARY_DESCRIPTOR>
      <LIBRARY_STRATEGY>WGS</LIBRARY_STRATEGY><LIBRARY_SOURCE>GENOMIC</LIBRARY_SOURCE>
      <LIBRARY_SELECTION>RANDOM</LIBRARY_SELECTION><LIBRARY_LAYOUT><SINGLE/></LIBRARY_LAYOUT>
    </LIBRARY_DESCRIPTOR></DESIGN><PLATFORM><ILLUMINA><INSTRUMENT_MODEL>HiSeq</INSTRUMENT_MODEL></ILLUMINA></PLATFORM>
  </EXPERIMENT>
  <EXPERIMENT>
    <TITLE>No accession</TITLE>
  </EXPERIMENT>
</EXPERIMENT_SET>"#;

        let tar_bytes = build_plain_tar(&[("SRA000001/SRA000001.experiment.xml", xml.as_slice())]);

        let cursor = std::io::Cursor::new(tar_bytes);
        let mut records = Vec::new();
        let mut errors = Vec::new();
        process_tar_archive(cursor, |result| match result {
            Ok(rec) => records.push(rec),
            Err(e) => errors.push(e),
        })
        .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].accession, "SRX000001");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("missing accession"));
    }

    #[test]
    fn test_corrupt_gzip_returns_error() {
        // Starts with the gzip magic but the rest is garbage — GzDecoder
        // must surface an I/O error rather than silently producing no records.
        let mut bytes = vec![GZIP_MAGIC[0], GZIP_MAGIC[1]];
        bytes.extend_from_slice(b"not actually gzip data");

        let cursor = std::io::Cursor::new(bytes);
        let result = process_tar_archive(cursor, |_| {});
        assert!(result.is_err(), "expected error for corrupt gzip input");
    }

    #[test]
    fn test_short_input_below_magic_does_not_panic() {
        // Single-byte input: magic peek falls back to the plain-tar branch.
        // tar::Archive may or may not error while iterating, but we must not panic.
        let cursor = std::io::Cursor::new(vec![0x1fu8]);
        let _ = process_tar_archive(cursor, |_| {});
    }
}
