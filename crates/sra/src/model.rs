use insdc_rdf_core::prefix::*;

/// The type of an SRA accession, derived from the "Type" column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SraType {
    Run,
    Experiment,
    Sample,
    Study,
    Submission,
    Analysis,
}

impl SraType {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "RUN" => Some(SraType::Run),
            "EXPERIMENT" => Some(SraType::Experiment),
            "SAMPLE" => Some(SraType::Sample),
            "STUDY" => Some(SraType::Study),
            "SUBMISSION" => Some(SraType::Submission),
            "ANALYSIS" => Some(SraType::Analysis),
            _ => None,
        }
    }

    /// Returns the DRA ontology class name for this type.
    pub fn rdf_class(&self) -> &'static str {
        match self {
            SraType::Run => "Run",
            SraType::Experiment => "Experiment",
            SraType::Sample => "Sample",
            SraType::Study => "Study",
            SraType::Submission => "Submission",
            SraType::Analysis => "Analysis",
        }
    }
}

/// A parsed row from SRA_Accessions.tab.
#[derive(Debug, Clone)]
pub struct SraAccessionRecord {
    pub accession: String,
    pub sra_type: SraType,
    pub submission: Option<String>,
    pub updated: Option<String>,
    pub published: Option<String>,
    pub center: Option<String>,
    pub experiment: Option<String>,
    pub sample: Option<String>,
    pub study: Option<String>,
    pub biosample: Option<String>,
    pub bioproject: Option<String>,
}

impl SraAccessionRecord {
    /// IRI for this accession: http://identifiers.org/insdc.sra/{accession}
    pub fn iri(&self) -> String {
        format!("{}{}", IDORG_SRA, self.accession)
    }

    /// Collect all cross-link IRIs (non-self, non-empty related accessions).
    pub fn see_also_iris(&self) -> Vec<String> {
        let mut iris = Vec::new();

        if let Some(ref exp) = self.experiment {
            iris.push(format!("{}{}", IDORG_SRA, exp));
        }
        if let Some(ref sample) = self.sample {
            iris.push(format!("{}{}", IDORG_SRA, sample));
        }
        if let Some(ref study) = self.study {
            iris.push(format!("{}{}", IDORG_SRA, study));
        }
        if let Some(ref bs) = self.biosample {
            iris.push(format!("{}{}", IDORG_BIOSAMPLE, bs));
        }
        if let Some(ref bp) = self.bioproject {
            iris.push(format!("{}{}", IDORG_BIOPROJECT, bp));
        }

        iris
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sra_type_from_str() {
        assert_eq!(SraType::parse("RUN"), Some(SraType::Run));
        assert_eq!(SraType::parse("EXPERIMENT"), Some(SraType::Experiment));
        assert_eq!(SraType::parse("BOGUS"), None);
    }

    #[test]
    fn test_iri() {
        let rec = SraAccessionRecord {
            accession: "SRR000001".to_string(),
            sra_type: SraType::Run,
            submission: None, updated: None, published: None, center: None,
            experiment: None, sample: None, study: None,
            biosample: None, bioproject: None,
        };
        assert_eq!(rec.iri(), "http://identifiers.org/insdc.sra/SRR000001");
    }

    #[test]
    fn test_see_also_iris() {
        let rec = SraAccessionRecord {
            accession: "DRR000001".to_string(),
            sra_type: SraType::Run,
            submission: None, updated: None, published: None, center: None,
            experiment: Some("DRX000001".to_string()),
            sample: Some("DRS000001".to_string()),
            study: Some("DRP000001".to_string()),
            biosample: Some("SAMD00016353".to_string()),
            bioproject: Some("PRJDA38027".to_string()),
        };
        let iris = rec.see_also_iris();
        assert_eq!(iris.len(), 5);
        assert!(iris[0].ends_with("DRX000001"));
        assert!(iris[3].contains("biosample/SAMD00016353"));
        assert!(iris[4].contains("bioproject/PRJDA38027"));
    }

    #[test]
    fn test_see_also_skips_empty() {
        let rec = SraAccessionRecord {
            accession: "DRS000001".to_string(),
            sra_type: SraType::Sample,
            submission: None, updated: None, published: None, center: None,
            experiment: None, sample: None, study: None,
            biosample: Some("SAMD00016353".to_string()),
            bioproject: None,
        };
        let iris = rec.see_also_iris();
        assert_eq!(iris.len(), 1);
    }
}
