//! Turns one result entry into the annotation nodes and term statements to write.

use std::collections::HashSet;

use insdc_rdf_core::iri::{encode_fragment, is_http_iri};
use insdc_rdf_core::prefix::{DDBJ_BIOSAMPLE, IDORG_BIOSAMPLE};

use crate::meta::{Release, Run};
use crate::model::{Annotation, Entry, Record, TermOut};

/// True for `SAM` + N/D/E + an optional uppercase letter + digits (SAMN…, SAMD…, SAMEA…).
pub fn is_valid_accession(acc: &str) -> bool {
    let Some(rest) = acc.strip_prefix("SAM") else {
        return false;
    };
    let mut chars = rest.chars().peekable();
    if !matches!(chars.next(), Some('N' | 'D' | 'E')) {
        return false;
    }
    if chars.peek().is_some_and(|c| c.is_ascii_uppercase()) {
        chars.next();
    }
    let digits: String = chars.collect();
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// `http://ddbj.nig.ac.jp/biosample/{acc}#bsllmner/{release}/{dataset}/{field}/{term_local}`,
/// where `term_local` is the term id with `:` replaced by `_`.
pub fn annotation_iri(
    accession: &str,
    release_id: &str,
    dataset: &str,
    field: &str,
    term_id: &str,
) -> String {
    let fragment = format!(
        "bsllmner/{}/{}/{}/{}",
        release_id,
        dataset,
        field,
        term_id.replace(':', "_")
    );
    format!(
        "{}{}#{}",
        DDBJ_BIOSAMPLE,
        accession,
        encode_fragment(&fragment)
    )
}

/// Remembers, across the whole conversion, which terms and (term, label) pairs were written.
#[derive(Debug, Default)]
pub struct TermRegistry {
    terms: HashSet<String>,
    labels: HashSet<(String, String)>,
}

impl TermRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns what still has to be written for this term and label, or None if nothing does.
    pub fn register(&mut self, term_uri: &str, label: &str) -> Option<TermOut> {
        let declare_type = self.terms.insert(term_uri.to_string());
        let new_label = self
            .labels
            .insert((term_uri.to_string(), label.to_string()));
        (declare_type || new_label).then(|| TermOut {
            term_uri: term_uri.to_string(),
            declare_type,
            label: label.to_string(),
        })
    }

    pub fn term_count(&self) -> usize {
        self.terms.len()
    }
}

/// Builds the record for one entry of `run`.
///
/// Returns `Err(log line)` for a malformed accession; nothing is registered then, so a
/// term first seen in a skipped entry is still written where it is next used. The `Vec`
/// holds log lines for items dropped because their term IRI is unusable.
pub fn build_record(
    entry: &Entry,
    run: &Run,
    release: &Release,
    terms: &mut TermRegistry,
) -> Result<(Record, Vec<String>), String> {
    let acc = &entry.extract.accession;
    if !is_valid_accession(acc) {
        return Err(format!(
            "{}: skipped entry with malformed accession {:?}",
            run.run_name, acc
        ));
    }

    let mut annotations: Vec<Annotation> = Vec::new();
    let mut term_outs = Vec::new();
    let mut dropped = Vec::new();
    for (field, items) in &entry.results {
        let field_start = annotations.len();
        for item in items {
            if !is_http_iri(&item.term_uri) {
                dropped.push(format!(
                    "{}: {} {}: dropped item with unusable term IRI {:?}",
                    run.run_name, acc, field, item.term_uri
                ));
                continue;
            }
            match annotations[field_start..]
                .iter_mut()
                .find(|a| a.term_uri == item.term_uri)
            {
                Some(a) => {
                    if !a.values.contains(&item.value) {
                        a.values.push(item.value.clone());
                    }
                    a.exact_match |= item.exact_match;
                }
                None => annotations.push(Annotation {
                    iri: annotation_iri(
                        acc,
                        &release.release_id,
                        &run.dataset,
                        field,
                        &item.term_id,
                    ),
                    field: field.clone(),
                    values: vec![item.value.clone()],
                    term_uri: item.term_uri.clone(),
                    exact_match: item.exact_match,
                }),
            }
            if let Some(t) = terms.register(&item.term_uri, &item.label) {
                term_outs.push(t);
            }
        }
    }

    let record = Record {
        biosample_iri: format!("{}{}", IDORG_BIOSAMPLE, acc),
        run_iri: release.run_iri(&run.run_name),
        annotations,
        terms: term_outs,
    };
    Ok((record, dropped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{Release, Run};
    use crate::model::{Annotation, Entry, TermOut};

    fn release() -> Release {
        Release {
            release_id: "rel-1".into(),
            name: "n".into(),
            date_published: "2026-09-26".into(),
            license: "https://creativecommons.org/licenses/by/4.0/".into(),
        }
    }

    fn run(dataset: &str) -> Run {
        Run {
            run_name: format!("{}_r1", dataset),
            dataset: dataset.into(),
            model: "m".into(),
            first_start: "s".into(),
            last_end: "e".into(),
            commit_urls: vec![],
            result_file: "f.json".into(),
        }
    }

    fn entry(json: &str) -> Entry {
        serde_json::from_str(json).unwrap()
    }

    const HEPG2: &str = r#"{
      "extract": {"accession": "SAMD00270091", "extracted": {"cell_line": "HepG2"}},
      "results": {
        "cell_line": [{"value": "HepG2", "term_id": "CVCL:0027",
          "term_uri": "http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027", "label": "Hep-G2",
          "exact_match": true, "reasoning": "Exact match on oboInOwl:hasRelatedSynonym"}],
        "drug": []
      }
    }"#;

    const CELLOSAURUS_0027: &str = "http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027";

    #[test]
    fn test_accession_validation() {
        for ok in ["SAMN00000002", "SAMD00270091", "SAMEA6161248"] {
            assert!(is_valid_accession(ok), "{}", ok);
        }
        for bad in [
            "",
            "SAM",
            "SAMN",
            "SAMX0001",
            "SAMN12a",
            "BADACC123",
            "SAMEAB123",
            " SAMN1",
        ] {
            assert!(!is_valid_accession(bad), "{}", bad);
        }
    }

    #[test]
    fn test_annotation_iri() {
        assert_eq!(
            annotation_iri(
                "SAMD00270091",
                "2026-06_mistral-small3.1-24b-v2",
                "rnaseq_human_5y",
                "cell_line",
                "CVCL:0027"
            ),
            "http://ddbj.nig.ac.jp/biosample/SAMD00270091#bsllmner/2026-06_mistral-small3.1-24b-v2/rnaseq_human_5y/cell_line/CVCL_0027"
        );
    }

    #[test]
    fn test_annotation_iri_percent_encodes_term_id() {
        let iri = annotation_iri("SAMN1", "r", "d", "f", "X:a b#c%β");
        assert!(
            iri.ends_with("#bsllmner/r/d/f/X_a%20b%23c%25%CE%B2"),
            "{}",
            iri
        );
        assert!(insdc_rdf_core::iri::is_http_iri(&iri));
    }

    #[test]
    fn test_build_record_scalar_field() {
        let mut reg = TermRegistry::new();
        let (rec, dropped) =
            build_record(&entry(HEPG2), &run("rnaseq_human_5y"), &release(), &mut reg).unwrap();
        assert!(dropped.is_empty());
        assert_eq!(
            rec.biosample_iri,
            "http://identifiers.org/biosample/SAMD00270091"
        );
        assert_eq!(
            rec.run_iri,
            "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/rel-1/#run-rnaseq_human_5y_r1"
        );
        assert_eq!(
            rec.annotations,
            vec![Annotation {
                iri: "http://ddbj.nig.ac.jp/biosample/SAMD00270091#bsllmner/rel-1/rnaseq_human_5y/cell_line/CVCL_0027".into(),
                field: "cell_line".into(),
                values: vec!["HepG2".into()],
                term_uri: CELLOSAURUS_0027.into(),
                exact_match: true,
            }]
        );
        assert_eq!(
            rec.terms,
            vec![TermOut {
                term_uri: CELLOSAURUS_0027.into(),
                declare_type: true,
                label: "Hep-G2".into()
            }]
        );
    }

    #[test]
    fn test_build_record_merges_items_with_same_term() {
        let e = entry(
            r#"{"extract": {"accession": "SAMN13836220"}, "results": {"knockout_gene": [
            {"value": "REV-ERB a", "term_id": "NCBIGene:353187", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_353187", "label": "Nr1d1", "exact_match": false, "reasoning": null},
            {"value": "REV-ERB b", "term_id": "NCBIGene:353187", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_353187", "label": "Nr1d1", "exact_match": true, "reasoning": null},
            {"value": "REV-ERB a", "term_id": "NCBIGene:353187", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_353187", "label": "Nr1d1", "exact_match": false, "reasoning": null}
        ]}}"#,
        );
        let mut reg = TermRegistry::new();
        let (rec, _) = build_record(&e, &run("rnaseq_mouse"), &release(), &mut reg).unwrap();
        assert_eq!(rec.annotations.len(), 1);
        assert_eq!(
            rec.annotations[0].values,
            vec!["REV-ERB a".to_string(), "REV-ERB b".to_string()]
        );
        assert!(rec.annotations[0].exact_match);
        assert_eq!(rec.terms.len(), 1);
    }

    #[test]
    fn test_same_term_in_two_fields_gives_two_nodes() {
        let e = entry(
            r#"{"extract": {"accession": "SAMN1"}, "results": {
            "chip_antigen": [{"value": "CTCF", "term_id": "NCBIGene:10664", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_10664", "label": "CTCF", "exact_match": true, "reasoning": null}],
            "knockdown_gene": [{"value": "CTCF", "term_id": "NCBIGene:10664", "term_uri": "http://purl.obolibrary.org/obo/NCBIGene_10664", "label": "CTCF", "exact_match": true, "reasoning": null}]
        }}"#,
        );
        let mut reg = TermRegistry::new();
        let (rec, _) = build_record(&e, &run("d"), &release(), &mut reg).unwrap();
        assert_eq!(rec.annotations.len(), 2);
        assert_ne!(rec.annotations[0].iri, rec.annotations[1].iri);
        assert_eq!(rec.terms.len(), 1);
    }

    #[test]
    fn test_datasets_give_distinct_iris() {
        let mut reg = TermRegistry::new();
        let (a, _) =
            build_record(&entry(HEPG2), &run("chipatlas_hg38"), &release(), &mut reg).unwrap();
        let (b, _) =
            build_record(&entry(HEPG2), &run("rnaseq_human_5y"), &release(), &mut reg).unwrap();
        assert_ne!(a.annotations[0].iri, b.annotations[0].iri);
        assert_ne!(a.run_iri, b.run_iri);
        assert!(
            b.terms.is_empty(),
            "term already written by the first record"
        );
    }

    #[test]
    fn test_empty_or_missing_results_give_no_annotations() {
        let mut reg = TermRegistry::new();
        for json in [
            r#"{"extract": {"accession": "SAMEA6161248", "extracted": {}}, "results": {}}"#,
            r#"{"extract": {"accession": "SAMEA6161248"}}"#,
        ] {
            let (rec, dropped) =
                build_record(&entry(json), &run("d"), &release(), &mut reg).unwrap();
            assert!(rec.annotations.is_empty() && rec.terms.is_empty() && dropped.is_empty());
        }
    }

    #[test]
    fn test_malformed_accession_registers_nothing() {
        let mut reg = TermRegistry::new();
        let bad = HEPG2.replace("SAMD00270091", "BADACC123");
        let err = build_record(&entry(&bad), &run("d"), &release(), &mut reg).unwrap_err();
        assert!(err.contains("BADACC123"), "{}", err);
        assert_eq!(reg.term_count(), 0);
        let (rec, _) = build_record(&entry(HEPG2), &run("d"), &release(), &mut reg).unwrap();
        assert!(rec.terms[0].declare_type);
    }

    #[test]
    fn test_unusable_term_iri_is_dropped_and_logged() {
        let e = entry(
            r#"{"extract": {"accession": "SAMN00000003"}, "results": {"disease": [
            {"value": "x", "term_id": "X:1", "term_uri": "not an iri", "label": "x", "exact_match": false, "reasoning": null},
            {"value": "cancer", "term_id": "MONDO:0004992", "term_uri": "http://purl.obolibrary.org/obo/MONDO_0004992", "label": "cancer", "exact_match": true, "reasoning": null}
        ]}}"#,
        );
        let mut reg = TermRegistry::new();
        let (rec, dropped) = build_record(&e, &run("d"), &release(), &mut reg).unwrap();
        assert_eq!(rec.annotations.len(), 1);
        assert_eq!(dropped.len(), 1);
        assert!(dropped[0].contains("not an iri"), "{}", dropped[0]);
        assert_eq!(reg.term_count(), 1);
    }

    #[test]
    fn test_registry_emits_new_label_without_type() {
        let mut reg = TermRegistry::new();
        assert!(reg.register("http://t/1", "a").unwrap().declare_type);
        assert_eq!(reg.register("http://t/1", "a"), None);
        assert_eq!(
            reg.register("http://t/1", "b"),
            Some(TermOut {
                term_uri: "http://t/1".into(),
                declare_type: false,
                label: "b".into()
            })
        );
        assert_eq!(reg.term_count(), 1);
    }
}
