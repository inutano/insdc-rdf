//! Types read from bsllmner-mk2 result files, and the records written from them.

use std::collections::BTreeMap;

use serde::Deserialize;

/// One `results/select_<run>.json`. Only the fields the converter uses are declared;
/// serde skips the rest (candidate lists, timings, run metadata) without keeping them.
#[derive(Debug, Deserialize)]
pub struct ResultFile {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub extract: Extract,
    /// Field name -> selected terms. A field that was extracted but not mapped is absent.
    #[serde(default)]
    pub results: BTreeMap<String, Vec<ResultItem>>,
}

#[derive(Debug, Deserialize)]
pub struct Extract {
    pub accession: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResultItem {
    pub value: String,
    pub term_id: String,
    pub term_uri: String,
    pub label: String,
    pub exact_match: bool,
}

/// One annotation node: all items of an entry and field that share a term.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub iri: String,
    pub field: String,
    pub values: Vec<String>,
    pub term_uri: String,
    pub exact_match: bool,
}

/// Term statements still to be written: the type the first time a term is seen,
/// and a label the first time each (term, label) pair is seen.
#[derive(Debug, Clone, PartialEq)]
pub struct TermOut {
    pub term_uri: String,
    pub declare_type: bool,
    pub label: String,
}

/// Everything written for one entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub biosample_iri: String,
    pub run_iri: String,
    pub annotations: Vec<Annotation>,
    pub terms: Vec<TermOut>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_file_ignores_other_keys() {
        let f: ResultFile = serde_json::from_str(
            r#"{"entries": [{"extract": {"accession": "SAMN1", "extracted": {"x": null}, "raw_output": "{}"},
                             "search_results": {"x": {}}, "text2term_results": {}, "select_timings": {},
                             "ambiguous_fields": {}, "results": {"x": []}}],
                "run_metadata": {"run_name": "r"}, "evaluation": null, "performance": {}, "errors": []}"#,
        )
        .unwrap();
        assert_eq!(f.entries.len(), 1);
        assert_eq!(f.entries[0].extract.accession, "SAMN1");
        assert!(f.entries[0].results["x"].is_empty());
    }

    #[test]
    fn test_entry_without_results_key() {
        let e: Entry = serde_json::from_str(r#"{"extract": {"accession": "SAMN1"}}"#).unwrap();
        assert!(e.results.is_empty());
    }
}
