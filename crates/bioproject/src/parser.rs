use std::io::BufRead;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::model::BioProjectRecord;
use insdc_rdf_core::error::ConvertError;

/// A streaming parser for NCBI bioproject.xml.
///
/// The XML structure is:
/// ```xml
/// <PackageSet>
///   <Package>
///     <Project>
///       <Project>
///         <ProjectID><ArchiveID accession="PRJNA3" .../></ProjectID>
///         <ProjectDescr>
///           <Name>...</Name>
///           <Title>...</Title>
///           <Description>...</Description>
///           <ProjectReleaseDate>...</ProjectReleaseDate>
///         </ProjectDescr>
///         <ProjectType>
///           <ProjectTypeSubmission>
///             <Target><Organism taxID="..." ><OrganismName>...</OrganismName></Organism></Target>
///           </ProjectTypeSubmission>
///         </ProjectType>
///       </Project>
///       <Submission submitted="..."/>
///     </Project>
///   </Package>
/// </PackageSet>
/// ```
pub struct BioProjectParser<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
}

impl<R: BufRead> BioProjectParser<R> {
    pub fn new(reader: R) -> Self {
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);
        BioProjectParser {
            reader: xml_reader,
            buf: Vec::new(),
        }
    }

    pub fn next_record(&mut self) -> Result<Option<BioProjectRecord>, ConvertError> {
        // Scan for <Package> start tag
        loop {
            self.buf.clear();
            let event =
                self.reader
                    .read_event_into(&mut self.buf)
                    .map_err(|e| ConvertError::XmlParse {
                        offset: self.reader.error_position(),
                        message: e.to_string(),
                    })?;

            match event {
                Event::Start(ref e) if e.name().as_ref() == b"Package" => {
                    return self.parse_package();
                }
                Event::Eof => return Ok(None),
                _ => {}
            }
        }
    }

    /// Parse a <Package> element and extract the BioProject record within.
    fn parse_package(&mut self) -> Result<Option<BioProjectRecord>, ConvertError> {
        let mut accession: Option<String> = None;
        let mut name: Option<String> = None;
        let mut title: Option<String> = None;
        let mut description: Option<String> = None;
        let mut organism_name: Option<String> = None;
        let mut taxonomy_id: Option<String> = None;
        let mut release_date: Option<String> = None;
        let mut submission_date: Option<String> = None;

        let mut depth: u32 = 1; // inside <Package>
        let mut text_target: TextTarget = TextTarget::None;
        let mut text_buf = String::new();

        loop {
            self.buf.clear();
            let event =
                self.reader
                    .read_event_into(&mut self.buf)
                    .map_err(|e| ConvertError::XmlParse {
                        offset: self.reader.error_position(),
                        message: e.to_string(),
                    })?;

            match event {
                Event::Start(ref e) => {
                    let tag = e.name();
                    let tag_bytes = tag.as_ref();

                    match tag_bytes {
                        b"ArchiveID" => {
                            let offset = self.reader.buffer_position();
                            for attr_result in e.attributes() {
                                let attr = attr_result.map_err(|err| ConvertError::XmlParse {
                                    offset,
                                    message: format!("attribute parse error: {}", err),
                                })?;
                                if attr.key.as_ref() == b"accession" {
                                    accession = Some(
                                        attr.unescape_value()
                                            .map_err(|err| ConvertError::XmlParse {
                                                offset,
                                                message: err.to_string(),
                                            })?
                                            .into_owned(),
                                    );
                                }
                            }
                        }
                        b"Name" if depth <= 5 && name.is_none() && accession.is_some() => {
                            // Only capture ProjectDescr/Name, not author names
                            text_target = TextTarget::Name;
                            text_buf.clear();
                        }
                        b"Title" if depth <= 5 && title.is_none() => {
                            text_target = TextTarget::Title;
                            text_buf.clear();
                        }
                        b"Description" if depth <= 5 && description.is_none() => {
                            text_target = TextTarget::Description;
                            text_buf.clear();
                        }
                        b"ProjectReleaseDate" => {
                            text_target = TextTarget::ReleaseDate;
                            text_buf.clear();
                        }
                        b"OrganismName" => {
                            text_target = TextTarget::OrganismName;
                            text_buf.clear();
                        }
                        b"Organism" => {
                            let offset = self.reader.buffer_position();
                            for attr_result in e.attributes() {
                                let attr = attr_result.map_err(|err| ConvertError::XmlParse {
                                    offset,
                                    message: format!("attribute parse error: {}", err),
                                })?;
                                if attr.key.as_ref() == b"taxID" {
                                    taxonomy_id = Some(
                                        attr.unescape_value()
                                            .map_err(|err| ConvertError::XmlParse {
                                                offset,
                                                message: err.to_string(),
                                            })?
                                            .into_owned(),
                                    );
                                }
                            }
                        }
                        b"Submission" => {
                            let offset = self.reader.buffer_position();
                            for attr_result in e.attributes() {
                                let attr = attr_result.map_err(|err| ConvertError::XmlParse {
                                    offset,
                                    message: format!("attribute parse error: {}", err),
                                })?;
                                if attr.key.as_ref() == b"submitted" {
                                    submission_date = Some(
                                        attr.unescape_value()
                                            .map_err(|err| ConvertError::XmlParse {
                                                offset,
                                                message: err.to_string(),
                                            })?
                                            .into_owned(),
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                    depth += 1;
                }
                Event::Empty(ref e) => {
                    let tag = e.name();
                    let tag_bytes = tag.as_ref();

                    if tag_bytes == b"ArchiveID" {
                        let offset = self.reader.buffer_position();
                        for attr_result in e.attributes() {
                            let attr = attr_result.map_err(|err| ConvertError::XmlParse {
                                offset,
                                message: format!("attribute parse error: {}", err),
                            })?;
                            if attr.key.as_ref() == b"accession" {
                                accession = Some(
                                    attr.unescape_value()
                                        .map_err(|err| ConvertError::XmlParse {
                                            offset,
                                            message: err.to_string(),
                                        })?
                                        .into_owned(),
                                );
                            }
                        }
                    } else if tag_bytes == b"Submission" {
                        let offset = self.reader.buffer_position();
                        for attr_result in e.attributes() {
                            let attr = attr_result.map_err(|err| ConvertError::XmlParse {
                                offset,
                                message: format!("attribute parse error: {}", err),
                            })?;
                            if attr.key.as_ref() == b"submitted" {
                                submission_date = Some(
                                    attr.unescape_value()
                                        .map_err(|err| ConvertError::XmlParse {
                                            offset,
                                            message: err.to_string(),
                                        })?
                                        .into_owned(),
                                );
                            }
                        }
                    } else if tag_bytes == b"Organism" {
                        let offset = self.reader.buffer_position();
                        for attr_result in e.attributes() {
                            let attr = attr_result.map_err(|err| ConvertError::XmlParse {
                                offset,
                                message: format!("attribute parse error: {}", err),
                            })?;
                            if attr.key.as_ref() == b"taxID" {
                                taxonomy_id = Some(
                                    attr.unescape_value()
                                        .map_err(|err| ConvertError::XmlParse {
                                            offset,
                                            message: err.to_string(),
                                        })?
                                        .into_owned(),
                                );
                            }
                        }
                    }
                }
                Event::End(ref e) => {
                    let tag = e.name();
                    let tag_bytes = tag.as_ref();

                    match tag_bytes {
                        b"Name" if text_target == TextTarget::Name => {
                            if !text_buf.is_empty() {
                                name = Some(text_buf.clone());
                            }
                            text_target = TextTarget::None;
                        }
                        b"Title" if text_target == TextTarget::Title => {
                            if !text_buf.is_empty() {
                                title = Some(text_buf.clone());
                            }
                            text_target = TextTarget::None;
                        }
                        b"Description" if text_target == TextTarget::Description => {
                            if !text_buf.is_empty() {
                                description = Some(text_buf.clone());
                            }
                            text_target = TextTarget::None;
                        }
                        b"ProjectReleaseDate" if text_target == TextTarget::ReleaseDate => {
                            if !text_buf.is_empty() {
                                release_date = Some(text_buf.clone());
                            }
                            text_target = TextTarget::None;
                        }
                        b"OrganismName" if text_target == TextTarget::OrganismName => {
                            if !text_buf.is_empty() {
                                organism_name = Some(text_buf.clone());
                            }
                            text_target = TextTarget::None;
                        }
                        b"Package" => {
                            // End of package
                            break;
                        }
                        _ => {}
                    }
                    depth -= 1;
                }
                Event::Text(ref e) => {
                    if text_target != TextTarget::None {
                        let text = e.unescape().map_err(|err| ConvertError::XmlParse {
                            offset: self.reader.buffer_position(),
                            message: format!("text unescape error: {}", err),
                        })?;
                        text_buf.push_str(&text);
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        match accession {
            Some(acc) => Ok(Some(BioProjectRecord {
                accession: acc,
                name,
                title,
                description,
                organism_name,
                taxonomy_id,
                release_date,
                submission_date,
            })),
            None => {
                // Package without an accession — skip
                Ok(None)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TextTarget {
    None,
    Name,
    Title,
    Description,
    ReleaseDate,
    OrganismName,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse_all(xml: &str) -> (Vec<BioProjectRecord>, Vec<ConvertError>) {
        let reader = std::io::BufReader::new(Cursor::new(xml.as_bytes().to_vec()));
        let mut parser = BioProjectParser::new(reader);
        let mut records = Vec::new();
        let mut errors = Vec::new();
        loop {
            match parser.next_record() {
                Ok(Some(rec)) => records.push(rec),
                Ok(None) => break,
                Err(e) => errors.push(e),
            }
        }
        (records, errors)
    }

    fn fixture_xml() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/bioproject_sample.xml"
        ))
        .unwrap()
    }

    #[test]
    fn test_parse_fixture_count() {
        let (records, errors) = parse_all(&fixture_xml());
        assert_eq!(errors.len(), 0);
        assert_eq!(records.len(), 3);
    }

    #[test]
    fn test_first_record_fields() {
        let (records, _) = parse_all(&fixture_xml());
        let rec = &records[0];
        assert_eq!(rec.accession, "PRJNA3");
        assert_eq!(rec.name.as_deref(), Some("Borreliella burgdorferi B31"));
        assert_eq!(rec.title.as_deref(), Some("Causes Lyme disease"));
        assert!(rec.description.as_deref().unwrap().contains("type strain"));
        assert_eq!(
            rec.organism_name.as_deref(),
            Some("Borreliella burgdorferi B31")
        );
        assert_eq!(rec.taxonomy_id.as_deref(), Some("224326"));
        assert_eq!(rec.release_date.as_deref(), Some("2001-01-09T00:00:00Z"));
        assert_eq!(rec.submission_date.as_deref(), Some("2003-02-23"));
    }

    #[test]
    fn test_record_without_name() {
        let (records, _) = parse_all(&fixture_xml());
        let rec = &records[1]; // PRJNA5 has no Name, only Title
        assert_eq!(rec.accession, "PRJNA5");
        assert_eq!(rec.name, None);
        assert!(rec.title.as_deref().unwrap().contains("Treponema"));
    }

    #[test]
    fn test_minimal_record() {
        let (records, _) = parse_all(&fixture_xml());
        let rec = &records[2]; // PRJNA7 — minimal
        assert_eq!(rec.accession, "PRJNA7");
        assert_eq!(rec.name.as_deref(), Some("Minimal project"));
        assert_eq!(rec.title, None);
        assert_eq!(rec.description, None);
        assert_eq!(rec.organism_name, None);
    }
}
