use std::io::Write;

use super::Serializer;
use crate::model::SraAccessionRecord;
use insdc_rdf_core::escape::escape_ntriples_string;
use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, Default)]
pub struct NTriplesSerializer;

impl Serializer for NTriplesSerializer {
    fn write_header<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &SraAccessionRecord,
    ) -> std::io::Result<()> {
        let subj = record.iri();
        let class = record.sra_type.rdf_class();

        // rdf:type
        writeln!(
            writer,
            "<{}> <{}> <{}{}> .",
            subj, RDF_TYPE, DDBJ_DRA_ONT, class
        )?;

        // dct:identifier
        writeln!(
            writer,
            "<{}> <{}identifier> \"{}\" .",
            subj,
            DCT,
            escape_ntriples_string(&record.accession)
        )?;

        // dates
        let xsd_dt = format!("{}dateTime", XSD);
        if let Some(ref date) = record.published {
            writeln!(
                writer,
                "<{}> <{}issued> \"{}\"^^<{}> .",
                subj,
                DCT,
                escape_ntriples_string(date),
                xsd_dt
            )?;
        }
        if let Some(ref date) = record.updated {
            writeln!(
                writer,
                "<{}> <{}modified> \"{}\"^^<{}> .",
                subj,
                DCT,
                escape_ntriples_string(date),
                xsd_dt
            )?;
        }

        // rdfs:seeAlso
        let see_also = format!("{}seeAlso", RDFS);
        for iri in record.see_also_iris() {
            writeln!(writer, "<{}> <{}> <{}> .", subj, see_also, iri)?;
        }

        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

impl NTriplesSerializer {
    pub fn new() -> Self {
        NTriplesSerializer
    }

    pub fn record_to_string(&self, record: &SraAccessionRecord) -> String {
        let mut buf = Vec::new();
        self.write_record(&mut buf, record).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SraType;

    fn run_record() -> SraAccessionRecord {
        SraAccessionRecord {
            accession: "SRR000001".to_string(),
            sra_type: SraType::Run,
            submission: None,
            updated: None,
            published: None,
            center: None,
            experiment: Some("SRX000001".to_string()),
            sample: Some("SRS000001".to_string()),
            study: Some("SRP000001".to_string()),
            biosample: Some("SAMN00000001".to_string()),
            bioproject: Some("PRJNA000001".to_string()),
        }
    }

    #[test]
    fn test_every_line_ends_with_space_dot() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&run_record());
        for line in s.lines() {
            assert!(line.ends_with(" ."), "Line: {:?}", line);
        }
    }

    #[test]
    fn test_contains_full_iris() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&run_record());
        assert!(s.contains("<http://identifiers.org/insdc.sra/SRR000001>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/Run>"));
        assert!(s.contains("<http://identifiers.org/insdc.sra/SRX000001>"));
        assert!(s.contains("<http://identifiers.org/biosample/SAMN00000001>"));
        assert!(s.contains("<http://identifiers.org/bioproject/PRJNA000001>"));
    }

    #[test]
    fn test_no_prefixed_names() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&run_record());
        assert!(!s.contains("insdc_sra:"));
        assert!(!s.contains("dra_ont:"));
    }
}
