use std::cell::Cell;
use std::io::Write;

use insdc_rdf_core::prefix::*;
use serde_json::{json, Map, Value};

use super::Serializer;
use crate::meta::{CrateMeta, CITATION};
use crate::model::Record;

fn context() -> Value {
    json!({
        "schema": SCHEMA,
        "rdfs": RDFS,
        "xsd": XSD,
        "prov": PROV,
        "biosample_ont": DDBJ_BIOSAMPLE_ONT,
    })
}

fn id(iri: &str) -> Value {
    json!({ "@id": iri })
}

fn typed(value: &str, datatype: &str) -> Value {
    json!({ "@value": value, "@type": datatype })
}

/// Writes each chunk as one JSON array of node objects, each with its own `@context`.
#[derive(Debug)]
pub struct JsonLdSerializer {
    first: Cell<bool>,
}

impl Default for JsonLdSerializer {
    fn default() -> Self {
        JsonLdSerializer {
            first: Cell::new(true),
        }
    }
}

impl JsonLdSerializer {
    pub fn new() -> Self {
        Self::default()
    }

    fn write_node<W: Write>(&self, w: &mut W, node: Value) -> std::io::Result<()> {
        if !self.first.replace(false) {
            write!(w, ",")?;
        }
        let s = serde_json::to_string_pretty(&node).map_err(std::io::Error::other)?;
        write!(w, "\n{}", s)
    }
}

impl Serializer for JsonLdSerializer {
    fn write_header<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        self.first.set(true);
        write!(w, "[")
    }

    fn write_provenance<W: Write>(&self, w: &mut W, meta: &CrateMeta) -> std::io::Result<()> {
        let rel = &meta.release;
        self.write_node(
            w,
            json!({
                "@context": context(),
                "@id": rel.iri(),
                "@type": "schema:Dataset",
                "schema:name": rel.name,
                "schema:version": rel.release_id,
                "schema:datePublished": typed(&rel.date_published, "xsd:date"),
                "schema:license": id(&rel.license),
                "schema:citation": id(CITATION),
            }),
        )?;
        for run in &meta.runs {
            let software: Vec<Value> = run.commit_urls.iter().map(|u| id(u)).collect();
            self.write_node(
                w,
                json!({
                    "@context": context(),
                    "@id": rel.run_iri(&run.run_name),
                    "@type": "prov:Activity",
                    "rdfs:label": run.run_name,
                    "prov:startedAtTime": typed(&run.first_start, "xsd:dateTime"),
                    "prov:endedAtTime": typed(&run.last_end, "xsd:dateTime"),
                    "prov:wasAssociatedWith": software,
                    "biosample_ont:llmModel": run.model,
                    "schema:isPartOf": id(&rel.iri()),
                }),
            )?;
        }
        Ok(())
    }

    fn write_record<W: Write>(&self, w: &mut W, rec: &Record) -> std::io::Result<()> {
        if !rec.annotations.is_empty() {
            let nodes: Vec<Value> = rec
                .annotations
                .iter()
                .map(|a| {
                    json!({
                        "@id": a.iri,
                        "@type": "schema:PropertyValue",
                        "schema:name": a.field,
                        "schema:propertyID": a.field,
                        "schema:value": a.values,
                        "schema:valueReference": id(&a.term_uri),
                        "biosample_ont:exactMatch": a.exact_match,
                        "prov:wasGeneratedBy": id(&rec.run_iri),
                    })
                })
                .collect();
            self.write_node(
                w,
                json!({
                    "@context": context(),
                    "@id": rec.biosample_iri,
                    "schema:additionalProperty": nodes,
                }),
            )?;
        }
        for t in &rec.terms {
            let mut node = Map::new();
            node.insert("@context".into(), context());
            node.insert("@id".into(), Value::from(t.term_uri.as_str()));
            if t.declare_type {
                node.insert("@type".into(), Value::from("schema:DefinedTerm"));
            }
            node.insert("rdfs:label".into(), Value::from(t.label.as_str()));
            self.write_node(w, Value::Object(node))?;
        }
        Ok(())
    }

    fn write_footer<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        writeln!(w, "\n]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serializer::test_data::*;

    #[test]
    fn test_jsonld_is_array_of_nodes() {
        let out = render(&JsonLdSerializer::new(), Some(&meta()), &[record()]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let nodes = v.as_array().unwrap();
        // release, 1 run, 1 BioSample, 1 term
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0]["@id"], REL);
        assert_eq!(nodes[0]["@type"], "schema:Dataset");
        assert_eq!(nodes[1]["@type"], "prov:Activity");
        assert_eq!(
            nodes[1]["prov:wasAssociatedWith"].as_array().unwrap().len(),
            2
        );
        let pv = &nodes[2]["schema:additionalProperty"][0];
        assert_eq!(nodes[2]["@id"], "http://identifiers.org/biosample/SAMN1");
        assert_eq!(pv["@id"], NODE);
        assert_eq!(
            pv["schema:value"],
            serde_json::json!(["REV-ERB a", "REV-ERB b"])
        );
        assert_eq!(pv["schema:valueReference"]["@id"], TERM);
        assert_eq!(pv["biosample_ont:exactMatch"], true);
        assert_eq!(nodes[3]["@type"], "schema:DefinedTerm");
        assert_eq!(nodes[3]["rdfs:label"], "Nr1d1");
        assert!(nodes.iter().all(|n| n.get("@context").is_some()));
    }

    #[test]
    fn test_jsonld_empty_chunk_is_valid() {
        let out = render(&JsonLdSerializer::new(), None, &[]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.as_array().unwrap().is_empty());
    }
}
