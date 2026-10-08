use std::io::Write;

use super::Serializer;
use crate::model::{to_uri_local, LibraryLayout, SraExperimentRecord};
use insdc_rdf_core::escape::escape_turtle_string;
use insdc_rdf_core::prefix::*;

#[derive(Debug, Clone, Default)]
pub struct TurtleSerializer;

impl Serializer for TurtleSerializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, "@prefix insdc_sra: <{}> .", IDORG_SRA)?;
        writeln!(writer, "@prefix dra_ont: <{}> .", DDBJ_DRA_ONT)?;
        writeln!(writer, "@prefix dct: <{}> .", DCT)?;
        writeln!(writer, "@prefix rdfs: <{}> .", RDFS)?;
        writeln!(writer, "@prefix xsd: <{}> .", XSD)?;
        writeln!(writer)?;
        Ok(())
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &SraExperimentRecord,
    ) -> std::io::Result<()> {
        let acc = &record.accession;

        // Collect top-level predicate-object pairs as strings.
        // Blank-node-valued properties are appended inline.
        let mut po_lines: Vec<String> = Vec::new();

        po_lines.push("a dra_ont:Experiment".to_string());

        // rdfs:label  "ACC: title" (only if title present)
        if let Some(ref title) = record.title {
            po_lines.push(format!(
                "rdfs:label \"{}: {}\"",
                escape_turtle_string(acc),
                escape_turtle_string(title)
            ));
        }

        po_lines.push(format!("dct:identifier \"{}\"", escape_turtle_string(acc)));

        if let Some(ref title) = record.title {
            po_lines.push(format!("dra_ont:title \"{}\"", escape_turtle_string(title)));
        }

        if let Some(ref desc) = record.design_description {
            po_lines.push(format!(
                "dra_ont:designDescription \"{}\"",
                escape_turtle_string(desc)
            ));
        }

        // Platform blank node
        if record.platform.is_some() || record.instrument_model.is_some() {
            let mut pbn_parts: Vec<String> = Vec::new();
            if let Some(ref plat) = record.platform {
                pbn_parts.push(format!("a {}", dra_ont_term(plat)));
            }
            if let Some(ref im) = record.instrument_model {
                pbn_parts.push(format!("dra_ont:instrumentModel {}", dra_ont_term(im)));
            }
            let bnode_body = format_blank_node(&pbn_parts);
            po_lines.push(format!("dra_ont:platform {}", bnode_body));
        }

        // Design blank node – only if at least one design-related field is present
        let has_design = record.library_name.is_some()
            || record.library_strategy.is_some()
            || record.library_source.is_some()
            || record.library_selection.is_some()
            || record.library_construction_protocol.is_some()
            || record.library_layout.is_some();

        if has_design {
            let mut dbn_parts: Vec<String> = Vec::new();
            dbn_parts.push("a dra_ont:ExperimentDesign".to_string());

            if let Some(ref name) = record.library_name {
                dbn_parts.push(format!(
                    "dra_ont:libraryName \"{}\"",
                    escape_turtle_string(name)
                ));
            }
            if let Some(ref strategy) = record.library_strategy {
                dbn_parts.push(format!(
                    "dra_ont:libraryStrategy {}",
                    dra_ont_term(strategy)
                ));
            }
            if let Some(ref source) = record.library_source {
                dbn_parts.push(format!("dra_ont:librarySource {}", dra_ont_term(source)));
            }
            if let Some(ref sel) = record.library_selection {
                dbn_parts.push(format!("dra_ont:librarySelection {}", dra_ont_term(sel)));
            }
            if let Some(ref proto) = record.library_construction_protocol {
                dbn_parts.push(format!(
                    "dra_ont:libraryConstructionProtocol \"{}\"",
                    escape_turtle_string(proto)
                ));
            }

            // Library layout sub-blank-node
            if let Some(ref layout) = record.library_layout {
                let layout_bnode = format_layout_blank_node(layout);
                dbn_parts.push(format!("dra_ont:libraryLayout {}", layout_bnode));
            }

            let bnode_body = format_blank_node(&dbn_parts);
            po_lines.push(format!("dra_ont:design {}", bnode_body));
        }

        // Write subject + predicate-object pairs
        writeln!(writer, "insdc_sra:{}", acc)?;
        let last = po_lines.len().saturating_sub(1);
        for (i, line) in po_lines.iter().enumerate() {
            if i < last {
                writeln!(writer, "  {} ;", line)?;
            } else {
                writeln!(writer, "  {} .", line)?;
            }
        }
        writeln!(writer)?;

        Ok(())
    }

    fn write_footer<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

/// Write a term in the DRA ontology namespace from a metadata value.
///
/// The value's local name (see `to_uri_local`) becomes a `dra_ont:` prefixed
/// name when it is a safe Turtle `PN_LOCAL`, and a full IRI otherwise. Values
/// such as `454 GS FLX+` contain characters a prefixed name cannot carry.
fn dra_ont_term(value: &str) -> String {
    let local = to_uri_local(value);
    if is_safe_pn_local(&local) {
        format!("dra_ont:{}", local)
    } else {
        format!("<{}{}>", DDBJ_DRA_ONT, local)
    }
}

/// A conservative test for a Turtle `PN_LOCAL` that needs no escaping:
/// non-empty ASCII `[A-Za-z0-9_-]`, plus `.` anywhere but first or last,
/// starting with `[A-Za-z0-9_]`.
fn is_safe_pn_local(local: &str) -> bool {
    let bytes = local.as_bytes();
    match (bytes.first(), bytes.last()) {
        (Some(&first), Some(&last)) => {
            (first.is_ascii_alphanumeric() || first == b'_')
                && last != b'.'
                && bytes
                    .iter()
                    .all(|&b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        }
        _ => false,
    }
}

/// Format a blank node body: `[\n  p o ;\n  p o\n]`
fn format_blank_node(parts: &[String]) -> String {
    if parts.is_empty() {
        return "[]".to_string();
    }
    let mut out = String::from("[\n");
    let last = parts.len().saturating_sub(1);
    for (i, part) in parts.iter().enumerate() {
        if i < last {
            out.push_str(&format!("    {} ;\n", part));
        } else {
            out.push_str(&format!("    {}\n", part));
        }
    }
    out.push_str("  ]");
    out
}

fn format_layout_blank_node(layout: &LibraryLayout) -> String {
    let mut parts: Vec<String> = Vec::new();
    match layout {
        LibraryLayout::Single => {
            parts.push("a dra_ont:SINGLE".to_string());
        }
        LibraryLayout::Paired {
            nominal_length,
            nominal_sdev,
        } => {
            parts.push("a dra_ont:PAIRED".to_string());
            if let Some(len) = nominal_length {
                parts.push(format!(
                    "dra_ont:nominalLength \"{}\"^^xsd:decimal",
                    format_decimal(*len)
                ));
            }
            if let Some(sd) = nominal_sdev {
                parts.push(format!(
                    "dra_ont:nominalSdev \"{}\"^^xsd:decimal",
                    format_decimal(*sd)
                ));
            }
        }
    }
    format_blank_node(&parts)
}

/// Format an f64 as a clean decimal string (no trailing ".0" for integers).
fn format_decimal(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

impl TurtleSerializer {
    pub fn new() -> Self {
        TurtleSerializer
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
    fn test_header_prefixes() {
        let ser = TurtleSerializer::new();
        let mut buf = Vec::new();
        ser.write_header(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("@prefix insdc_sra:"));
        assert!(s.contains("@prefix dra_ont:"));
        assert!(s.contains("@prefix dct:"));
        assert!(s.contains("@prefix rdfs:"));
        assert!(s.contains("@prefix xsd:"));
    }

    #[test]
    fn test_full_record_output() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&full_record());
        assert!(s.contains("insdc_sra:SRX000001"));
        assert!(s.contains("a dra_ont:Experiment"));
        assert!(
            s.contains("rdfs:label \"SRX000001: RNA-Seq of human brain tissue\""),
            "missing rdfs:label"
        );
        assert!(s.contains("dct:identifier \"SRX000001\""));
        assert!(
            s.contains("dra_ont:title \"RNA-Seq of human brain tissue\""),
            "missing title"
        );
        assert!(
            s.contains("dra_ont:designDescription \"Total RNA was extracted and sequenced.\""),
            "missing designDescription"
        );
        assert!(s.contains("a dra_ont:ILLUMINA"), "missing platform type");
        assert!(
            s.contains("dra_ont:instrumentModel dra_ont:Illumina_NovaSeq_6000"),
            "missing instrumentModel"
        );
        assert!(
            s.contains("a dra_ont:ExperimentDesign"),
            "missing ExperimentDesign type"
        );
        assert!(
            s.contains("dra_ont:libraryName \"Brain RNA lib1\""),
            "missing libraryName"
        );
        assert!(
            s.contains("dra_ont:libraryStrategy dra_ont:RNA-Seq"),
            "missing libraryStrategy"
        );
        assert!(
            s.contains("dra_ont:librarySource dra_ont:TRANSCRIPTOMIC"),
            "missing librarySource"
        );
        assert!(
            s.contains("dra_ont:librarySelection dra_ont:cDNA"),
            "missing librarySelection"
        );
        assert!(
            s.contains("dra_ont:libraryConstructionProtocol \"TruSeq RNA protocol\""),
            "missing protocol"
        );
        assert!(s.contains("a dra_ont:PAIRED"), "missing PAIRED layout type");
        assert!(
            s.contains("dra_ont:nominalLength \"300\"^^xsd:decimal"),
            "missing nominalLength"
        );
        assert!(
            s.contains("dra_ont:nominalSdev \"25.5\"^^xsd:decimal"),
            "missing nominalSdev"
        );
    }

    #[test]
    fn test_record_ends_with_dot() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&full_record());
        let trimmed = s.trim();
        assert!(trimmed.ends_with('.'));
    }

    #[test]
    fn test_minimal_record() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&minimal_record());
        assert!(s.contains("insdc_sra:SRX999999"));
        assert!(s.contains("a dra_ont:Experiment"));
        assert!(s.contains("dct:identifier \"SRX999999\""));
        // No platform or design blank nodes
        assert!(!s.contains("dra_ont:platform"), "platform should be absent");
        assert!(!s.contains("dra_ont:design"), "design should be absent");
        assert!(!s.contains("rdfs:label"), "label should be absent");
        assert!(s.trim().ends_with('.'));
    }

    #[test]
    fn test_single_layout() {
        let ser = TurtleSerializer::new();
        let rec = SraExperimentRecord {
            accession: "DRX000002".to_string(),
            title: None,
            design_description: None,
            library_name: None,
            library_strategy: Some("WGS".to_string()),
            library_source: Some("GENOMIC".to_string()),
            library_selection: Some("RANDOM".to_string()),
            library_layout: Some(LibraryLayout::Single),
            library_construction_protocol: None,
            platform: None,
            instrument_model: None,
        };
        let s = ser.record_to_string(&rec);
        assert!(s.contains("a dra_ont:SINGLE"), "missing SINGLE layout");
        assert!(
            !s.contains("dra_ont:nominalLength"),
            "SINGLE should not have nominalLength"
        );
    }

    fn record_with_terms(platform: &str, model: &str, term: &str) -> SraExperimentRecord {
        SraExperimentRecord {
            accession: "SRX1056924".to_string(),
            title: None,
            design_description: None,
            library_name: None,
            library_strategy: Some(term.to_string()),
            library_source: Some(term.to_string()),
            library_selection: Some(term.to_string()),
            library_layout: Some(LibraryLayout::Single),
            library_construction_protocol: None,
            platform: Some(platform.to_string()),
            instrument_model: Some(model.to_string()),
        }
    }

    #[test]
    fn test_is_safe_pn_local() {
        for ok in [
            "Illumina_NovaSeq_6000",
            "RNA-Seq",
            "454_GS_FLX",
            "_x",
            "0abc",
            "a.b",
            "a.b.c",
            "PAIRED",
        ] {
            assert!(is_safe_pn_local(ok), "{:?} should be safe", ok);
        }
        for bad in [
            "",
            "454_GS_FLX+",
            "DNBSEQ-T1+",
            "Hi-C/3C",
            "x(y)",
            "-abc",
            ".abc",
            "abc.",
            "a:b",
            "a%20b",
            "a#b",
            "caf\u{e9}",
            "a b",
        ] {
            assert!(!is_safe_pn_local(bad), "{:?} should not be safe", bad);
        }
    }

    #[test]
    fn test_unsafe_instrument_model_written_as_full_iri() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&record_with_terms("LS454", "454 GS FLX+", "WGS"));
        assert!(
            s.contains(
                "dra_ont:instrumentModel <http://ddbj.nig.ac.jp/ontologies/dra/454_GS_FLX+>"
            ),
            "instrument model with '+' must be a full IRI:\n{}",
            s
        );
        assert!(
            !s.contains("dra_ont:454_GS_FLX+"),
            "invalid prefixed name:\n{}",
            s
        );
        // Safe locals in the same record keep their prefixed names.
        assert!(s.contains("a dra_ont:LS454"), "{}", s);
        assert!(s.contains("dra_ont:libraryStrategy dra_ont:WGS"), "{}", s);
    }

    #[test]
    fn test_unsafe_locals_in_every_position_written_as_full_iris() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&record_with_terms("ION+TORRENT", "DNBSEQ-T1+", "Hi-C/3C"));
        let dra = "http://ddbj.nig.ac.jp/ontologies/dra/";
        assert!(s.contains(&format!("a <{}ION+TORRENT>", dra)), "{}", s);
        assert!(
            s.contains(&format!("dra_ont:instrumentModel <{}DNBSEQ-T1+>", dra)),
            "{}",
            s
        );
        for p in ["libraryStrategy", "librarySource", "librarySelection"] {
            assert!(
                s.contains(&format!("dra_ont:{} <{}Hi-C/3C>", p, dra)),
                "{} not a full IRI:\n{}",
                p,
                s
            );
        }
        assert!(!s.contains("dra_ont:ION+"), "{}", s);
        assert!(!s.contains("dra_ont:DNBSEQ-T1+"), "{}", s);
        assert!(!s.contains("dra_ont:Hi-C/3C"), "{}", s);
    }

    #[test]
    fn test_safe_instrument_model_keeps_prefixed_name() {
        let ser = TurtleSerializer::new();
        let s = ser.record_to_string(&record_with_terms(
            "ILLUMINA",
            "Illumina NovaSeq 6000",
            "RNA-Seq",
        ));
        assert!(
            s.contains("dra_ont:instrumentModel dra_ont:Illumina_NovaSeq_6000"),
            "{}",
            s
        );
        assert!(s.contains("a dra_ont:ILLUMINA"), "{}", s);
        assert!(s.contains("dra_ont:librarySource dra_ont:RNA-Seq"), "{}", s);
        assert!(
            !s.contains("<http://ddbj.nig.ac.jp/ontologies/dra/"),
            "{}",
            s
        );
    }

    #[test]
    fn test_ntriples_for_unsafe_local_unchanged() {
        use crate::serializer::ntriples::NTriplesSerializer;
        let nt = NTriplesSerializer::new().record_to_string(&record_with_terms(
            "LS454",
            "454 GS FLX+",
            "WGS",
        ));
        // Blank node labels come from a global counter: number them in order of appearance.
        let mut labels: Vec<String> = Vec::new();
        let normalised: Vec<String> = nt
            .lines()
            .map(|line| {
                line.split(' ')
                    .map(|w| {
                        if !w.starts_with("_:b") {
                            return w.to_string();
                        }
                        let i = labels.iter().position(|l| l == w).unwrap_or_else(|| {
                            labels.push(w.to_string());
                            labels.len() - 1
                        });
                        format!("_:n{}", i)
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let s = "<http://identifiers.org/insdc.sra/SRX1056924>";
        let d = "http://ddbj.nig.ac.jp/ontologies/dra/";
        let t = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
        let expected = vec![
            format!("{s} <{t}> <{d}Experiment> ."),
            format!("{s} <http://purl.org/dc/terms/identifier> \"SRX1056924\" ."),
            format!("{s} <{d}platform> _:n0 ."),
            format!("_:n0 <{t}> <{d}LS454> ."),
            format!("_:n0 <{d}instrumentModel> <{d}454_GS_FLX+> ."),
            format!("{s} <{d}design> _:n1 ."),
            format!("_:n1 <{t}> <{d}ExperimentDesign> ."),
            format!("_:n1 <{d}libraryStrategy> <{d}WGS> ."),
            format!("_:n1 <{d}librarySource> <{d}WGS> ."),
            format!("_:n1 <{d}librarySelection> <{d}WGS> ."),
            format!("_:n1 <{d}libraryLayout> _:n2 ."),
            format!("_:n2 <{t}> <{d}SINGLE> ."),
        ];
        assert_eq!(normalised, expected);
        assert!(nt.ends_with(" .\n"));
    }

    #[test]
    fn test_escaping() {
        let ser = TurtleSerializer::new();
        let rec = SraExperimentRecord {
            accession: "SRX000099".to_string(),
            title: Some("A \"quoted\" title\nwith newline".to_string()),
            design_description: None,
            library_name: None,
            library_strategy: None,
            library_source: None,
            library_selection: None,
            library_layout: None,
            library_construction_protocol: None,
            platform: None,
            instrument_model: None,
        };
        let s = ser.record_to_string(&rec);
        assert!(
            s.contains("A \\\"quoted\\\" title\\nwith newline"),
            "special characters should be escaped: {}",
            s
        );
    }
}
