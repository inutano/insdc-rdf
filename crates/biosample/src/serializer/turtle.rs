use std::io::Write;

use crate::model::BioSampleRecord;
use super::Serializer;
use insdc_rdf_core::escape::escape_turtle_string;
use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, Default)]
pub struct TurtleSerializer;

impl Serializer for TurtleSerializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, "@prefix : <{}> .", SCHEMA)?;
        writeln!(writer, "@prefix idorg: <{}> .", IDORG_BIOSAMPLE)?;
        writeln!(writer, "@prefix dct: <{}> .", DCT)?;
        writeln!(writer, "@prefix ddbjont: <{}> .", DDBJ_BIOSAMPLE_ONT)?;
        writeln!(writer, "@prefix rdfs: <{}> .", RDFS)?;
        writeln!(writer, "@prefix xsd: <{}> .", XSD)?;
        writeln!(writer)?;
        Ok(())
    }

    fn write_record<W: Write>(&self, writer: &mut W, record: &BioSampleRecord) -> std::io::Result<()> {
        let acc = &record.accession;

        let mut po_lines: Vec<String> = Vec::new();

        po_lines.push("a ddbjont:BioSampleRecord".to_string());
        po_lines.push(format!("dct:identifier \"{}\"", escape_turtle_string(acc)));

        if let Some(title) = &record.title {
            po_lines.push(format!("dct:description \"{}\"", escape_turtle_string(title)));
            po_lines.push(format!("rdfs:label \"{}\"", escape_turtle_string(title)));
        }

        if let Some(date) = &record.submission_date {
            po_lines.push(format!("dct:created \"{}\"^^xsd:dateTime", escape_turtle_string(date)));
        }
        if let Some(date) = &record.last_update {
            po_lines.push(format!("dct:modified \"{}\"^^xsd:dateTime", escape_turtle_string(date)));
        }
        if let Some(date) = &record.publication_date {
            po_lines.push(format!("dct:issued \"{}\"^^xsd:dateTime", escape_turtle_string(date)));
        }

        if !record.attributes.is_empty() {
            let prop_iris: Vec<String> = record
                .attributes
                .iter()
                .map(|a| format!("    <{}>", a.property_iri(acc)))
                .collect();
            let ap_block = format!("  :additionalProperty\n{}", prop_iris.join(" ,\n"));
            writeln!(writer, "idorg:{}", acc)?;
            for line in po_lines.iter() {
                writeln!(writer, "  {} ;", line)?;
            }
            writeln!(writer, "{} .", ap_block)?;
        } else {
            writeln!(writer, "idorg:{}", acc)?;
            let last = po_lines.len().saturating_sub(1);
            for (i, line) in po_lines.iter().enumerate() {
                if i < last {
                    writeln!(writer, "  {} ;", line)?;
                } else {
                    writeln!(writer, "  {} .", line)?;
                }
            }
        }
        writeln!(writer)?;

        for attr in &record.attributes {
            let prop_iri = attr.property_iri(acc);
            let name = attr.preferred_name();
            let value = attr.value.as_deref().unwrap_or("");
            writeln!(writer, "<{}> a :PropertyValue ;", prop_iri)?;
            writeln!(writer, "  :name \"{}\" ;", escape_turtle_string(name))?;
            writeln!(writer, "  :value \"{}\" .", escape_turtle_string(value))?;
            writeln!(writer)?;
        }

        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

impl TurtleSerializer {
    pub fn new() -> Self {
        TurtleSerializer
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
    fn test_header_contains_prefixes() {
        let ser = TurtleSerializer::new();
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("@prefix : <http://schema.org/> ."));
        assert!(s.contains("@prefix idorg: <http://identifiers.org/biosample/> ."));
    }

    #[test]
    fn test_record_output_contains_correct_triples() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&sample_record());
        assert!(s.contains("idorg:SAMD00000345"));
        assert!(s.contains("a ddbjont:BioSampleRecord"));
        assert!(s.contains("dct:identifier \"SAMD00000345\""));
        assert!(s.contains(":additionalProperty"));
        assert!(s.contains("a :PropertyValue"));
    }

    #[test]
    fn test_empty_attributes_list() {
        let ser = TurtleSerializer::new();
        let rec = BioSampleRecord {
            accession: "SAMD00000001".to_string(),
            submission_date: None,
            last_update: None,
            publication_date: None,
            title: None,
            attributes: vec![],
        };
        let s = ser.record_to_string(&rec);
        assert!(!s.contains(":additionalProperty"));
        assert!(s.trim().ends_with('.'));
    }
}
