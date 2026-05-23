use clap::Parser;
use miette::{IntoDiagnostic, Result};
use std::{path::PathBuf, sync::Arc};
use thasia_core::{
    BundleMode, Direction, ImageFormat, OutputFormat, VolumeNaming, VolumePlan, apply_naming,
    auto_group,
};
use thasia_packager::{CbzGenerator, EpubGenerator, Generator, RawGenerator};
use thasia_parser::{Resolver, RuleConfig};
use thasia_processor::EncodeOptions;
use thasia_source::{LocalSource, Source};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::prelude::*;

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
    bundle: BundleMode,

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

    // 4. Build the conversion plan using the shared helpers.
    let naming = VolumeNaming {
        name: &args.name,
        separator: &args.volume_separator,
        hide_single_volume: args.hide_single_volume,
    };
    let groups = auto_group(parsed_images, args.bundle);
    let plans = apply_naming(groups, &naming);
    let total_volumes = plans.len();

    // 5. Output root.
    let out_root = if args.create_directory {
        args.out.join(&args.name)
    } else {
        args.out.clone()
    };
    tokio::fs::create_dir_all(&out_root)
        .await
        .into_diagnostic()?;

    let options = EncodeOptions {
        format: args.format,
        max_width: args.max_width,
    };

    // 6. Process each volume sequentially: one packager open at a time.
    let mut total_pages = 0u32;
    for plan in plans {
        let pages_this_vol = process_volume(plan, &out_root, &args, options, source.clone()).await?;
        total_pages += pages_this_vol;
    }

    tracing::info!(
        "Done — {} page(s) across {} volume(s)",
        total_pages,
        total_volumes
    );
    Ok(())
}

async fn process_volume(
    plan: VolumePlan,
    out_root: &std::path::Path,
    args: &Cli,
    encode_opts: EncodeOptions,
    source: Arc<LocalSource>,
) -> Result<u32> {
    let display_name = plan.display_name.clone();
    let mut pkg: Box<dyn Generator> = match args.output {
        OutputFormat::Cbz => Box::new(CbzGenerator::new()),
        OutputFormat::Epub => Box::new(EpubGenerator::new().with_direction(args.direction)),
        OutputFormat::Raw => Box::new(RawGenerator::new()),
    };
    pkg.init(out_root, &display_name).await.into_diagnostic()?;

    // Feed this volume's pages into the pipeline.
    let (tx_parsed, rx_parsed) = tokio::sync::mpsc::channel(256);
    let pages = plan.pages;
    tokio::spawn(async move {
        for img in pages {
            if tx_parsed.send(img).await.is_err() {
                break;
            }
        }
    });

    let mut rx_processed = thasia_processor::start_pipeline(source, rx_parsed, encode_opts).await;

    // Collect, sort by page_number, then write — matches the Tauri convert flow.
    let mut all = Vec::new();
    while let Some(result) = rx_processed.recv().await {
        all.push(result.into_diagnostic()?);
    }
    let count = all.len() as u32;
    all.sort_by_key(|img| img.parsed_data.page_number);
    for img in all {
        pkg.add_page(img).await.into_diagnostic()?;
    }
    pkg.finalize().await.into_diagnostic()?;

    tracing::info!("Volume '{}' complete", display_name);
    Ok(count)
}
