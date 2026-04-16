use clap::Parser;
use miette::{IntoDiagnostic, Result};
use std::{path::PathBuf, sync::Arc};
use thasia_packager::{CbzGenerator, Generator};
use thasia_parser::{Resolver, RuleConfig};
use thasia_processor::EncodeOptions;
use thasia_source::{LocalSource, Source};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::prelude::*;

#[derive(Parser)]
#[command(name = "thasia", about = "Headless Manga Processing Engine", version)]
struct Cli {
    /// Source directory or CBZ/ZIP file
    #[arg(short, long)]
    source: PathBuf,

    /// Output directory
    #[arg(short, long)]
    out: PathBuf,

    /// Image format: avif, webp, or original
    #[arg(short, long, default_value = "avif")]
    format: String,

    /// Max image width (downscales wider images preserving aspect ratio)
    #[arg(long)]
    max_width: Option<u32>,

    /// Output volume name (used as filename for CBZ)
    #[arg(short, long, default_value = "output")]
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let indicatif = IndicatifLayer::new();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(indicatif.get_stderr_writer()),
        )
        .with(indicatif)
        .init();

    let args = Cli::parse();

    // 1. Source
    let source = Arc::new(LocalSource::new(args.source));

    // 2. Parser
    let resolver = Resolver::new(RuleConfig::default());

    // 3. Discover + parse
    let mut rx_discover = source.discover().await.into_diagnostic()?;
    let (tx_parsed, rx_parsed) = tokio::sync::mpsc::channel(256);

    tokio::spawn(async move {
        while let Some(img) = rx_discover.recv().await {
            match resolver.resolve(img) {
                Ok(parsed) => {
                    let _ = tx_parsed.send(parsed).await;
                }
                Err(e) => tracing::warn!("Parse failed: {}", e),
            }
        }
    });

    // 4. Process
    let options = Arc::new(EncodeOptions {
        format: args.format,
        max_width: args.max_width,
    });
    let mut rx_processed =
        thasia_processor::start_pipeline(source.clone(), rx_parsed, options).await;

    // 5. Package
    let mut cbz = CbzGenerator::new();
    cbz.init(&args.out, &args.name).await.into_diagnostic()?;

    let mut pages_written = 0u32;
    while let Some(res) = rx_processed.recv().await {
        let processed = res.into_diagnostic()?;
        cbz.add_page(processed).await.into_diagnostic()?;
        pages_written += 1;
        tracing::info!("Pages written: {}", pages_written);
    }

    Box::new(cbz).finalize().await.into_diagnostic()?;
    tracing::info!("Done! {} pages packaged into {}.cbz", pages_written, args.name);
    Ok(())
}
