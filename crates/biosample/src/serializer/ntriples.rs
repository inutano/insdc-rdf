use std::io::Write;

use super::Serializer;
use crate::model::BioSampleRecord;
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
        record: &BioSampleRecord,
    ) -> std::io::Result<()> {
        let acc = &record.accession;
        let subj = format!("{}{}", IDORG_BIOSAMPLE, acc);

        writeln!(
            writer,
            "<{}> <{}> <{}BioSampleRecord> .",
            subj, RDF_TYPE, DDBJ_BIOSAMPLE_ONT
        )?;
        writeln!(
            writer,
            "<{}> <{}identifier> \"{}\" .",
            subj,
            DCT,
            escape_ntriples_string(acc)
        )?;

        if let Some(title) = &record.title {
            writeln!(
                writer,
                "<{}> <{}description> \"{}\" .",
                subj,
                DCT,
                escape_ntriples_string(title)
            )?;
            writeln!(
                writer,
                "<{}> <{}label> \"{}\" .",
                subj,
                RDFS,
                escape_ntriples_string(title)
            )?;
        }

        let xsd_datetime = format!("{}dateTime", XSD);
        if let Some(date) = &record.submission_date {
            writeln!(
                writer,
                "<{}> <{}created> \"{}\"^^<{}> .",
                subj,
                DCT,
                escape_ntriples_string(date),
                xsd_datetime
            )?;
        }
        if let Some(date) = &record.last_update {
            writeln!(
                writer,
                "<{}> <{}modified> \"{}\"^^<{}> .",
                subj,
                DCT,
                escape_ntriples_string(date),
                xsd_datetime
            )?;
        }
        if let Some(date) = &record.publication_date {
            writeln!(
                writer,
                "<{}> <{}issued> \"{}\"^^<{}> .",
                subj,
                DCT,
                escape_ntriples_string(date),
                xsd_datetime
            )?;
        }

        let schema_additional = format!("{}additionalProperty", SCHEMA);
        let schema_propval = format!("{}PropertyValue", SCHEMA);
        let schema_name = format!("{}name", SCHEMA);
        let schema_value = format!("{}value", SCHEMA);

        for attr in &record.attributes {
            let prop_iri = attr.property_iri(acc);
            let name = attr.preferred_name();
            let value = attr.value.as_deref().unwrap_or("");

            writeln!(
                writer,
                "<{}> <{}> <{}> .",
                subj, schema_additional, prop_iri
            )?;
            writeln!(
                writer,
                "<{}> <{}> <{}> .",
                prop_iri, RDF_TYPE, schema_propval
            )?;
            writeln!(
                writer,
                "<{}> <{}> \"{}\" .",
                prop_iri,
                schema_name,
                escape_ntriples_string(name)
            )?;
            writeln!(
                writer,
                "<{}> <{}> \"{}\" .",
                prop_iri,
                schema_value,
                escape_ntriples_string(value)
            )?;
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

    pub fn record_to_string(&self, record: &BioSampleRecord) -> String {
        let mut buf = Vec::new();
        self.write_record(&mut buf, record).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Attribute;

    fn sample_record() -> BioSampleRecord {
        BioSampleRecord {
            accession: "SAMD00000345".to_string(),
            submission_date: Some("2014-07-30T00:00:00Z".to_string()),
            last_update: None,
            publication_date: None,
            title: Some("type strain".to_string()),
            attributes: vec![Attribute {
                attribute_name: "organism".to_string(),
                harmonized_name: Some("organism".to_string()),
                display_name: None,
                value: Some("Homo sapiens".to_string()),
            }],
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
        assert!(s.contains("<http://identifiers.org/biosample/SAMD00000345>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/biosample/BioSampleRecord>"));
    }

    #[test]
    fn test_no_prefixed_names() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&sample_record());
        assert!(!s.contains("idorg:"));
        assert!(!s.contains("dct:"));
        assert!(!s.contains("ddbjont:"));
    }
}
