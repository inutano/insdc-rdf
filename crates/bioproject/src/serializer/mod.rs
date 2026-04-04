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
