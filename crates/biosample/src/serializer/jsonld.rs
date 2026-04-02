use std::io::Write;
use serde::Serialize;

use crate::model::BioSampleRecord;
use super::Serializer;

#[derive(Debug, Clone, Serialize)]
struct JsonLdContext {
    #[serde(rename = "@base")]
    base: &'static str,
    #[serde(rename = "@vocab")]
    vocab: &'static str,
    dct: &'static str,
    ddbjont: &'static str,
    rdfs: &'static str,
    xsd: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct TypedDateTime {
    #[serde(rename = "@value")]
    value: String,
    #[serde(rename = "@type")]
    r#type: &'static str,
}

impl TypedDateTime {
    fn new(v: impl Into<String>) -> Self {
        TypedDateTime { value: v.into(), r#type: "xsd:dateTime" }
    }
}

#[derive(Debug, Clone, Serialize)]
struct PropertyValue {
    #[serde(rename = "@type")]
    r#type: &'static str,
    #[serde(rename = "@id")]
    id: String,
    name: String,
    value: String,
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
    #[serde(rename = "dct:created", skip_serializing_if = "Option::is_none")]
    dct_created: Option<TypedDateTime>,
    #[serde(rename = "dct:modified", skip_serializing_if = "Option::is_none")]
    dct_modified: Option<TypedDateTime>,
    #[serde(rename = "dct:issued", skip_serializing_if = "Option::is_none")]
    dct_issued: Option<TypedDateTime>,
    #[serde(rename = "additionalProperty", skip_serializing_if = "Vec::is_empty")]
    additional_property: Vec<PropertyValue>,
}

#[derive(Debug, Clone, Default)]
pub struct JsonLdSerializer;

impl Serializer for JsonLdSerializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "[")?;
        Ok(())
    }

    fn write_record<W: Write>(&self, writer: &mut W, record: &BioSampleRecord) -> std::io::Result<()> {
        let acc = &record.accession;

        let additional_property: Vec<PropertyValue> = record
            .attributes
            .iter()
            .map(|attr| PropertyValue {
                r#type: "PropertyValue",
                id: attr.property_iri(acc),
                name: attr.preferred_name().to_owned(),
                value: attr.value.clone().unwrap_or_default(),
            })
            .collect();

        let obj = JsonLdRecord {
            context: JsonLdContext {
                base: "http://identifiers.org/biosample/",
                vocab: "http://schema.org/",
                dct: "http://purl.org/dc/terms/",
                ddbjont: "http://ddbj.nig.ac.jp/ontologies/biosample/",
                rdfs: "http://www.w3.org/2000/01/rdf-schema#",
                xsd: "http://www.w3.org/2001/XMLSchema#",
            },
            r#type: "ddbjont:BioSampleRecord",
            id: record.iri(),
            dct_identifier: acc.clone(),
            dct_description: record.title.clone(),
            rdfs_label: record.title.clone(),
            dct_created: record.submission_date.as_deref().map(TypedDateTime::new),
            dct_modified: record.last_update.as_deref().map(TypedDateTime::new),
            dct_issued: record.publication_date.as_deref().map(TypedDateTime::new),
            additional_property,
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

    fn full_output(ser: &JsonLdSerializer, record: &BioSampleRecord) -> String {
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        ser.write_record(&mut buf, record).unwrap();
        ser.write_footer(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn test_output_is_valid_json() {
        let ser = JsonLdSerializer::new();
        let s = full_output(&ser, &sample_record());
        let parsed: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
        assert!(parsed.is_array());
    }

    #[test]
    fn test_has_context_and_id() {
        let ser = JsonLdSerializer::new();
        let s = full_output(&ser, &sample_record());
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        let obj = &parsed[0];
        assert!(obj.get("@context").is_some());
        assert_eq!(obj["@id"], "http://identifiers.org/biosample/SAMD00000345");
    }
}
