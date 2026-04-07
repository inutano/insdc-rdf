pub mod chunk;
pub mod model;
pub mod parser;
pub mod serializer;

use std::path::Path;

pub fn run_convert(_input: &Path, _output_dir: &Path, _chunk_size: usize) -> anyhow::Result<()> {
    todo!("SRA experiment conversion not yet implemented")
}
