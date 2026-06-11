use clap::{Parser, Subcommand};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf, process::Command};

#[derive(Parser)]
#[command(name = "thasia-dist")]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Subcommand)]
enum CommandKind {
    Bundle,
    CheckIcons,
    Manifest {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "")]
        notes: String,
        #[arg(long, default_value = "update-manifest.json")]
        output: PathBuf,
    },
}

#[derive(Serialize)]
struct Manifest {
    version: String,
    pub_date: String,
    notes: String,
    platforms: BTreeMap<String, PlatformAsset>,
}

#[derive(Serialize)]
struct PlatformAsset {
    url: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        CommandKind::Bundle => bundle()?,
        CommandKind::CheckIcons => check_icons()?,
        CommandKind::Manifest {
            version,
            notes,
            output,
        } => manifest(&version, &notes, &output)?,
    }
    Ok(())
}

fn bundle() -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("cargo")
        .args(["build", "-p", "thasia-ui", "--release"])
        .status()?;
    if !status.success() {
        return Err("release build failed".into());
    }
    println!("Release binary: target/release/thasia-gpui");
    println!("Use cargo-bundle with crates/thasia-ui package metadata for platform installers.");
    Ok(())
}

fn check_icons() -> Result<(), Box<dyn std::error::Error>> {
    for path in [
        "assets/icons/icon.icns",
        "assets/icons/icon.ico",
        "assets/icons/32x32.png",
        "assets/icons/128x128.png",
        "assets/icons/128x128@2x.png",
    ] {
        if !std::path::Path::new(path).is_file() {
            return Err(format!("missing icon: {path}").into());
        }
    }
    println!("Icon assets are present.");
    Ok(())
}

fn manifest(
    version: &str,
    notes: &str,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let version = version.trim_start_matches('v');
    let base = format!("https://github.com/LuMiSxh/Thasia/releases/download/v{version}");
    let targets = [
        ("darwin-aarch64", "aarch64-apple-darwin", "tar.gz"),
        ("darwin-x86_64", "x86_64-apple-darwin", "tar.gz"),
        ("windows-x86_64", "x86_64-pc-windows-msvc", "zip"),
        ("linux-x86_64", "x86_64-unknown-linux-gnu", "tar.gz"),
    ];
    let platforms = targets
        .into_iter()
        .map(|(platform, target, extension)| {
            (
                platform.to_string(),
                PlatformAsset {
                    url: format!("{base}/thasia-v{version}-{target}.{extension}"),
                },
            )
        })
        .collect();
    let manifest = Manifest {
        version: version.to_string(),
        pub_date: chrono::Utc::now().to_rfc3339(),
        notes: notes.to_string(),
        platforms,
    };
    std::fs::write(output, serde_json::to_vec_pretty(&manifest)?)?;
    println!("Wrote {}", output.display());
    Ok(())
}
