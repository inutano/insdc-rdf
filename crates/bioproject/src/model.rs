use insdc_rdf_core::prefix::*;

/// A parsed BioProject record.
#[derive(Debug, Clone, PartialEq)]
pub struct BioProjectRecord {
    pub accession: String,
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub organism_name: Option<String>,
    pub taxonomy_id: Option<String>,
    pub release_date: Option<String>,
    pub submission_date: Option<String>,
}

impl BioProjectRecord {
    /// IRI: http://identifiers.org/bioproject/{accession}
    pub fn iri(&self) -> String {
        format!("{}{}", IDORG_BIOPROJECT, self.accession)
    }

    /// Label: use name if available, otherwise title
    pub fn label(&self) -> Option<&str> {
        self.name.as_deref().or(self.title.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iri() {
        let rec = BioProjectRecord {
            accession: "PRJNA3".to_string(),
            name: None,
            title: None,
            description: None,
            organism_name: None,
            taxonomy_id: None,
            release_date: None,
            submission_date: None,
        };
        assert_eq!(rec.iri(), "http://identifiers.org/bioproject/PRJNA3");
    }

    #[test]
    fn test_label_prefers_name() {
        let rec = BioProjectRecord {
            accession: "PRJNA3".to_string(),
            name: Some("Project Name".to_string()),
            title: Some("Project Title".to_string()),
            description: None,
            organism_name: None,
            taxonomy_id: None,
            release_date: None,
            submission_date: None,
        };
        assert_eq!(rec.label(), Some("Project Name"));
    }

    #[test]
    fn test_label_falls_back_to_title() {
        let rec = BioProjectRecord {
            accession: "PRJNA3".to_string(),
            name: None,
            title: Some("Project Title".to_string()),
            description: None,
            organism_name: None,
            taxonomy_id: None,
            release_date: None,
            submission_date: None,
        };
        assert_eq!(rec.label(), Some("Project Title"));
    }
}
