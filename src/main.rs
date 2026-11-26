//! CLI for sigmaos-betteroffice.

use clap::{Parser, Subcommand};
use sigmaos_betteroffice::{default_docs_dir, DocKind, Document};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sigmaos-betteroffice", about = "SigmaOS office document tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new document.
    New {
        name: String,
        /// text|sheet|slides
        #[arg(long, default_value = "text")]
        kind: String,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Show document info / preview.
    Show { file: PathBuf },
    /// Validate a document file.
    Validate { file: PathBuf },
    /// List documents in a directory.
    List {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

fn parse_kind(s: &str) -> Result<DocKind, Box<dyn std::error::Error>> {
    Ok(match s {
        "text" => DocKind::Text,
        "sheet" => DocKind::Sheet,
        "slides" => DocKind::Slides,
        other => return Err(format!("unknown kind: {other}").into()),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    match Cli::parse().command {
        Commands::New { name, kind, dir } => {
            let dir = dir
                .or_else(default_docs_dir)
                .unwrap_or_else(|| PathBuf::from("."));
            std::fs::create_dir_all(&dir)?;
            let doc = Document::new(&name, parse_kind(&kind)?);
            let path = dir.join(format!("{}.sdoc", name.replace(' ', "-")));
            doc.save(&path)?;
            println!("created: {}", path.display());
        }
        Commands::Show { file } => {
            let doc = Document::load(&file)?;
            println!("# {} ({:?})", doc.name, doc.kind);
            println!("{}", doc.render_text());
        }
        Commands::Validate { file } => {
            Document::load(&file)?;
            println!("valid: {}", file.display());
        }
        Commands::List { dir } => {
            let dir = dir
                .or_else(default_docs_dir)
                .unwrap_or_else(|| PathBuf::from("."));
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|x| x.to_str()) == Some("sdoc") {
                        println!("{}", p.display());
                    }
                }
            }
        }
    }
    Ok(())
}
