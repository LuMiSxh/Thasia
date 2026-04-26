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

    // 3. Discover + parse — collect metadata only (tiny structs, ~300 bytes each).
    // This lets us count volumes and name packagers before the encode pipeline starts,
    // without holding any pixel or encoded data in memory.
    let mut rx_discover = source.discover().await.into_diagnostic()?;
    let mut parsed_images = Vec::new();
    while let Some(img) = rx_discover.recv().await {
        match resolver.resolve(img) {
            Ok(parsed) => parsed_images.push(parsed),
            Err(e) => tracing::warn!("Parse failed: {}", e),
        }
    }

    if parsed_images.is_empty() {
        tracing::warn!("No images were processed.");
        return Ok(());
    }

    // 4. Count distinct volumes so packagers can be named correctly up front.
    let volume_set: std::collections::BTreeSet<u32> = if args.bundle == BundleStrategy::Flatten {
        std::iter::once(1u32).collect()
    } else {
        parsed_images
            .iter()
            .map(|p| p.identifier.volume.unwrap_or(1))
            .collect()
    };
    let total_volumes = volume_set.len();

    // 5. Determine output root and create one packager per volume.
    let out_root = if args.create_directory {
        args.out.join(&args.name)
    } else {
        args.out.clone()
    };
    tokio::fs::create_dir_all(&out_root)
        .await
        .into_diagnostic()?;

    let mut packagers: BTreeMap<u32, Box<dyn Generator>> = BTreeMap::new();
    for &vol_num in &volume_set {
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
        packagers.insert(vol_num, pkg);
    }

    // 6. Process — feed parsed metadata into pipeline and stream results directly
    // to packagers. CBZ/Raw write each page to disk immediately in add_page, so
    // encoded image data is never accumulated in memory across all pages at once.
    let options = Arc::new(EncodeOptions {
        format: args.format,
        max_width: args.max_width,
    });
    let (tx_parsed, rx_parsed) = tokio::sync::mpsc::channel(256);
    tokio::spawn(async move {
        for img in parsed_images {
            if tx_parsed.send(img).await.is_err() {
                break;
            }
        }
    });

    let mut rx_processed =
        thasia_processor::start_pipeline(source.clone(), rx_parsed, options).await;
    let mut total_pages = 0u32;

    while let Some(result) = rx_processed.recv().await {
        let img = result.into_diagnostic()?;
        let vol = if args.bundle == BundleStrategy::Flatten {
            1
        } else {
            img.parsed_data.identifier.volume.unwrap_or(1)
        };
        total_pages += 1;
        packagers
            .get_mut(&vol)
            .unwrap()
            .add_page(img)
            .await
            .into_diagnostic()?;
    }

    // 7. Finalize — CBZ flushes the ZIP central directory, EPUB generates the file.
    for (vol_num, pkg) in packagers {
        let vol_name = if total_volumes == 1 && args.hide_single_volume {
            args.name.clone()
        } else {
            format!("{}{}{}", args.name, args.volume_separator, vol_num)
        };
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
