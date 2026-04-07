use serde::Serialize;
use std::io::Write;

use super::Serializer;
use crate::model::{to_uri_local, LibraryLayout, SraExperimentRecord};

#[derive(Debug, Clone, Serialize)]
struct JsonLdContext {
    insdc_sra: &'static str,
    dra_ont: &'static str,
    dct: &'static str,
    rdfs: &'static str,
    xsd: &'static str,
}

static CONTEXT: JsonLdContext = JsonLdContext {
    insdc_sra: "http://identifiers.org/insdc.sra/",
    dra_ont: "http://ddbj.nig.ac.jp/ontologies/dra/",
    dct: "http://purl.org/dc/terms/",
    rdfs: "http://www.w3.org/2000/01/rdf-schema#",
    xsd: "http://www.w3.org/2001/XMLSchema#",
};

#[derive(Debug, Clone, Serialize)]
struct IdRef {
    #[serde(rename = "@id")]
    id: String,
}

#[derive(Debug, Clone, Serialize)]
struct TypedDecimal {
    #[serde(rename = "@value")]
    value: String,
    #[serde(rename = "@type")]
    r#type: &'static str,
}

impl TypedDecimal {
    fn new(v: f64) -> Self {
        TypedDecimal {
            value: format_decimal(v),
            r#type: "xsd:decimal",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct LayoutNode {
    #[serde(rename = "@type")]
    r#type: String,
    #[serde(
        rename = "dra_ont:nominalLength",
        skip_serializing_if = "Option::is_none"
    )]
    nominal_length: Option<TypedDecimal>,
    #[serde(
        rename = "dra_ont:nominalSdev",
        skip_serializing_if = "Option::is_none"
    )]
    nominal_sdev: Option<TypedDecimal>,
}

#[derive(Debug, Clone, Serialize)]
struct DesignNode {
    #[serde(rename = "@type")]
    r#type: &'static str,
    #[serde(
        rename = "dra_ont:libraryName",
        skip_serializing_if = "Option::is_none"
    )]
    library_name: Option<String>,
    #[serde(
        rename = "dra_ont:libraryStrategy",
        skip_serializing_if = "Option::is_none"
    )]
    library_strategy: Option<IdRef>,
    #[serde(
        rename = "dra_ont:librarySource",
        skip_serializing_if = "Option::is_none"
    )]
    library_source: Option<IdRef>,
    #[serde(
        rename = "dra_ont:librarySelection",
        skip_serializing_if = "Option::is_none"
    )]
    library_selection: Option<IdRef>,
    #[serde(
        rename = "dra_ont:libraryConstructionProtocol",
        skip_serializing_if = "Option::is_none"
    )]
    library_construction_protocol: Option<String>,
    #[serde(
        rename = "dra_ont:libraryLayout",
        skip_serializing_if = "Option::is_none"
    )]
    library_layout: Option<LayoutNode>,
}

#[derive(Debug, Clone, Serialize)]
struct PlatformNode {
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(
        rename = "dra_ont:instrumentModel",
        skip_serializing_if = "Option::is_none"
    )]
    instrument_model: Option<IdRef>,
}

#[derive(Debug, Clone, Serialize)]
struct JsonLdRecord {
    #[serde(rename = "@context")]
    context: &'static JsonLdContext,
    #[serde(rename = "@type")]
    r#type: &'static str,
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "rdfs:label", skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(rename = "dct:identifier")]
    identifier: String,
    #[serde(rename = "dra_ont:title", skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(
        rename = "dra_ont:designDescription",
        skip_serializing_if = "Option::is_none"
    )]
    design_description: Option<String>,
    #[serde(rename = "dra_ont:platform", skip_serializing_if = "Option::is_none")]
    platform: Option<PlatformNode>,
    #[serde(rename = "dra_ont:design", skip_serializing_if = "Option::is_none")]
    design: Option<DesignNode>,
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
        record: &SraExperimentRecord,
    ) -> std::io::Result<()> {
        // Platform node
        let platform = if record.platform.is_some() || record.instrument_model.is_some() {
            Some(PlatformNode {
                r#type: record
                    .platform
                    .as_ref()
                    .map(|p| format!("dra_ont:{}", to_uri_local(p))),
                instrument_model: record.instrument_model.as_ref().map(|im| IdRef {
                    id: format!("dra_ont:{}", to_uri_local(im)),
                }),
            })
        } else {
            None
        };

        // Design node
        let has_design = record.library_name.is_some()
            || record.library_strategy.is_some()
            || record.library_source.is_some()
            || record.library_selection.is_some()
            || record.library_construction_protocol.is_some()
            || record.library_layout.is_some();

        let design = if has_design {
            let library_layout = record.library_layout.as_ref().map(|layout| match layout {
                LibraryLayout::Single => LayoutNode {
                    r#type: "dra_ont:SINGLE".to_string(),
                    nominal_length: None,
                    nominal_sdev: None,
                },
                LibraryLayout::Paired {
                    nominal_length,
                    nominal_sdev,
                } => LayoutNode {
                    r#type: "dra_ont:PAIRED".to_string(),
                    nominal_length: nominal_length.map(TypedDecimal::new),
                    nominal_sdev: nominal_sdev.map(TypedDecimal::new),
                },
            });

            Some(DesignNode {
                r#type: "dra_ont:ExperimentDesign",
                library_name: record.library_name.clone(),
                library_strategy: record.library_strategy.as_ref().map(|s| IdRef {
                    id: format!("dra_ont:{}", to_uri_local(s)),
                }),
                library_source: record.library_source.as_ref().map(|s| IdRef {
                    id: format!("dra_ont:{}", to_uri_local(s)),
                }),
                library_selection: record.library_selection.as_ref().map(|s| IdRef {
                    id: format!("dra_ont:{}", to_uri_local(s)),
                }),
                library_construction_protocol: record.library_construction_protocol.clone(),
                library_layout,
            })
        } else {
            None
        };

        let label = record
            .title
            .as_ref()
            .map(|t| format!("{}: {}", record.accession, t));

        let obj = JsonLdRecord {
            context: &CONTEXT,
            r#type: "dra_ont:Experiment",
            id: record.iri(),
            label,
            identifier: record.accession.clone(),
            title: record.title.clone(),
            design_description: record.design_description.clone(),
            platform,
            design,
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

/// Format an f64 as a clean decimal string (no trailing ".0" for integers).
fn format_decimal(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
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

    fn full_record() -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "SRX000001".to_string(),
            title: Some("RNA-Seq of human brain tissue".to_string()),
            design_description: Some("Total RNA was extracted and sequenced.".to_string()),
            library_name: Some("Brain RNA lib1".to_string()),
            library_strategy: Some("RNA-Seq".to_string()),
            library_source: Some("TRANSCRIPTOMIC".to_string()),
            library_selection: Some("cDNA".to_string()),
            library_layout: Some(LibraryLayout::Paired {
                nominal_length: Some(300.0),
                nominal_sdev: Some(25.5),
            }),
            library_construction_protocol: Some("TruSeq RNA protocol".to_string()),
            platform: Some("ILLUMINA".to_string()),
            instrument_model: Some("Illumina NovaSeq 6000".to_string()),
        }
    }

    fn minimal_record() -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "SRX999999".to_string(),
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
        }
    }

    fn render_full() -> String {
        let ser = JsonLdSerializer::new();
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        ser.write_record(&mut buf, &full_record()).unwrap();
        ser.write_footer(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn render_minimal() -> String {
        let ser = JsonLdSerializer::new();
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        ser.write_record(&mut buf, &minimal_record()).unwrap();
        ser.write_footer(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn test_valid_json() {
        let s = render_full();
        let parsed: serde_json::Value = serde_json::from_str(&s).expect("should be valid JSON");
        assert!(parsed.is_array());
        let first = &parsed[0];
        assert_eq!(first["@type"], "dra_ont:Experiment");
        assert_eq!(first["@id"], "http://identifiers.org/insdc.sra/SRX000001");
        assert_eq!(first["dct:identifier"], "SRX000001");
        assert_eq!(
            first["rdfs:label"],
            "SRX000001: RNA-Seq of human brain tissue"
        );
    }

    #[test]
    fn test_contains_design() {
        let s = render_full();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        let design = &parsed[0]["dra_ont:design"];
        assert_eq!(design["@type"], "dra_ont:ExperimentDesign");
        assert_eq!(design["dra_ont:libraryName"], "Brain RNA lib1");
        assert_eq!(design["dra_ont:libraryStrategy"]["@id"], "dra_ont:RNA-Seq");
        assert_eq!(
            design["dra_ont:librarySource"]["@id"],
            "dra_ont:TRANSCRIPTOMIC"
        );
        assert_eq!(design["dra_ont:librarySelection"]["@id"], "dra_ont:cDNA");
        assert_eq!(
            design["dra_ont:libraryConstructionProtocol"],
            "TruSeq RNA protocol"
        );

        let layout = &design["dra_ont:libraryLayout"];
        assert_eq!(layout["@type"], "dra_ont:PAIRED");
        assert_eq!(layout["dra_ont:nominalLength"]["@value"], "300");
        assert_eq!(layout["dra_ont:nominalLength"]["@type"], "xsd:decimal");
        assert_eq!(layout["dra_ont:nominalSdev"]["@value"], "25.5");
    }

    #[test]
    fn test_contains_platform() {
        let s = render_full();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        let platform = &parsed[0]["dra_ont:platform"];
        assert_eq!(platform["@type"], "dra_ont:ILLUMINA");
        assert_eq!(
            platform["dra_ont:instrumentModel"]["@id"],
            "dra_ont:Illumina_NovaSeq_6000"
        );
    }

    #[test]
    fn test_minimal_record_omits_optional() {
        let s = render_minimal();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        let first = &parsed[0];
        assert_eq!(first["@type"], "dra_ont:Experiment");
        assert_eq!(first["dct:identifier"], "SRX999999");
        // Optional fields should be absent (null in JSON Value access)
        assert!(
            first.get("dra_ont:platform").is_none(),
            "platform should be absent"
        );
        assert!(
            first.get("dra_ont:design").is_none(),
            "design should be absent"
        );
        assert!(
            first.get("dra_ont:title").is_none(),
            "title should be absent"
        );
        assert!(
            first.get("dra_ont:designDescription").is_none(),
            "designDescription should be absent"
        );
        assert!(first.get("rdfs:label").is_none(), "label should be absent");
    }
}
