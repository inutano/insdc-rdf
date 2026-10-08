//! Release and run metadata read from a bsllmner-mk2 RO-Crate.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context};
use insdc_rdf_core::iri::is_http_iri;

/// Base URL under which bsllmner-mk2 releases are published.
pub const RELEASE_BASE: &str = "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/";

/// The bsllmner-mk2 publication. The RO-Crate root carries no citation, so it is fixed here.
pub const CITATION: &str = "https://doi.org/10.1101/2025.02.17.638570";

const RUN_INDEX_COLUMNS: [&str; 7] = [
    "run_name",
    "dataset",
    "model",
    "first_start",
    "last_end",
    "code_commit",
    "result_file",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub release_id: String,
    pub name: String,
    pub date_published: String,
    pub license: String,
}

impl Release {
    pub fn iri(&self) -> String {
        format!("{}{}/", RELEASE_BASE, self.release_id)
    }

    /// The RO-Crate's `#run-<name>` id, resolved against the crate root.
    pub fn run_iri(&self, run_name: &str) -> String {
        format!("{}#run-{}", self.iri(), run_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub run_name: String,
    pub dataset: String,
    pub model: String,
    pub first_start: String,
    pub last_end: String,
    pub commit_urls: Vec<String>,
    pub result_file: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrateMeta {
    pub release: Release,
    pub runs: Vec<Run>,
}

fn is_safe_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The release id is the crate directory's name, e.g. `2026-06_mistral-small3.1-24b-v2`.
pub fn release_id_from_dir(dir: &Path) -> anyhow::Result<String> {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("cannot take a release id from {:?}", dir))?;
    if !is_safe_name(name) {
        bail!(
            "release id {:?} (the input directory name) must match [A-Za-z0-9._-]+",
            name
        );
    }
    Ok(name.to_string())
}

/// Reads release and run metadata from an unpacked RO-Crate directory.
pub fn load_crate(dir: &Path) -> anyhow::Result<CrateMeta> {
    let dir = dir
        .canonicalize()
        .with_context(|| format!("input directory {:?}", dir))?;
    let release_id = release_id_from_dir(&dir)?;

    let meta_path = dir.join("ro-crate-metadata.json");
    let text =
        fs::read_to_string(&meta_path).with_context(|| format!("reading {:?}", meta_path))?;
    let meta: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {:?}", meta_path))?;
    let (release, commits) = parse_ro_crate(&meta, release_id)?;

    let tsv_path = dir.join("provenance").join("run_index.tsv");
    let tsv = fs::read_to_string(&tsv_path).with_context(|| format!("reading {:?}", tsv_path))?;
    let runs = parse_run_index(&tsv, &commits)?;

    Ok(CrateMeta { release, runs })
}

/// Reads the root Dataset and the `#bsllmner2-<short commit>` SoftwareApplications.
/// Returns the release and a map from short commit hash to commit URL.
pub fn parse_ro_crate(
    meta: &serde_json::Value,
    release_id: String,
) -> anyhow::Result<(Release, HashMap<String, String>)> {
    let graph = meta["@graph"]
        .as_array()
        .ok_or_else(|| anyhow!("ro-crate-metadata.json has no @graph array"))?;
    let root = graph
        .iter()
        .find(|e| e["@id"] == "./")
        .ok_or_else(|| anyhow!("ro-crate-metadata.json has no root Dataset \"./\""))?;
    let text = |key: &str| -> anyhow::Result<String> {
        root[key]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("root Dataset has no string {:?}", key))
    };
    let license = root["license"]["@id"]
        .as_str()
        .or_else(|| root["license"].as_str())
        .filter(|l| is_http_iri(l))
        .ok_or_else(|| anyhow!("root Dataset has no http(s) license IRI"))?
        .to_string();
    let release = Release {
        release_id,
        name: text("name")?,
        date_published: text("datePublished")?,
        license,
    };

    let mut commits = HashMap::new();
    for entity in graph {
        let Some(short) = entity["@id"]
            .as_str()
            .and_then(|id| id.strip_prefix("#bsllmner2-"))
        else {
            continue;
        };
        let url = entity["url"]
            .as_str()
            .filter(|u| is_http_iri(u))
            .ok_or_else(|| anyhow!("#bsllmner2-{} has no http(s) url", short))?;
        commits.insert(short.to_string(), url.to_string());
    }
    Ok((release, commits))
}

/// Parses `provenance/run_index.tsv`, keeping file order. Each `code_commit`
/// (short hashes joined by `+`) is resolved to commit URLs through `commits`.
pub fn parse_run_index(tsv: &str, commits: &HashMap<String, String>) -> anyhow::Result<Vec<Run>> {
    let mut lines = tsv.lines();
    let header: Vec<&str> = lines
        .next()
        .ok_or_else(|| anyhow!("run_index.tsv is empty"))?
        .split('\t')
        .collect();
    let mut idx: HashMap<&str, usize> = HashMap::new();
    for col in RUN_INDEX_COLUMNS {
        let i = header
            .iter()
            .position(|h| *h == col)
            .ok_or_else(|| anyhow!("run_index.tsv has no column {:?}", col))?;
        idx.insert(col, i);
    }

    let mut runs = Vec::new();
    for (n, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let line_no = n + 2;
        let cols: Vec<&str> = line.split('\t').collect();
        let get = |col: &str| -> anyhow::Result<String> {
            cols.get(idx[col])
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("run_index.tsv line {}: no value for {:?}", line_no, col))
        };
        let run_name = get("run_name")?;
        let dataset = get("dataset")?;
        for (kind, value) in [("run_name", &run_name), ("dataset", &dataset)] {
            if !is_safe_name(value) {
                bail!(
                    "run_index.tsv line {}: {} {:?} must match [A-Za-z0-9._-]+",
                    line_no,
                    kind,
                    value
                );
            }
        }
        let commit_urls = get("code_commit")?
            .split('+')
            .map(|short| {
                commits.get(short).cloned().ok_or_else(|| {
                    anyhow!(
                        "run_index.tsv line {}: commit {:?} has no #bsllmner2-{} entity in ro-crate-metadata.json",
                        line_no,
                        short,
                        short
                    )
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        runs.push(Run {
            run_name,
            dataset,
            model: get("model")?,
            first_start: get("first_start")?,
            last_end: get("last_end")?,
            commit_urls,
            result_file: get("result_file")?,
        });
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/bsllmner/2026-06_test-release"
        ))
    }

    const HEADER: &str =
        "run_name\tdataset\tmodel\tfirst_start\tlast_end\tcode_commit\tresult_file\n";

    #[test]
    fn test_load_crate_release() {
        let meta = load_crate(&fixture_dir()).unwrap();
        let rel = &meta.release;
        assert_eq!(rel.release_id, "2026-06_test-release");
        assert_eq!(rel.name, "Test release of ontology-mapped named entities");
        assert_eq!(rel.date_published, "2026-09-26");
        assert_eq!(rel.license, "https://creativecommons.org/licenses/by/4.0/");
        assert_eq!(
            rel.iri(),
            "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_test-release/"
        );
        assert_eq!(
            rel.run_iri("chipatlas_hg38_part1"),
            "https://biosampleplus.s3.ap-northeast-1.amazonaws.com/releases/2026-06_test-release/#run-chipatlas_hg38_part1"
        );
    }

    #[test]
    fn test_load_crate_accepts_dot_path() {
        let meta = load_crate(&fixture_dir().join(".")).unwrap();
        assert_eq!(meta.release.release_id, "2026-06_test-release");
    }

    #[test]
    fn test_load_crate_runs_in_order_with_commits() {
        let meta = load_crate(&fixture_dir()).unwrap();
        assert_eq!(meta.runs.len(), 2);
        assert_eq!(
            meta.runs[0],
            Run {
                run_name: "chipatlas_hg38_part1".into(),
                dataset: "chipatlas_hg38".into(),
                model: "mistral-small3.1:24b".into(),
                first_start: "2026-05-27T07:37:37+00:00".into(),
                last_end: "2026-05-27T22:45:00+00:00".into(),
                commit_urls: vec![
                    "https://github.com/dbcls/bsllmner-mk2/commit/5a5744ee95cac05ce02bf272a690d3975261d0a0".into()
                ],
                result_file: "select_chipatlas_hg38_part1.json".into(),
            }
        );
        assert_eq!(
            meta.runs[1].commit_urls,
            vec![
                "https://github.com/dbcls/bsllmner-mk2/commit/5a5744ee95cac05ce02bf272a690d3975261d0a0".to_string(),
                "https://github.com/dbcls/bsllmner-mk2/commit/9a3828811f1ab4bac85615e0e1b2efcc6603265f".to_string(),
            ]
        );
    }

    #[test]
    fn test_release_id_rejects_unsafe_names() {
        assert!(release_id_from_dir(Path::new("/x/2026-06_ok.v2")).is_ok());
        assert!(release_id_from_dir(Path::new("/x/bad name")).is_err());
        assert!(release_id_from_dir(Path::new("/x/bad#id")).is_err());
    }

    #[test]
    fn test_unknown_commit_is_an_error() {
        let tsv = format!("{}r1\td1\tm\ts\te\tdeadbee\tf.json\n", HEADER);
        let err = parse_run_index(&tsv, &HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("deadbee"), "{}", err);
    }

    #[test]
    fn test_missing_column_is_an_error() {
        let err = parse_run_index("run_name\tdataset\nr1\td1\n", &HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("model"), "{}", err);
    }

    #[test]
    fn test_unsafe_run_name_or_dataset_is_an_error() {
        let mut commits = HashMap::new();
        commits.insert("abc".to_string(), "https://example.org/abc".to_string());
        for row in [
            "r 1\td1\tm\ts\te\tabc\tf.json\n",
            "r1\td#1\tm\ts\te\tabc\tf.json\n",
        ] {
            assert!(
                parse_run_index(&format!("{}{}", HEADER, row), &commits).is_err(),
                "{}",
                row
            );
        }
    }

    #[test]
    fn test_ro_crate_without_root_is_an_error() {
        let meta: serde_json::Value = serde_json::json!({"@graph": []});
        assert!(parse_ro_crate(&meta, "r".into()).is_err());
    }

    #[test]
    fn test_ro_crate_with_bad_commit_url_is_an_error() {
        let meta = serde_json::json!({"@graph": [
            {"@id": "./", "name": "n", "datePublished": "2026-01-01", "license": {"@id": "https://creativecommons.org/licenses/by/4.0/"}},
            {"@id": "#bsllmner2-abc", "url": "not a url"}
        ]});
        assert!(parse_ro_crate(&meta, "r".into()).is_err());
    }
}
