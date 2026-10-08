use serde::Serialize;
use std::io::Write;

use super::{xsd_date_type, Serializer};
use crate::model::BioProjectRecord;

#[derive(Debug, Clone, Serialize)]
struct JsonLdContext {
    idorg_bp: &'static str,
    bp_ont: &'static str,
    dct: &'static str,
    rdfs: &'static str,
    xsd: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct TypedDate {
    #[serde(rename = "@value")]
    value: String,
    #[serde(rename = "@type")]
    r#type: &'static str,
}

impl TypedDate {
    fn new(v: &str) -> Self {
        TypedDate {
            value: v.to_string(),
            r#type: match xsd_date_type(v) {
                "date" => "xsd:date",
                _ => "xsd:dateTime",
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct JsonLdRecord {
    #[serde(rename = "@context")]
    context: JsonLdContext,
    #[serde(rename = "@type")]
    r#type: &'static str,
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "dct:identifier")]
    dct_identifier: String,
    #[serde(rename = "dct:description", skip_serializing_if = "Option::is_none")]
    dct_description: Option<String>,
    #[serde(rename = "rdfs:label", skip_serializing_if = "Option::is_none")]
    rdfs_label: Option<String>,
    #[serde(rename = "dct:issued", skip_serializing_if = "Option::is_none")]
    dct_issued: Option<TypedDate>,
    #[serde(rename = "dct:created", skip_serializing_if = "Option::is_none")]
    dct_created: Option<TypedDate>,
}

#[derive(Debug, Clone, Default)]
pub struct JsonLdSerializer;

impl Serializer for JsonLdSerializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "[")?;
        Ok(())
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &BioProjectRecord,
    ) -> std::io::Result<()> {
        let obj = JsonLdRecord {
            context: JsonLdContext {
                idorg_bp: "http://identifiers.org/bioproject/",
                bp_ont: "http://ddbj.nig.ac.jp/ontologies/bioproject/",
                dct: "http://purl.org/dc/terms/",
                rdfs: "http://www.w3.org/2000/01/rdf-schema#",
                xsd: "http://www.w3.org/2001/XMLSchema#",
            },
            r#type: "bp_ont:BioProjectRecord",
            id: record.iri(),
            dct_identifier: record.accession.clone(),
            dct_description: record.title.clone(),
            rdfs_label: record.label().map(|s| s.to_string()),
            dct_issued: record.release_date.as_deref().map(TypedDate::new),
            dct_created: record.submission_date.as_deref().map(TypedDate::new),
        };

        let json = serde_json::to_string_pretty(&obj).map_err(std::io::Error::other)?;
        write!(writer, "\n{}", json)?;
        Ok(())
    }

    fn write_footer<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "\n]")?;
        Ok(())
    }
}

impl JsonLdSerializer {
    pub fn new() -> Self {
        JsonLdSerializer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_is_valid_json() {
        let ser = JsonLdSerializer::new();
        let rec = BioProjectRecord {
            accession: "PRJNA3".to_string(),
            name: Some("Test".to_string()),
            title: Some("Title".to_string()),
            description: None,
            organism_name: None,
            taxonomy_id: None,
            release_date: None,
            submission_date: None,
        };
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        ser.write_record(&mut buf, &rec).unwrap();
        ser.write_footer(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
        assert!(parsed.is_array());
        assert_eq!(parsed[0]["@type"], "bp_ont:BioProjectRecord");
    }

    fn dated_record(issued: &str, created: &str) -> serde_json::Value {
        let ser = JsonLdSerializer::new();
        let rec = BioProjectRecord {
            accession: "PRJNA3".to_string(),
            name: None,
            title: None,
            description: None,
            organism_name: None,
            taxonomy_id: None,
            release_date: Some(issued.to_string()),
            submission_date: Some(created.to_string()),
        };
        let mut buf = Vec::new();
        ser.write_record(&mut buf, &rec).unwrap();
        serde_json::from_slice(&buf).expect("valid JSON")
    }

    #[test]
    fn test_date_datatype_follows_lexical_form() {
        let v = dated_record("2001-01-09T00:00:00Z", "2003-02-23");
        assert_eq!(
            v["dct:issued"],
            serde_json::json!({"@value": "2001-01-09T00:00:00Z", "@type": "xsd:dateTime"})
        );
        assert_eq!(
            v["dct:created"],
            serde_json::json!({"@value": "2003-02-23", "@type": "xsd:date"})
        );

        let v = dated_record("2001-01-09", "2003-02-23T10:20:30Z");
        assert_eq!(
            v["dct:issued"],
            serde_json::json!({"@value": "2001-01-09", "@type": "xsd:date"})
        );
        assert_eq!(
            v["dct:created"],
            serde_json::json!({"@value": "2003-02-23T10:20:30Z", "@type": "xsd:dateTime"})
        );
    }
}
