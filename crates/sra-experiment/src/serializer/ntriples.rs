use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use super::Serializer;
use crate::model::{to_uri_local, LibraryLayout, SraExperimentRecord};
use insdc_rdf_core::escape::escape_ntriples_string;
use insdc_rdf_core::prefix::*;

static BLANK_NODE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_bnode() -> String {
    format!("_:b{}", BLANK_NODE_COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Debug, Clone, Default)]
pub struct NTriplesSerializer;

impl Serializer for NTriplesSerializer {
    fn write_header<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &SraExperimentRecord,
    ) -> std::io::Result<()> {
        let subj = record.iri();
        let dra = DDBJ_DRA_ONT;

        // rdf:type
        writeln!(writer, "<{}> <{}> <{}Experiment> .", subj, RDF_TYPE, dra)?;

        // rdfs:label
        if let Some(ref title) = record.title {
            writeln!(
                writer,
                "<{}> <{}label> \"{}: {}\" .",
                subj,
                RDFS,
                escape_ntriples_string(&record.accession),
                escape_ntriples_string(title)
            )?;
        }

        // dct:identifier
        writeln!(
            writer,
            "<{}> <{}identifier> \"{}\" .",
            subj,
            DCT,
            escape_ntriples_string(&record.accession)
        )?;

        // dra_ont:title
        if let Some(ref title) = record.title {
            writeln!(
                writer,
                "<{}> <{}title> \"{}\" .",
                subj,
                dra,
                escape_ntriples_string(title)
            )?;
        }

        // dra_ont:designDescription
        if let Some(ref desc) = record.design_description {
            writeln!(
                writer,
                "<{}> <{}designDescription> \"{}\" .",
                subj,
                dra,
                escape_ntriples_string(desc)
            )?;
        }

        // Platform blank node
        if record.platform.is_some() || record.instrument_model.is_some() {
            let pbn = next_bnode();
            writeln!(writer, "<{}> <{}platform> {} .", subj, dra, pbn)?;
            if let Some(ref plat) = record.platform {
                writeln!(
                    writer,
                    "{} <{}> <{}{}> .",
                    pbn,
                    RDF_TYPE,
                    dra,
                    to_uri_local(plat)
                )?;
            }
            if let Some(ref im) = record.instrument_model {
                writeln!(
                    writer,
                    "{} <{}instrumentModel> <{}{}> .",
                    pbn,
                    dra,
                    dra,
                    to_uri_local(im)
                )?;
            }
        }

        // Design blank node
        let has_design = record.library_name.is_some()
            || record.library_strategy.is_some()
            || record.library_source.is_some()
            || record.library_selection.is_some()
            || record.library_construction_protocol.is_some()
            || record.library_layout.is_some();

        if has_design {
            let dbn = next_bnode();
            writeln!(writer, "<{}> <{}design> {} .", subj, dra, dbn)?;
            writeln!(writer, "{} <{}> <{}ExperimentDesign> .", dbn, RDF_TYPE, dra)?;

            if let Some(ref name) = record.library_name {
                writeln!(
                    writer,
                    "{} <{}libraryName> \"{}\" .",
                    dbn,
                    dra,
                    escape_ntriples_string(name)
                )?;
            }
            if let Some(ref strategy) = record.library_strategy {
                writeln!(
                    writer,
                    "{} <{}libraryStrategy> <{}{}> .",
                    dbn,
                    dra,
                    dra,
                    to_uri_local(strategy)
                )?;
            }
            if let Some(ref source) = record.library_source {
                writeln!(
                    writer,
                    "{} <{}librarySource> <{}{}> .",
                    dbn,
                    dra,
                    dra,
                    to_uri_local(source)
                )?;
            }
            if let Some(ref sel) = record.library_selection {
                writeln!(
                    writer,
                    "{} <{}librarySelection> <{}{}> .",
                    dbn,
                    dra,
                    dra,
                    to_uri_local(sel)
                )?;
            }
            if let Some(ref proto) = record.library_construction_protocol {
                writeln!(
                    writer,
                    "{} <{}libraryConstructionProtocol> \"{}\" .",
                    dbn,
                    dra,
                    escape_ntriples_string(proto)
                )?;
            }

            // Library layout sub-blank-node
            if let Some(ref layout) = record.library_layout {
                let lbn = next_bnode();
                writeln!(writer, "{} <{}libraryLayout> {} .", dbn, dra, lbn)?;
                match layout {
                    LibraryLayout::Single => {
                        writeln!(writer, "{} <{}> <{}SINGLE> .", lbn, RDF_TYPE, dra)?;
                    }
                    LibraryLayout::Paired {
                        nominal_length,
                        nominal_sdev,
                    } => {
                        writeln!(writer, "{} <{}> <{}PAIRED> .", lbn, RDF_TYPE, dra)?;
                        let xsd_decimal = format!("{}decimal", XSD);
                        if let Some(len) = nominal_length {
                            writeln!(
                                writer,
                                "{} <{}nominalLength> \"{}\"^^<{}> .",
                                lbn,
                                dra,
                                format_decimal(*len),
                                xsd_decimal
                            )?;
                        }
                        if let Some(sd) = nominal_sdev {
                            writeln!(
                                writer,
                                "{} <{}nominalSdev> \"{}\"^^<{}> .",
                                lbn,
                                dra,
                                format_decimal(*sd),
                                xsd_decimal
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

/// Format an f64 as a clean decimal string (no trailing ".0" for integers).
fn format_decimal(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

impl NTriplesSerializer {
    pub fn new() -> Self {
        NTriplesSerializer
    }

    pub fn record_to_string(&self, record: &SraExperimentRecord) -> String {
        let mut buf = Vec::new();
        self.write_record(&mut buf, record).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_record() -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "SRX000001".to_string(),
            title: Some("RNA-Seq of human brain tissue".to_string()),
            design_description: Some("Total RNA was extracted and sequenced.".to_string()),
            library_name: Some("Brain RNA lib1".to_string()),
            library_strategy: Some("RNA-Seq".to_string()),
            library_source: Some("TRANSCRIPTOMIC".to_string()),
            library_selection: Some("cDNA".to_string()),
            library_layout: Some(LibraryLayout::Paired {
                nominal_length: Some(300.0),
                nominal_sdev: Some(25.5),
            }),
            library_construction_protocol: Some("TruSeq RNA protocol".to_string()),
            platform: Some("ILLUMINA".to_string()),
            instrument_model: Some("Illumina NovaSeq 6000".to_string()),
        }
    }

    fn minimal_record() -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "SRX999999".to_string(),
            title: None,
            design_description: None,
            library_name: None,
            library_strategy: None,
            library_source: None,
            library_selection: None,
            library_layout: None,
            library_construction_protocol: None,
            platform: None,
            instrument_model: None,
        }
    }

    #[test]
    fn test_every_line_ends_with_space_dot() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        for line in s.lines() {
            assert!(
                line.ends_with(" ."),
                "Line does not end with ' .': {:?}",
                line
            );
        }
    }

    #[test]
    fn test_contains_full_iris() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(s.contains("<http://identifiers.org/insdc.sra/SRX000001>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/Experiment>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/ILLUMINA>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/Illumina_NovaSeq_6000>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/RNA-Seq>"));
        assert!(s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/TRANSCRIPTOMIC>"));
    }

    #[test]
    fn test_no_prefixed_names() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(
            !s.contains("insdc_sra:"),
            "should not contain insdc_sra: prefix"
        );
        assert!(
            !s.contains("dra_ont:"),
            "should not contain dra_ont: prefix"
        );
        assert!(!s.contains("dct:"), "should not contain dct: prefix");
        assert!(!s.contains("rdfs:"), "should not contain rdfs: prefix");
        assert!(!s.contains("xsd:"), "should not contain xsd: prefix");
    }

    #[test]
    fn test_blank_nodes_present() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(s.contains("_:b"), "should contain blank nodes");
        // There should be 3 distinct blank node IDs (platform, design, layout)
        let bnodes: std::collections::HashSet<&str> = s
            .split_whitespace()
            .filter(|t| t.starts_with("_:b"))
            .collect();
        assert_eq!(
            bnodes.len(),
            3,
            "expected 3 distinct blank nodes, got {:?}",
            bnodes
        );
    }

    #[test]
    fn test_decimal_typing() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&full_record());
        let xsd_decimal = format!("{}decimal", XSD);
        assert!(
            s.contains(&format!("\"300\"^^<{}>", xsd_decimal)),
            "nominalLength should be typed decimal"
        );
        assert!(
            s.contains(&format!("\"25.5\"^^<{}>", xsd_decimal)),
            "nominalSdev should be typed decimal"
        );
    }

    #[test]
    fn test_minimal_record() {
        let ser = NTriplesSerializer::new();
        let s = ser.record_to_string(&minimal_record());
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "minimal record should have exactly 2 lines: type + identifier"
        );
        assert!(s.contains("Experiment"), "should have type triple");
        assert!(s.contains("identifier"), "should have identifier triple");
        assert!(!s.contains("_:b"), "should have no blank nodes");
    }
}
