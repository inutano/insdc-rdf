use std::io::Write;

use crate::model::BioProjectRecord;
use super::Serializer;
use insdc_rdf_core::escape::escape_ntriples_string;
use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, Default)]
pub struct NTriplesSerializer;

impl Serializer for NTriplesSerializer {
    fn write_header<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> { Ok(()) }

    fn write_record<W: Write>(&self, writer: &mut W, record: &BioProjectRecord) -> std::io::Result<()> {
        let subj = record.iri();

        writeln!(writer, "<{}> <{}> <{}BioProjectRecord> .", subj, RDF_TYPE, DDBJ_BIOPROJECT_ONT)?;
        writeln!(writer, "<{}> <{}identifier> \"{}\" .", subj, DCT, escape_ntriples_string(&record.accession))?;

        if let Some(ref title) = record.title {
            writeln!(writer, "<{}> <{}description> \"{}\" .", subj, DCT, escape_ntriples_string(title))?;
        }
        if let Some(label) = record.label() {
            writeln!(writer, "<{}> <{}label> \"{}\" .", subj, RDFS, escape_ntriples_string(label))?;
        }

        let xsd_dt = format!("{}dateTime", XSD);
        if let Some(ref date) = record.release_date {
            writeln!(writer, "<{}> <{}issued> \"{}\"^^<{}> .", subj, DCT, escape_ntriples_string(date), xsd_dt)?;
        }
        if let Some(ref date) = record.submission_date {
            writeln!(writer, "<{}> <{}created> \"{}\"^^<{}> .", subj, DCT, escape_ntriples_string(date), xsd_dt)?;
        }

        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> { Ok(()) }
}

impl NTriplesSerializer {
    pub fn new() -> Self { NTriplesSerializer }

    pub fn record_to_string(&self, record: &BioProjectRecord) -> String {
        let mut buf = Vec::new();
        self.write_record(&mut buf, record).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> BioProjectRecord {
        BioProjectRecord {
            accession: "PRJNA3".to_string(),
            name: Some("Project Name".to_string()),
            title: Some("Project Title".to_string()),
            description: None, organism_name: None, taxonomy_id: None,
            release_date: None, submission_date: None,
        }
    }

    #[test]
    fn test_every_line_ends_with_space_dot() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&sample_record());
        for line in s.lines() {
            assert!(line.ends_with(" ."), "Line: {:?}", line);
        }
    }

    #[test]
    fn test_contains_full_iris() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&sample_record());
        assert!(s.contains("<http://identifiers.org/bioproject/PRJNA3>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/bioproject/BioProjectRecord>"));
    }
}
