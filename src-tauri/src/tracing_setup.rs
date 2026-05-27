//! Tracing initialization.
//!
//! Dev builds log to stderr with ANSI colours.
//! Release builds log to `<app_data>/com.thasia/logs/thasia.log` using a
//! non-blocking writer; the returned guard must be kept alive for the
//! lifetime of the program so the worker thread can flush.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

const APP_IDENTIFIER: &str = "com.thasia";
const LOG_FILE_PREFIX: &str = "thasia";
const LOG_FILE_SUFFIX: &str = "log";
const LOG_MAX_FILES: usize = 7;
const DEFAULT_FILTER_DEV: &str =
    "info,thasia_tauri_lib=debug,thasia_source=debug,thasia_processor=debug,thasia_parser=debug";
const DEFAULT_FILTER_RELEASE: &str = "info";

pub fn init() -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_filter()));

    if cfg!(debug_assertions) {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(std::io::stderr).with_ansi(true))
            .init();
        return None;
    }

    let Some(log_dir) = log_dir() else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(std::io::stderr).with_ansi(false))
            .init();
        return None;
    };

    if std::fs::create_dir_all(&log_dir).is_err() {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(std::io::stderr).with_ansi(false))
            .init();
        return None;
    }

    let appender = match RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix(LOG_FILE_SUFFIX)
        .max_log_files(LOG_MAX_FILES)
        .build(&log_dir)
    {
        Ok(appender) => appender,
        Err(_) => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().with_writer(std::io::stderr).with_ansi(false))
                .init();
            return None;
        }
    };
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(writer).with_ansi(false))
        .init();
    Some(guard)
}

/// Returns the directory used by `init()` for rolling log files. Exposed so
/// the Suwayomi subprocess can put its captured stdout/stderr in the same
/// place (rather than burying them under `suwayomi-data/`).
pub fn log_dir() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join(APP_IDENTIFIER).join("logs"))
}

fn default_filter() -> &'static str {
    if cfg!(debug_assertions) {
        DEFAULT_FILTER_DEV
    } else {
        DEFAULT_FILTER_RELEASE
    }
}
