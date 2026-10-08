pub mod jsonld;
pub mod ntriples;
pub mod turtle;

use std::io::Write;

use crate::meta::CrateMeta;
use crate::model::Record;

pub trait Serializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
    fn write_provenance<W: Write>(&self, writer: &mut W, meta: &CrateMeta) -> std::io::Result<()>;
    fn write_record<W: Write>(&self, writer: &mut W, record: &Record) -> std::io::Result<()>;
    fn write_footer<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
}

#[cfg(test)]
pub(crate) mod test_data {
    use crate::meta::{CrateMeta, Release, Run};
    use crate::model::{Annotation, Record, TermOut};

    pub const NODE: &str =
        "http://ddbj.nig.ac.jp/biosample/SAMN1#bsllmner/rel-1/d1/knockout_gene/NCBIGene_353187";
    pub const TERM: &str = "http://purl.obolibrary.org/obo/NCBIGene_353187";
    pub const RUN1: &str =
        "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/rel-1/#run-r1";
    pub const REL: &str = "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/rel-1/";

    pub fn meta() -> CrateMeta {
        CrateMeta {
            release: Release {
                release_id: "rel-1".into(),
                name: "Release \"one\"".into(),
                date_published: "2026-09-26".into(),
                license: "https://creativecommons.org/licenses/by/4.0/".into(),
            },
            runs: vec![Run {
                run_name: "r1".into(),
                dataset: "d1".into(),
                model: "mistral-small3.1:24b".into(),
                first_start: "2026-05-30T17:20:09+00:00".into(),
                last_end: "2026-05-30T22:16:22+00:00".into(),
                commit_urls: vec![
                    "https://example.org/c1".into(),
                    "https://example.org/c2".into(),
                ],
                result_file: "f.json".into(),
            }],
        }
    }

    pub fn record_with(values: Vec<&str>, label: &str) -> Record {
        Record {
            biosample_iri: "http://identifiers.org/biosample/SAMN1".into(),
            run_iri: RUN1.into(),
            annotations: vec![Annotation {
                iri: NODE.into(),
                field: "knockout_gene".into(),
                values: values.into_iter().map(String::from).collect(),
                term_uri: TERM.into(),
                exact_match: true,
            }],
            terms: vec![TermOut {
                term_uri: TERM.into(),
                declare_type: true,
                label: label.into(),
            }],
        }
    }

    pub fn record() -> Record {
        record_with(vec!["REV-ERB a", "REV-ERB b"], "Nr1d1")
    }

    pub fn render<S: super::Serializer>(
        ser: &S,
        meta: Option<&CrateMeta>,
        records: &[Record],
    ) -> String {
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        if let Some(m) = meta {
            ser.write_provenance(&mut buf, m).unwrap();
        }
        for r in records {
            ser.write_record(&mut buf, r).unwrap();
        }
        ser.write_footer(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::jsonld::JsonLdSerializer;
    use super::ntriples::NTriplesSerializer;
    use super::test_data::*;
    use super::turtle::TurtleSerializer;

    #[test]
    fn test_escaping_in_all_formats() {
        let nasty = "say \"hi\" \\ back\nnew\ttab β-catenin";
        let rec = record_with(vec![nasty], nasty);

        let nt = render(&NTriplesSerializer::new(), Some(&meta()), &[rec.clone()]);
        for line in nt.lines() {
            assert!(line.ends_with(" ."), "unterminated: {}", line);
        }
        assert!(nt.contains("\"say \\\"hi\\\" \\\\ back\\nnew\\ttab β-catenin\""));

        let ttl = render(&TurtleSerializer::new(), Some(&meta()), &[rec.clone()]);
        assert!(ttl.contains("\"say \\\"hi\\\" \\\\ back\\nnew\\ttab β-catenin\""));
        assert!(!ttl.contains("back\nnew"));

        let js = render(&JsonLdSerializer::new(), Some(&meta()), &[rec]);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(
            v[2]["schema:additionalProperty"][0]["schema:value"][0],
            nasty
        );
        assert_eq!(v[3]["rdfs:label"], nasty);
    }

    #[test]
    fn test_annotation_nodes_have_no_schema_name() {
        let rec = record();

        let nt = render(&NTriplesSerializer::new(), None, &[rec.clone()]);
        assert!(nt.lines().all(|l| !l.contains("<http://schema.org/name>")));
        assert!(nt
            .lines()
            .any(|l| l.ends_with("<http://schema.org/propertyID> \"knockout_gene\" .")));

        let ttl = render(&TurtleSerializer::new(), None, &[rec.clone()]);
        assert!(!ttl.contains("schema:name"));
        assert!(ttl.contains("    schema:propertyID \"knockout_gene\" ;"));

        let js = render(&JsonLdSerializer::new(), None, &[rec]);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        let node = &v[0]["schema:additionalProperty"][0];
        assert!(node.get("schema:name").is_none());
        assert_eq!(node["schema:propertyID"], "knockout_gene");
    }
}
