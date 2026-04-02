use std::io::Write;

use crate::model::SraAccessionRecord;
use super::Serializer;
use insdc_rdf_core::escape::escape_turtle_string;
use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, Default)]
pub struct TurtleSerializer;

impl Serializer for TurtleSerializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, "@prefix insdc_sra: <{}> .", IDORG_SRA)?;
        writeln!(writer, "@prefix idorg_biosample: <{}> .", IDORG_BIOSAMPLE)?;
        writeln!(writer, "@prefix idorg_bioproject: <{}> .", IDORG_BIOPROJECT)?;
        writeln!(writer, "@prefix dra_ont: <{}> .", DDBJ_DRA_ONT)?;
        writeln!(writer, "@prefix dct: <{}> .", DCT)?;
        writeln!(writer, "@prefix rdfs: <{}> .", RDFS)?;
        writeln!(writer, "@prefix xsd: <{}> .", XSD)?;
        writeln!(writer)?;
        Ok(())
    }

    fn write_record<W: Write>(&self, writer: &mut W, record: &SraAccessionRecord) -> std::io::Result<()> {
        let acc = &record.accession;
        let class = record.sra_type.rdf_class();

        let mut po_lines: Vec<String> = Vec::new();
        po_lines.push(format!("a dra_ont:{}", class));
        po_lines.push(format!("dct:identifier \"{}\"", escape_turtle_string(acc)));

        if let Some(ref date) = record.published {
            po_lines.push(format!("dct:issued \"{}\"^^xsd:dateTime", escape_turtle_string(date)));
        }
        if let Some(ref date) = record.updated {
            po_lines.push(format!("dct:modified \"{}\"^^xsd:dateTime", escape_turtle_string(date)));
        }

        // rdfs:seeAlso cross-links
        for iri in record.see_also_iris() {
            po_lines.push(format!("rdfs:seeAlso <{}>", iri));
        }

        writeln!(writer, "insdc_sra:{}", acc)?;
        let last = po_lines.len().saturating_sub(1);
        for (i, line) in po_lines.iter().enumerate() {
            if i < last {
                writeln!(writer, "  {} ;", line)?;
            } else {
                writeln!(writer, "  {} .", line)?;
            }
        }
        writeln!(writer)?;

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
            accession: "DRR000001".to_string(),
            sra_type: SraType::Run,
            submission: Some("DRA000001".to_string()),
            updated: Some("2022-09-23T10:09:59Z".to_string()),
            published: Some("2010-03-24T03:10:22Z".to_string()),
            center: Some("KEIO".to_string()),
            experiment: Some("DRX000001".to_string()),
            sample: Some("DRS000001".to_string()),
            study: Some("DRP000001".to_string()),
            biosample: Some("SAMD00016353".to_string()),
            bioproject: Some("PRJDA38027".to_string()),
        }
    }

    #[test]
    fn test_header_contains_prefixes() {
        let ser = TurtleSerializer::new();
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("@prefix insdc_sra:"));
        assert!(s.contains("@prefix dra_ont:"));
        assert!(s.contains("@prefix rdfs:"));
    }

    #[test]
    fn test_run_record_output() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&run_record());
        assert!(s.contains("insdc_sra:DRR000001"));
        assert!(s.contains("a dra_ont:Run"));
        assert!(s.contains("dct:identifier \"DRR000001\""));
        assert!(s.contains("rdfs:seeAlso <http://identifiers.org/insdc.sra/DRX000001>"));
        assert!(s.contains("rdfs:seeAlso <http://identifiers.org/biosample/SAMD00016353>"));
        assert!(s.contains("rdfs:seeAlso <http://identifiers.org/bioproject/PRJDA38027>"));
    }

    #[test]
    fn test_record_ends_with_dot() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&run_record());
        let trimmed = s.trim();
        assert!(trimmed.ends_with('.'));
    }
}
