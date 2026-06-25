<div align="center">

<img src=".github/assets/icon.png" width="128" height="128" />

# Thasia

**A fast manga converter and processor for Windows, Linux, and macOS**

Convert, resize, and package your manga into CBZ or EPUB formats. It features automatic folder detection, a visual page editor, AVIF/WebP encoding, and multi-core processing.

[![License](https://img.shields.io/badge/license-BSD%203-blue.svg)](LICENSE)
[![Version](https://img.shields.io/github/v/release/LuMiSxh/Thasia)](https://github.com/LuMiSxh/Thasia/releases)

[Features](#features) • [Installation](#installation) • [Quick Start](#quick-start) • [Development](#development)

<br />

<img src=".github/assets/App-Landing-Dark.png" width="100%" />

</div>

---

From _Anastasia_ (meaning "resurrection" or "rise again"), Thasia is a complete rewrite of my older project, [Palaxy](https://github.com/LuMiSxh/Palaxy). Built from scratch in Rust with a Tauri v2 and Svelte 5 frontend, it is designed to be much faster, cleaner, and easier to maintain.

---

## Features

### File & Directory Parsing

Thasia scans your folders and files to detect chapters and volumes automatically using a custom parser (`logos`):

- **Direct Archive Support:** Import `.zip` and `.cbz` files directly without having to unpack them first.
- **Smart Naming Detection:** Automatically recognizes chapters, volumes, and pages. It supports standard formats like HakuNeko-style names (`001-023/01.png`), explicit names (`Vol 1/Ch 24`), or nested folders.
- **Flexible Bundling:** Group your files by volume or merge everything into a single archive.

### Image Optimization & Encoding

Processes images on all available CPU cores using Rayon to speed up conversion:

- **Original:** Keeps original images untouched for quick packaging.
- **AVIF (AV1 Image Format):** Great compression. Includes a 5-tier quality selector and a fast grayscale detector that skips color encoding for black-and-white pages to save extra space.
- **WebP:** A good middle ground between file size, encoding speed, and compatibility with older e-readers.
- **Resizing:** Downscale overly large pages to a maximum width (like 1920px) to keep file sizes manageable.

### Visual Page Editor

Take control over your pages before you convert:

- **Drag & Drop:** Easily reorder pages if the automatic sort order isn't correct.
- **Custom Covers:** Set any image from your computer as the cover for your EPUB or CBZ.
- **Remove Pages:** Skip credit pages, translator notes, or ads with a single click.
- **Split Volumes:** Visually split chapters or pages into separate volumes before exporting.

### Supported Formats

- **CBZ (Comic Book Archive):** Uses standard ZIP compression. If you've already compressed your images to WebP/AVIF, it uses "Stored" mode to avoid wasting CPU power re-compressing them. Great for Komga, Kavita, or local readers.
- **EPUB 3.0:** Fixed-layout EPUBs made for e-readers, supporting both Left-to-Right (LTR) and Right-to-Left (RTL) reading directions.
- **Raw Directory:** Saves the organized and optimized pages into flat, numbered folders.

### User Interface

- **Keyboard Shortcuts:** Navigate and toggle options quickly using shortcuts (like `Shift+Arrows`), with a helper bar at the bottom of the window.
- **Themes:** Switch between a clean Light and Dark mode depending on your preference.

<p align="center">
  <img src=".github/assets/App-Landing-Dark.png" width="48%" />
  <img src=".github/assets/App-Landing-Light.png" width="48%" />
</p>

## Manga Downloader Integration

Thasia can connect with [Suwayomi](https://github.com/Suwayomi/Suwayomi-Server), letting you search and download manga from various online sources directly inside the app.

- **Easy Setup:** Thasia can download and run Suwayomi-Server in the background automatically.
- **Direct Import:** Search for titles and download chapters straight into your conversion list.
- **No Manual File Moving:** You don't need to juggle external downloader tools; the files are ready to edit and pack right away.

<p align="center">
  <img src=".github/assets/App-Suwayomi.png" width="100%" />
</p>

---

## Installation

Visit the [releases page](https://github.com/LuMiSxh/Thasia/releases) and download the latest version for your operating system:

- **Windows**: `Thasia_x.x.x_x64_en-US.msi` or `Thasia_x.x.x_x64-setup.exe`
- **macOS Intel**: `Thasia_x.x.x_x64.dmg`
- **macOS Apple Silicon**: `Thasia_x.x.x_aarch64.dmg`
- **Linux**: `Thasia_x.x.x_amd64.AppImage`, `.deb`, or `.rpm`

### System Requirements

- **OS**: Windows 10+, macOS 11+, or modern Linux distribution
- **RAM**: 2GB minimum, 4GB+ recommended for parallelized AVIF encoding
- **Disk**: Temporary space equal to the size of your input manga (if using archives)

---

## Quick Start

### Graphical Interface Workflow

Thasia guides you through the process step-by-step:

**Step 1: Source & Destination**
Drag and drop your folders, `.zip`, or `.cbz` files into the app, then choose where to save the output.

**Step 2: Format & Encoding**
Select your output format (CBZ, EPUB, or Raw) and image encoding (AVIF, WebP, or Original). If you're exporting to EPUB, choose your reading direction (RTL or LTR).

<br />
<img src=".github/assets/App-Pipeline-Formats.png" width="100%" />

**Step 3: Bundling & Volumes**
Tweak how chapters are grouped into volumes if the automatic detection didn't get it quite right.

<br />
<img src=".github/assets/App-Pipeline-Volume.png" width="100%" />

**Step 4: Page Editor**
Check the pages before converting. You can drag them around to reorder, exclude unwanted pages, or add a custom cover.

<br />
<img src=".github/assets/App-Pipeline-Filter.png" width="100%" />

**Step 5: Convert**
Review your settings and click "Start Converting". The app will use your CPU's available cores to process and pack everything.

## Development

Thasia is built using a modern stack: **Rust (Edition 2024)**, **Tauri v2**, **Svelte 5**, and **Tailwind CSS v4**.

### Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [pnpm](https://pnpm.io/) (required)
- [Rust](https://www.rust-lang.org/) 1.70+
- Rust component: `rustup component add llvm-tools-preview`
- [CMake](https://cmake.org/) (required for bundled AVIF decoding)
- [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS

Linux developers should also install `lld` for the workspace linker configuration:

```bash
sudo apt-get install lld
```

### Setup

```bash
# Clone the repository
git clone https://github.com/LuMiSxh/Thasia.git
cd Thasia

# Install frontend dependencies
pnpm install

# Run the Tauri app in development mode
pnpm run tauri dev

# Build for production
pnpm run tauri build
```

---

## License

This project is licensed under the BSD-3 Clause License - see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- Built with [Tauri v2](https://v2.tauri.app) and [Svelte 5](https://svelte.dev)
- Fast AVIF encoding powered by [ravif](https://github.com/kornelski/ravif-rs) and [rav1e](https://github.com/xiph/rav1e)
- The successor to [Palaxy](https://github.com/LuMiSxh/Palaxy)

---

<div align="center">

**An open-source project by LuMiSxh**

[GitHub](https://github.com/LuMiSxh/Thasia) • [Issues](https://github.com/LuMiSxh/Thasia/issues) • [Releases](https://github.com/LuMiSxh/Thasia/releases)

</div>
