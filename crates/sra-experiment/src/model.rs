use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, PartialEq)]
pub enum LibraryLayout {
    Single,
    Paired {
        nominal_length: Option<f64>,
        nominal_sdev: Option<f64>,
    },
}

#[derive(Debug, Clone)]
pub struct SraExperimentRecord {
    pub accession: String,
    pub title: Option<String>,
    pub design_description: Option<String>,
    pub library_name: Option<String>,
    pub library_strategy: Option<String>,
    pub library_source: Option<String>,
    pub library_selection: Option<String>,
    pub library_layout: Option<LibraryLayout>,
    pub library_construction_protocol: Option<String>,
    pub platform: Option<String>,
    pub instrument_model: Option<String>,
}

impl SraExperimentRecord {
    pub fn iri(&self) -> String {
        format!("{}{}", IDORG_SRA, self.accession)
    }
}

/// Convert a metadata value to a URI-safe local name.
/// Strips leading/trailing whitespace, replaces internal spaces with `_`.
pub fn to_uri_local(s: &str) -> String {
    s.trim().replace(' ', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iri() {
        let rec = SraExperimentRecord {
            accession: "SRX000001".to_string(),
            title: None, design_description: None, library_name: None,
            library_strategy: None, library_source: None, library_selection: None,
            library_layout: None, library_construction_protocol: None,
            platform: None, instrument_model: None,
        };
        assert_eq!(rec.iri(), "http://identifiers.org/insdc.sra/SRX000001");
    }

    #[test]
    fn test_to_uri_local_spaces() {
        assert_eq!(to_uri_local("Illumina NovaSeq 6000"), "Illumina_NovaSeq_6000");
    }

    #[test]
    fn test_to_uri_local_no_spaces() {
        assert_eq!(to_uri_local("WGS"), "WGS");
    }

    #[test]
    fn test_to_uri_local_trim() {
        assert_eq!(to_uri_local("  GENOMIC  "), "GENOMIC");
    }

    #[test]
    fn test_library_layout_equality() {
        assert_eq!(LibraryLayout::Single, LibraryLayout::Single);
        assert_eq!(
            LibraryLayout::Paired { nominal_length: Some(300.0), nominal_sdev: None },
            LibraryLayout::Paired { nominal_length: Some(300.0), nominal_sdev: None },
        );
    }
}
