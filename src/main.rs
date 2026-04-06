use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "insdc-rdf")]
#[command(about = "Convert INSDC sequence archive metadata to RDF")]
#[command(version = env!("BUILD_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert source data to RDF (TTL, JSON-LD, N-Triples)
    Convert {
        /// Data source type
        #[arg(short = 's', long, default_value = "biosample")]
        source: SourceType,

        /// Path to input file
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for chunked RDF files
        #[arg(short, long, default_value = "./output")]
        output_dir: PathBuf,

        /// Number of records per output chunk
        #[arg(short, long, default_value_t = 100_000)]
        chunk_size: usize,
    },

    /// Validate output RDF files
    Validate {
        /// Path to file or directory to validate
        path: PathBuf,
    },
}

#[derive(Clone, ValueEnum)]
enum SourceType {
    Biosample,
    Sra,
    Bioproject,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            source,
            input,
            output_dir,
            chunk_size,
        } => match source {
            SourceType::Biosample => {
                insdc_rdf_biosample::run_convert(&input, &output_dir, chunk_size)
            }
            SourceType::Sra => insdc_rdf_sra::run_convert(&input, &output_dir, chunk_size),
            SourceType::Bioproject => {
                insdc_rdf_bioproject::run_convert(&input, &output_dir, chunk_size)
            }
        },
        Commands::Validate { path } => {
            use insdc_rdf_biosample::validate;
            let results = if path.is_dir() {
                validate::validate_directory(&path)
            } else if path.extension().map(|e| e == "ttl").unwrap_or(false) {
                vec![validate::validate_turtle(&path)]
            } else if path.extension().map(|e| e == "nt").unwrap_or(false) {
                vec![validate::validate_ntriples(&path)]
            } else {
                eprintln!("Unsupported file type: {:?}", path);
                std::process::exit(1);
            };
            let mut total_errors = 0;
            let mut total_records = 0;
            for result in &results {
                total_records += result.record_count;
                if result.errors.is_empty() {
                    eprintln!("  OK: {} ({} records)", result.file, result.record_count);
                } else {
                    for err in &result.errors {
                        eprintln!("  ERROR: {} - {}", result.file, err);
                    }
                    total_errors += result.errors.len();
                }
            }
            eprintln!(
                "Validation complete: {} files, {} records, {} errors",
                results.len(),
                total_records,
                total_errors
            );
            if total_errors > 0 {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}
