use std::io::Write;

use insdc_rdf_core::escape::escape_ntriples_string as esc;
use insdc_rdf_core::prefix::*;

use super::Serializer;
use crate::meta::{CrateMeta, CITATION};
use crate::model::Record;

#[derive(Debug, Clone, Default)]
pub struct NTriplesSerializer;

impl NTriplesSerializer {
    pub fn new() -> Self {
        NTriplesSerializer
    }
}

impl Serializer for NTriplesSerializer {
    fn write_header<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    fn write_provenance<W: Write>(&self, w: &mut W, meta: &CrateMeta) -> std::io::Result<()> {
        let rel = &meta.release;
        let r = rel.iri();
        writeln!(w, "<{}> <{}> <{}Dataset> .", r, RDF_TYPE, SCHEMA)?;
        writeln!(w, "<{}> <{}name> \"{}\" .", r, SCHEMA, esc(&rel.name))?;
        writeln!(
            w,
            "<{}> <{}version> \"{}\" .",
            r,
            SCHEMA,
            esc(&rel.release_id)
        )?;
        writeln!(
            w,
            "<{}> <{}datePublished> \"{}\"^^<{}date> .",
            r,
            SCHEMA,
            esc(&rel.date_published),
            XSD
        )?;
        writeln!(w, "<{}> <{}license> <{}> .", r, SCHEMA, rel.license)?;
        writeln!(w, "<{}> <{}citation> <{}> .", r, SCHEMA, CITATION)?;
        for run in &meta.runs {
            let s = rel.run_iri(&run.run_name);
            writeln!(w, "<{}> <{}> <{}Activity> .", s, RDF_TYPE, PROV)?;
            writeln!(w, "<{}> <{}label> \"{}\" .", s, RDFS, esc(&run.run_name))?;
            writeln!(
                w,
                "<{}> <{}startedAtTime> \"{}\"^^<{}dateTime> .",
                s,
                PROV,
                esc(&run.first_start),
                XSD
            )?;
            writeln!(
                w,
                "<{}> <{}endedAtTime> \"{}\"^^<{}dateTime> .",
                s,
                PROV,
                esc(&run.last_end),
                XSD
            )?;
            for url in &run.commit_urls {
                writeln!(w, "<{}> <{}wasAssociatedWith> <{}> .", s, PROV, url)?;
            }
            writeln!(
                w,
                "<{}> <{}llmModel> \"{}\" .",
                s,
                DDBJ_BIOSAMPLE_ONT,
                esc(&run.model)
            )?;
            writeln!(w, "<{}> <{}isPartOf> <{}> .", s, SCHEMA, r)?;
        }
        Ok(())
    }

    fn write_record<W: Write>(&self, w: &mut W, rec: &Record) -> std::io::Result<()> {
        for a in &rec.annotations {
            writeln!(
                w,
                "<{}> <{}additionalProperty> <{}> .",
                rec.biosample_iri, SCHEMA, a.iri
            )?;
            writeln!(w, "<{}> <{}> <{}PropertyValue> .", a.iri, RDF_TYPE, SCHEMA)?;
            writeln!(w, "<{}> <{}name> \"{}\" .", a.iri, SCHEMA, esc(&a.field))?;
            writeln!(
                w,
                "<{}> <{}propertyID> \"{}\" .",
                a.iri,
                SCHEMA,
                esc(&a.field)
            )?;
            for v in &a.values {
                writeln!(w, "<{}> <{}value> \"{}\" .", a.iri, SCHEMA, esc(v))?;
            }
            writeln!(
                w,
                "<{}> <{}valueReference> <{}> .",
                a.iri, SCHEMA, a.term_uri
            )?;
            writeln!(
                w,
                "<{}> <{}exactMatch> \"{}\"^^<{}boolean> .",
                a.iri, DDBJ_BIOSAMPLE_ONT, a.exact_match, XSD
            )?;
            writeln!(
                w,
                "<{}> <{}wasGeneratedBy> <{}> .",
                a.iri, PROV, rec.run_iri
            )?;
        }
        for t in &rec.terms {
            if t.declare_type {
                writeln!(
                    w,
                    "<{}> <{}> <{}DefinedTerm> .",
                    t.term_uri, RDF_TYPE, SCHEMA
                )?;
            }
            writeln!(
                w,
                "<{}> <{}label> \"{}\" .",
                t.term_uri,
                RDFS,
                esc(&t.label)
            )?;
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
    fn test_record_triples() {
        let out = render(&NTriplesSerializer::new(), None, &[record()]);
        let expected = format!(
            "<http://identifiers.org/biosample/SAMN1> <http://schema.org/additionalProperty> <{n}> .\n\
             <{n}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://schema.org/PropertyValue> .\n\
             <{n}> <http://schema.org/name> \"knockout_gene\" .\n\
             <{n}> <http://schema.org/propertyID> \"knockout_gene\" .\n\
             <{n}> <http://schema.org/value> \"REV-ERB a\" .\n\
             <{n}> <http://schema.org/value> \"REV-ERB b\" .\n\
             <{n}> <http://schema.org/valueReference> <{t}> .\n\
             <{n}> <http://ddbj.nig.ac.jp/ontologies/biosample/exactMatch> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n\
             <{n}> <http://www.w3.org/ns/prov#wasGeneratedBy> <{r}> .\n\
             <{t}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://schema.org/DefinedTerm> .\n\
             <{t}> <http://www.w3.org/2000/01/rdf-schema#label> \"Nr1d1\" .\n",
            n = NODE,
            t = TERM,
            r = RUN1
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn test_label_only_term() {
        let mut rec = record();
        rec.terms[0].declare_type = false;
        let out = render(&NTriplesSerializer::new(), None, &[rec]);
        assert!(!out.contains("DefinedTerm"));
        assert!(out.contains(&format!(
            "<{}> <http://www.w3.org/2000/01/rdf-schema#label> \"Nr1d1\" .",
            TERM
        )));
    }

    #[test]
    fn test_provenance_triples() {
        let out = render(&NTriplesSerializer::new(), Some(&meta()), &[]);
        assert!(out.contains(&format!(
            "<{}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://schema.org/Dataset> .",
            REL
        )));
        assert!(out.contains(&format!(
            "<{}> <http://schema.org/name> \"Release \\\"one\\\"\" .",
            REL
        )));
        assert!(out.contains(&format!(
            "<{}> <http://schema.org/version> \"rel-1\" .",
            REL
        )));
        assert!(out.contains(&format!("<{}> <http://schema.org/datePublished> \"2026-09-26\"^^<http://www.w3.org/2001/XMLSchema#date> .", REL)));
        assert!(out.contains(&format!(
            "<{}> <http://schema.org/license> <https://creativecommons.org/licenses/by/4.0/> .",
            REL
        )));
        assert!(out.contains(&format!(
            "<{}> <http://schema.org/citation> <https://doi.org/10.1101/2025.02.17.638570> .",
            REL
        )));
        assert!(out.contains(&format!("<{}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/prov#Activity> .", RUN1)));
        assert!(out.contains(&format!(
            "<{}> <http://www.w3.org/2000/01/rdf-schema#label> \"r1\" .",
            RUN1
        )));
        assert!(out.contains(&format!("<{}> <http://www.w3.org/ns/prov#startedAtTime> \"2026-05-30T17:20:09+00:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .", RUN1)));
        assert!(out.contains(&format!("<{}> <http://www.w3.org/ns/prov#endedAtTime> \"2026-05-30T22:16:22+00:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .", RUN1)));
        assert_eq!(
            out.matches("<http://www.w3.org/ns/prov#wasAssociatedWith>")
                .count(),
            2
        );
        assert!(out.contains(&format!(
            "<{}> <http://ddbj.nig.ac.jp/ontologies/biosample/llmModel> \"mistral-small3.1:24b\" .",
            RUN1
        )));
        assert!(out.contains(&format!(
            "<{}> <http://schema.org/isPartOf> <{}> .",
            RUN1, REL
        )));
    }

    #[test]
    fn test_record_without_annotations_writes_nothing() {
        let mut rec = record();
        rec.annotations.clear();
        rec.terms.clear();
        assert_eq!(render(&NTriplesSerializer::new(), None, &[rec]), "");
    }
}
