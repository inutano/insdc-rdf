use std::io::Write;

use super::Serializer;
use crate::model::BioProjectRecord;
use insdc_rdf_core::escape::escape_turtle_string;
use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, Default)]
pub struct TurtleSerializer;

impl Serializer for TurtleSerializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, "@prefix idorg_bp: <{}> .", IDORG_BIOPROJECT)?;
        writeln!(writer, "@prefix bp_ont: <{}> .", DDBJ_BIOPROJECT_ONT)?;
        writeln!(writer, "@prefix dct: <{}> .", DCT)?;
        writeln!(writer, "@prefix rdfs: <{}> .", RDFS)?;
        writeln!(writer, "@prefix xsd: <{}> .", XSD)?;
        writeln!(writer)?;
        Ok(())
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &BioProjectRecord,
    ) -> std::io::Result<()> {
        let acc = &record.accession;
        let mut po_lines: Vec<String> = Vec::new();

        po_lines.push("a bp_ont:BioProjectRecord".to_string());
        po_lines.push(format!("dct:identifier \"{}\"", escape_turtle_string(acc)));

        if let Some(ref title) = record.title {
            po_lines.push(format!(
                "dct:description \"{}\"",
                escape_turtle_string(title)
            ));
        }

        if let Some(label) = record.label() {
            po_lines.push(format!("rdfs:label \"{}\"", escape_turtle_string(label)));
        }

        if let Some(ref date) = record.release_date {
            po_lines.push(format!(
                "dct:issued \"{}\"^^xsd:dateTime",
                escape_turtle_string(date)
            ));
        }
        if let Some(ref date) = record.submission_date {
            po_lines.push(format!(
                "dct:created \"{}\"^^xsd:dateTime",
                escape_turtle_string(date)
            ));
        }

        writeln!(writer, "idorg_bp:{}", acc)?;
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
            name: Some("Borreliella burgdorferi B31".to_string()),
            title: Some("Causes Lyme disease".to_string()),
            description: Some("Type strain".to_string()),
            organism_name: Some("Borreliella burgdorferi B31".to_string()),
            taxonomy_id: Some("224326".to_string()),
            release_date: Some("2001-01-09T00:00:00Z".to_string()),
            submission_date: Some("2003-02-23".to_string()),
        }
    }

    #[test]
    fn test_record_output() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&sample_record());
        assert!(s.contains("idorg_bp:PRJNA3"));
        assert!(s.contains("a bp_ont:BioProjectRecord"));
        assert!(s.contains("dct:identifier \"PRJNA3\""));
        assert!(s.contains("dct:description \"Causes Lyme disease\""));
        assert!(s.contains("rdfs:label \"Borreliella burgdorferi B31\""));
        assert!(s.contains("dct:issued \"2001-01-09T00:00:00Z\"^^xsd:dateTime"));
        assert!(s.contains("dct:created \"2003-02-23\"^^xsd:dateTime"));
    }

    #[test]
    fn test_record_ends_with_dot() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&sample_record());
        assert!(s.trim().ends_with('.'));
    }
}
