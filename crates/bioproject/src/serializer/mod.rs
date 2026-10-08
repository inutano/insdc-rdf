pub mod jsonld;
pub mod ntriples;
pub mod turtle;

use crate::model::BioProjectRecord;
use std::io::Write;

pub trait Serializer {
    fn write_header<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &BioProjectRecord,
    ) -> std::io::Result<()>;
    fn write_footer<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
}

/// The XSD datatype local name (`date` or `dateTime`) for a date value.
///
/// A calendar date with no time (`YYYY-MM-DD`) is an `xsd:date`; anything
/// else keeps the `xsd:dateTime` the converter has always written.
pub(crate) fn xsd_date_type(value: &str) -> &'static str {
    let b = value.as_bytes();
    let date_only = b.len() == 10
        && b.iter().enumerate().all(|(i, &c)| match i {
            4 | 7 => c == b'-',
            _ => c.is_ascii_digit(),
        });
    if date_only {
        "date"
    } else {
        "dateTime"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xsd_date_type() {
        assert_eq!(xsd_date_type("2003-02-23"), "date");
        assert_eq!(xsd_date_type("2026-07-02"), "date");
        for v in [
            "2001-01-09T00:00:00Z",
            "2003-02-23T10:20:30",
            "2003-02-23Z",
            "2003-2-23",
            "20030223",
            "2003-02-233",
            " 2003-02-23",
            "2003-02-23 ",
            "2003/02/23",
            "\u{0662}003-02-23",
            "",
        ] {
            assert_eq!(xsd_date_type(v), "dateTime", "{:?}", v);
        }
    }
}
