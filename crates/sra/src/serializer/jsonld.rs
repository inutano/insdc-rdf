use serde::Serialize;
use std::io::Write;

use super::Serializer;
use crate::model::SraAccessionRecord;

#[derive(Debug, Clone, Serialize)]
struct JsonLdContext {
    #[serde(rename = "@vocab")]
    vocab: &'static str,
    insdc_sra: &'static str,
    idorg_biosample: &'static str,
    idorg_bioproject: &'static str,
    dra_ont: &'static str,
    dct: &'static str,
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
        TypedDateTime {
            value: v.into(),
            r#type: "xsd:dateTime",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SeeAlso {
    #[serde(rename = "@id")]
    id: String,
}

#[derive(Debug, Clone, Serialize)]
struct JsonLdRecord {
    #[serde(rename = "@context")]
    context: JsonLdContext,
    #[serde(rename = "@type")]
    r#type: String,
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "dct:identifier")]
    dct_identifier: String,
    #[serde(rename = "dct:issued", skip_serializing_if = "Option::is_none")]
    dct_issued: Option<TypedDateTime>,
    #[serde(rename = "dct:modified", skip_serializing_if = "Option::is_none")]
    dct_modified: Option<TypedDateTime>,
    #[serde(rename = "rdfs:seeAlso", skip_serializing_if = "Vec::is_empty")]
    see_also: Vec<SeeAlso>,
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
        record: &SraAccessionRecord,
    ) -> std::io::Result<()> {
        let see_also: Vec<SeeAlso> = record
            .see_also_iris()
            .into_iter()
            .map(|iri| SeeAlso { id: iri })
            .collect();

        let obj = JsonLdRecord {
            context: JsonLdContext {
                vocab: "http://schema.org/",
                insdc_sra: "http://identifiers.org/insdc.sra/",
                idorg_biosample: "http://identifiers.org/biosample/",
                idorg_bioproject: "http://identifiers.org/bioproject/",
                dra_ont: "http://ddbj.nig.ac.jp/ontologies/dra/",
                dct: "http://purl.org/dc/terms/",
                rdfs: "http://www.w3.org/2000/01/rdf-schema#",
                xsd: "http://www.w3.org/2001/XMLSchema#",
            },
            r#type: format!("dra_ont:{}", record.sra_type.rdf_class()),
            id: record.iri(),
            dct_identifier: record.accession.clone(),
            dct_issued: record.published.as_deref().map(TypedDateTime::new),
            dct_modified: record.updated.as_deref().map(TypedDateTime::new),
            see_also,
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
    use crate::model::SraType;

    fn run_record() -> SraAccessionRecord {
        SraAccessionRecord {
            accession: "DRR000001".to_string(),
            sra_type: SraType::Run,
            submission: None,
            updated: None,
            published: Some("2010-03-24T03:10:22Z".to_string()),
            center: None,
            experiment: Some("DRX000001".to_string()),
            sample: None,
            study: None,
            biosample: Some("SAMD00016353".to_string()),
            bioproject: None,
        }
    }

    #[test]
    fn test_output_is_valid_json() {
        let ser = JsonLdSerializer::new();
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        ser.write_record(&mut buf, &run_record()).unwrap();
        ser.write_footer(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
        assert!(parsed.is_array());
        assert_eq!(parsed[0]["@type"], "dra_ont:Run");
    }
}
