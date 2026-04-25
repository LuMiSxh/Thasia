use clap::Parser;
use miette::{IntoDiagnostic, Result};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use thasia_core::models::{Direction, ImageFormat, OutputFormat};
use thasia_packager::{CbzGenerator, EpubGenerator, Generator, RawGenerator};
use thasia_parser::{Resolver, RuleConfig};
use thasia_processor::EncodeOptions;
use thasia_source::{LocalSource, Source};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::prelude::*;

/// How chapters are grouped into volumes.
#[derive(Debug, Clone, PartialEq, Default, clap::ValueEnum)]
enum BundleStrategy {
    /// Group by volume number extracted from path (default).
    #[default]
    Auto,
    /// Merge all chapters into a single output file.
    Flatten,
}

#[derive(Parser)]
#[command(name = "thasia", about = "Headless Manga Processing Engine", version)]
struct Cli {
    /// Source directory, CBZ, or ZIP file.
    #[arg(short, long)]
    source: PathBuf,

    /// Output directory.
    #[arg(short, long)]
    out: PathBuf,

    /// Image encoding format.
    #[arg(short, long, default_value = "avif")]
    format: ImageFormat,

    /// Output container format.
    #[arg(long, default_value = "cbz")]
    output: OutputFormat,

    /// EPUB reading direction (only relevant when --output epub).
    #[arg(long, default_value = "ltr")]
    direction: Direction,

    /// Max image width in pixels — wider images are downscaled proportionally.
    #[arg(long)]
    max_width: Option<u32>,

    /// Output volume name (used as the base filename).
    #[arg(short, long, default_value = "output")]
    name: String,

    /// Volume grouping strategy.
    #[arg(long, default_value = "auto")]
    bundle: BundleStrategy,

    /// Separator placed between the name and volume number.
    #[arg(long, default_value = " - ")]
    volume_separator: String,

    /// Omit the volume number suffix when only one volume is produced.
    #[arg(long)]
    hide_single_volume: bool,

    /// Create a named subdirectory inside the output directory.
    #[arg(long)]
    create_directory: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let indicatif = IndicatifLayer::new();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer().with_writer(indicatif.get_stderr_writer()))
        .with(indicatif)
        .init();

    let args = Cli::parse();

    // 1. Source — detect ZIP/CBZ and extract automatically
    let source = Arc::new(if LocalSource::is_archive(&args.source) {
        LocalSource::from_archive(args.source.clone())
            .await
            .into_diagnostic()?
    } else {
        LocalSource::new(args.source.clone())
    });

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

    // 5. Collect all processed images
    let mut volume_map: BTreeMap<u32, Vec<_>> = BTreeMap::new();
    while let Some(result) = rx_processed.recv().await {
        let img = result.into_diagnostic()?;
        let vol = if args.bundle == BundleStrategy::Flatten {
            1
        } else {
            img.parsed_data.identifier.volume.unwrap_or(1)
        };
        volume_map.entry(vol).or_default().push(img);
    }

    if volume_map.is_empty() {
        tracing::warn!("No images were processed.");
        return Ok(());
    }

    // 6. Determine output root
    let out_root = if args.create_directory {
        args.out.join(&args.name)
    } else {
        args.out.clone()
    };
    tokio::fs::create_dir_all(&out_root)
        .await
        .into_diagnostic()?;

    // 7. Package each volume
    let total_volumes = volume_map.len();
    let mut total_pages = 0u32;

    for (vol_num, mut pages) in volume_map {
        pages.sort_by_key(|p| p.parsed_data.page_number);

        let vol_name = if total_volumes == 1 && args.hide_single_volume {
            args.name.clone()
        } else {
            format!("{}{}{}", args.name, args.volume_separator, vol_num)
        };

        let mut pkg: Box<dyn Generator> = match args.output {
            OutputFormat::Cbz => Box::new(CbzGenerator::new()),
            OutputFormat::Epub => {
                Box::new(EpubGenerator::new().with_direction(args.direction.clone()))
            }
            OutputFormat::Raw => Box::new(RawGenerator::new()),
        };

        pkg.init(&out_root, &vol_name).await.into_diagnostic()?;
        for page in pages {
            total_pages += 1;
            pkg.add_page(page).await.into_diagnostic()?;
        }
        pkg.finalize().await.into_diagnostic()?;
        tracing::info!("Volume '{}' complete", vol_name);
    }

    tracing::info!(
        "Done — {} page(s) across {} volume(s)",
        total_pages,
        total_volumes
    );
    Ok(())
}
