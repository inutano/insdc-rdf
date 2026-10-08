use std::io::Write;

use insdc_rdf_core::escape::escape_turtle_string as esc;
use insdc_rdf_core::prefix::*;

use super::Serializer;
use crate::meta::{CrateMeta, CITATION};
use crate::model::Record;

#[derive(Debug, Clone, Default)]
pub struct TurtleSerializer;

impl TurtleSerializer {
    pub fn new() -> Self {
        TurtleSerializer
    }
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", esc(s))
}

/// Writes `<subject> po1 ;\n    po2 ;\n    poN .`
fn write_block<W: Write>(w: &mut W, subject: &str, po: &[String]) -> std::io::Result<()> {
    writeln!(w, "<{}> {} .", subject, po.join(" ;\n    "))
}

impl Serializer for TurtleSerializer {
    fn write_header<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        writeln!(w, "@prefix schema: <{}> .", SCHEMA)?;
        writeln!(w, "@prefix rdfs: <{}> .", RDFS)?;
        writeln!(w, "@prefix xsd: <{}> .", XSD)?;
        writeln!(w, "@prefix prov: <{}> .", PROV)?;
        writeln!(w, "@prefix biosample_ont: <{}> .", DDBJ_BIOSAMPLE_ONT)?;
        writeln!(w)
    }

    fn write_provenance<W: Write>(&self, w: &mut W, meta: &CrateMeta) -> std::io::Result<()> {
        let rel = &meta.release;
        write_block(
            w,
            &rel.iri(),
            &[
                "a schema:Dataset".to_string(),
                format!("schema:name {}", quoted(&rel.name)),
                format!("schema:version {}", quoted(&rel.release_id)),
                format!(
                    "schema:datePublished {}^^xsd:date",
                    quoted(&rel.date_published)
                ),
                format!("schema:license <{}>", rel.license),
                format!("schema:citation <{}>", CITATION),
            ],
        )?;
        for run in &meta.runs {
            let mut po = vec![
                "a prov:Activity".to_string(),
                format!("rdfs:label {}", quoted(&run.run_name)),
                format!(
                    "prov:startedAtTime {}^^xsd:dateTime",
                    quoted(&run.first_start)
                ),
                format!("prov:endedAtTime {}^^xsd:dateTime", quoted(&run.last_end)),
            ];
            if !run.commit_urls.is_empty() {
                let urls: Vec<String> =
                    run.commit_urls.iter().map(|u| format!("<{}>", u)).collect();
                po.push(format!("prov:wasAssociatedWith {}", urls.join(", ")));
            }
            po.push(format!("biosample_ont:llmModel {}", quoted(&run.model)));
            po.push(format!("schema:isPartOf <{}>", rel.iri()));
            write_block(w, &rel.run_iri(&run.run_name), &po)?;
        }
        writeln!(w)
    }

    fn write_record<W: Write>(&self, w: &mut W, rec: &Record) -> std::io::Result<()> {
        if !rec.annotations.is_empty() {
            let nodes: Vec<String> = rec
                .annotations
                .iter()
                .map(|a| format!("<{}>", a.iri))
                .collect();
            writeln!(
                w,
                "<{}> schema:additionalProperty {} .",
                rec.biosample_iri,
                nodes.join(", ")
            )?;
        }
        for a in &rec.annotations {
            let values: Vec<String> = a.values.iter().map(|v| quoted(v)).collect();
            write_block(
                w,
                &a.iri,
                &[
                    "a schema:PropertyValue".to_string(),
                    format!("schema:propertyID {}", quoted(&a.field)),
                    format!("schema:value {}", values.join(", ")),
                    format!("schema:valueReference <{}>", a.term_uri),
                    format!("biosample_ont:exactMatch {}", a.exact_match),
                    format!("prov:wasGeneratedBy <{}>", rec.run_iri),
                ],
            )?;
        }
        for t in &rec.terms {
            let mut po = Vec::new();
            if t.declare_type {
                po.push("a schema:DefinedTerm".to_string());
            }
            po.push(format!("rdfs:label {}", quoted(&t.label)));
            write_block(w, &t.term_uri, &po)?;
        }
        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serializer::test_data::*;

    #[test]
    fn test_turtle_record() {
        let out = render(&TurtleSerializer::new(), Some(&meta()), &[record()]);
        assert!(out.starts_with("@prefix schema: <http://schema.org/> ."));
        assert!(out.contains("@prefix prov: <http://www.w3.org/ns/prov#> ."));
        assert!(out.contains(&format!(
            "<http://identifiers.org/biosample/SAMN1> schema:additionalProperty <{}> .",
            NODE
        )));
        assert!(out.contains(&format!("<{}> a schema:PropertyValue ;", NODE)));
        assert!(out.contains("    schema:value \"REV-ERB a\", \"REV-ERB b\" ;"));
        assert!(out.contains(&format!("    schema:valueReference <{}> ;", TERM)));
        assert!(out.contains("    biosample_ont:exactMatch true ;"));
        assert!(out.contains(&format!("    prov:wasGeneratedBy <{}> .", RUN1)));
        assert!(out.contains(&format!(
            "<{}> a schema:DefinedTerm ;\n    rdfs:label \"Nr1d1\" .",
            TERM
        )));
        assert!(out.contains(&format!("<{}> a schema:Dataset ;", REL)));
        assert!(out.contains("    schema:datePublished \"2026-09-26\"^^xsd:date ;"));
        assert!(out.contains(
            "    prov:wasAssociatedWith <https://example.org/c1>, <https://example.org/c2> ;"
        ));
    }
}
