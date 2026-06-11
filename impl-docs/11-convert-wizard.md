# Convert View: Wizard Implementation

## Overview

The Convert page is the most complex view — a 9-step wizard that replaces the
SvelteKit wizard at `src/routes/convert/` and `src/lib/wizard/state.svelte`.

---

## WizardStep enum

```rust
// src/views/convert/mod.rs
#[derive(Clone, PartialEq)]
pub enum WizardStep {
    SelectSource,     // Step 1: pick file/folder
    Scanning,         // Step 2: scanning in progress (auto-advances)
    Configure,        // Step 3: image options (format, resize, enhance…)
    EditVolumes,      // Step 4: per-volume name/output overrides
    OutputOptions,    // Step 5: output dir, format (CBZ/EPUB/Raw), name pattern
    PipelinePlan,     // Step 6: computed cost preview
    Confirm,          // Step 7: final confirmation before run
    Converting,       // Step 8: live progress (auto-advances on complete)
    Complete,         // Step 9: summary + open output
}
```

---

## ConvertView entity

```rust
pub struct ConvertView {
    // State
    pub step: WizardStep,
    pub source_path: Option<PathBuf>,
    pub scan_result: Option<Vec<VolumeMeta>>,
    pub volume_edits: Vec<VolumeEdit>,
    pub options: ConvertOptions,
    pub pipeline_plan: Option<PipelinePlan>,

    // Progress
    pub progress: ConversionProgress,
    pub completed_volumes: Vec<VolumeCompleteInfo>,
    pub summary: Option<ConversionSummary>,

    // Error
    pub error: Option<AppError>,
    pub is_running: bool,
    pub is_cancelling: bool,
}

#[derive(Default)]
pub struct ConversionProgress {
    pub current_volume: u32,
    pub total_volumes: u32,
    pub current_volume_name: String,
    pub image_current: u32,
    pub image_total: u32,
    pub pages_per_sec: f32,
    pub elapsed_secs: f32,
    pub estimated_remaining_secs: f32,
    pub input_bytes: u64,
    pub output_bytes: u64,
}
```

---

## Step rendering dispatch

```rust
impl Render for ConvertView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .fill()
            .flex_col()
            .child(self.render_step_header(cx))
            .child(match &self.step {
                WizardStep::SelectSource  => self.render_select_source(window, cx),
                WizardStep::Scanning      => self.render_scanning(cx),
                WizardStep::Configure     => self.render_configure(cx),
                WizardStep::EditVolumes   => self.render_edit_volumes(cx),
                WizardStep::OutputOptions => self.render_output_options(cx),
                WizardStep::PipelinePlan  => self.render_pipeline_plan(cx),
                WizardStep::Confirm       => self.render_confirm(cx),
                WizardStep::Converting    => self.render_converting(cx),
                WizardStep::Complete      => self.render_complete(cx),
            })
    }
}
```

---

## Step 1: SelectSource

```rust
fn render_select_source(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
    let handle = cx.weak_entity();
    Card::new()
        .child(
            div()
                .flex_col()
                .gap_standard()
                .child("Select a manga archive (.zip, .cbz) or folder")
                .child(
                    Button::new("Browse…")
                        .variant(ButtonVariant::Primary)
                        .on_click(move |_, _, cx| {
                            let h = handle.clone();
                            cx.spawn(|cx| async move {
                                let file = rfd::AsyncFileDialog::new()
                                    .add_filter("Archives", &["zip", "cbz"])
                                    .pick_file()
                                    .await;
                                if let Some(f) = file {
                                    h.update(&mut cx, |view, cx| {
                                        view.source_path = Some(f.path().to_path_buf());
                                        cx.notify();
                                    });
                                }
                            }).detach();
                        })
                )
                .child(
                    self.source_path.as_ref().map(|p| {
                        PathDisplay::new(p.display().to_string())
                    })
                )
                .child(
                    Button::new("Scan source")
                        .variant(ButtonVariant::Primary)
                        .disabled(self.source_path.is_none())
                        .on_click(cx.listener(Self::start_scan))
                )
        )
        .into_any_element()
}
```

---

## Step 8: Converting (live progress)

```rust
fn render_converting(&self, cx: &mut Context<Self>) -> AnyElement {
    let p = &self.progress;
    div()
        .flex_col()
        .gap_standard()
        .padding_standard()
        .child(
            div()
                .text_primary(cx)
                .child(format!("Volume {}/{}: {}", p.current_volume, p.total_volumes, p.current_volume_name))
        )
        .child(
            ProgressBar::new(
                p.image_current as f32 / p.image_total.max(1) as f32
            )
        )
        .child(
            div()
                .flex()
                .gap_standard()
                .child(Badge::new(format!("{:.1} pages/s", p.pages_per_sec)))
                .child(Badge::new(format!("~{:.0}s remaining", p.estimated_remaining_secs)))
                .child(Badge::new(format!(
                    "{} → {}",
                    format_bytes(p.input_bytes),
                    format_bytes(p.output_bytes)
                )))
        )
        .child(
            Button::new(if self.is_cancelling { "Cancelling…" } else { "Cancel" })
                .variant(ButtonVariant::Secondary)
                .disabled(self.is_cancelling)
                .on_click(cx.listener(Self::cancel_conversion))
        )
        .into_any_element()
}
```

---

## Resetting the wizard

```rust
impl ConvertView {
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        *self = ConvertView::default();
        cx.notify();
    }
}
```

Called from Step 9 Complete → "Convert another" button, or from NavigateConvert
action if already complete.
