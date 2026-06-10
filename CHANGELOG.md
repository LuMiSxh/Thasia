# Changelog

All notable changes to this project will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Conversion observability** — conversion progress now carries live elapsed time, throughput, estimated remaining time, byte counters, passthrough/encoded page counts, and aggregate fetch/decode/transform/encode timings for the final result view.
- **Pipeline review contract** — the backend now exposes an ordered pipeline plan with active/skipped stages, transform metadata, page counts, and volume counts so the review page can render the conversion pipeline without guessing backend behavior.
- **Optional image enhancement transforms** — conversion can now apply deterministic color enhancement for washed-out color scans and mild sharpening for soft pages. Both are off by default, disable passthrough when enabled, and are exposed in wizard/settings defaults.
- **Memory-mapped local image reads** — large local source images can now be decoded from read-only memory maps during re-encode jobs, reducing compressed-input copies before the decode stage.
- **Conversion result outputs** — completed conversions now report the generated volume paths in the final result view and completion event.

### Changed

- Optional pixel transforms now run after max-width downscaling so expensive cleanup, enhancement, and sharpening work on fewer pixels when resizing is enabled.
- Internal conversion code now uses concrete pipeline/result types, shared wizard payload builders, focused pipeline-plan modules, and typed `thiserror` conversion errors instead of passing opaque tuples and string errors through the backend.
- Platform-specific local file-read hints are grouped behind a source-local platform module, making future OS-specific read/mmap tuning easier to isolate.

### Removed

- Removed unused no-cancel pipeline entry points, an unused CBZ extraction helper, and an unused Suwayomi manager accessor.

## [0.3.0] - 2026-05-30

### Added

- **Forced re-encode path** — matching AVIF/WebP inputs pass through by default, but the app can now force a full decode, transform, and encode pass when smaller normalized output is preferred.
- **Mandatory native AVIF decode** — AVIF input decoding now uses `avif-decode`, allowing forced AVIF re-encode instead of instantly failing on AVIF sources the `image` crate cannot decode.
- **Optional clean scan tones transform** — grayscale and line-art pages can normalize near-white paper to pure white and near-black ink to pure black without cropping or changing page geometry.
- **Modular transform pipeline** — decoding, default transforms, optional transforms, resizing, and encoding are now explicit stages, making future filters and metadata cleanup easier to add.
- **Page editor hover preview** — hovering a page tile for a short dwell opens a larger inspect preview with page metadata, making tall manhua pages and mid-volume ads easier to identify without changing the grid workflow.
- **Release changelog automation** — the publish workflow validates the requested tag against the workspace version and uses the matching `CHANGELOG.md` section as release notes.

### Changed

- **Conversion pipeline throughput** — packaging now receives processed pages in deterministic order while conversion stays parallel, with bounded in-flight image work to avoid runaway memory use on large jobs.
- **Archive and output safety** — archive extraction and generated filenames are sanitized more aggressively, including CBZ ComicInfo XML escaping and safer custom image paths.
- **CBZ packaging** — already-compressed image formats are stored directly instead of being ZIP-deflated again, saving CPU without increasing output size.
- **Large-volume page editor performance** — the page review grid now virtualizes visible tiles, uses async image decoding, shows loading placeholders, and suppresses hover previews while scrolling, avoiding thousands of mounted thumbnails on 2,000+ page inputs.
- **Build requirements** — CMake is now required for development and release builds because AVIF decoding is bundled at compile time.
- **Release builds** — CI installs CMake and platform link tooling needed by the Rust/Tauri build.
- **macOS release builds** — v0.3.0 now publishes Apple Silicon macOS artifacts only; Intel macOS is skipped because the bundled AVIF decoder's `libaom` CMake build currently fails against NASM 3 during x86 builds.
- **Codec acceleration** — image processing now enables the `image` crate's NASM acceleration hook for AVIF assembly paths while keeping release artifacts CPU-generic for broad compatibility.

### Fixed

- AVIF inputs no longer fail immediately when force re-encoding is enabled.
- Large conversions are less likely to fail from queued raw-image buffers piling up faster than the encoder and packager can consume them.
- Page review no longer tries to load every active-volume thumbnail at once, reducing frontend lag, RAM spikes, resize stalls, and blank/black thumbnail states on very large inputs.
- Cancelled Tauri conversions stop the pipeline more reliably instead of continuing background work unnecessarily.

### Removed

- Removed the headless CLI crate and release artifacts. Thasia 0.3.0 is GUI/Tauri-first.
- Removed Intel macOS release artifacts from the publish matrix for v0.3.0.

## [0.2.0] - 2026-05-28

### Added

- **Direct manga downloads** — integrated Suwayomi support for discovering sources, searching manga, downloading chapters, and feeding downloaded pages into Thasia's conversion flow.
- **Suwayomi management** — app-side controls for installing, updating, starting, stopping, and restarting the bundled Suwayomi server.
- **Download workspace settings** — configurable download directory and source repository management for the discover/download view.
- **Tauri release workflow** — automated multi-platform desktop builds for Windows, Linux, Intel macOS, and Apple Silicon macOS.

### Changed

- Expanded the app from a local-file converter into an end-to-end manga download and conversion workflow.
- Improved settings persistence for conversion defaults and download/server preferences.
- Reworked parts of the UI for a more consistent and intuitive experience across the app.

## [0.1.0] - 2026-05-27

### Added

- **Initial Thasia release** — Tauri v2 + Svelte 5 desktop app backed by a Rust workspace.
- **Local source import** — process manga folders, ZIP files, and CBZ archives.
- **Parsing engine** — detects volumes, chapters, covers, and page ordering from common manga folder and filename patterns.
- **Visual conversion wizard** — source selection, destination setup, format selection, volume grouping, page review, and conversion progress.
- **Page editor** — reorder pages, exclude unwanted pages, insert custom images, and adjust output volumes before conversion.
- **Parallel image conversion** — Rayon-backed AVIF/WebP encoding with grayscale detection and optional max-width downscaling.
- **Output packaging** — CBZ, EPUB 3 fixed-layout, and raw directory output.
- **Themes and keyboard hints** — light/dark themes, shortcut hint bar, and keyboard-friendly navigation.
