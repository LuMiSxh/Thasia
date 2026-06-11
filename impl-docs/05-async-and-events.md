# Async Tasks & Progress Events

## Current model

Tauri runs commands on a tokio runtime and pushes progress back to the WebView via
`app_handle.emit("event-name", payload)`. The frontend subscribes with
`listen("event-name", handler)`. This requires serialization at every step.

---

## GPUI model

GPUI owns a `BackgroundExecutor` (tokio under the hood). Tasks are spawned with
`cx.spawn()` or `cx.background_executor().spawn()`. To update UI from a task,
you capture a `WeakEntity` handle and call `.update()` on it — this schedules a
mutation on the main thread.

### Pattern: spawn + callback

```rust
// Inside a ConvertView method, triggered by a button click:
fn start_conversion(&mut self, cx: &mut Context<Self>) {
    self.is_running = true;
    self.progress = None;
    self.error = None;
    cx.notify();

    let options = self.options.clone();
    let edits = self.edits.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    cx.update_global::<ConvState, _>(|s, _| s.cancel_flag = cancel.clone());

    let weak = cx.weak_entity();

    cx.spawn(|cx| async move {
        let result = convert(
            options,
            edits,
            cancel,
            move |event| {
                // This closure is called from the async task thread.
                // We schedule a UI update:
                weak.upgrade().map(|handle| {
                    handle.update(&mut cx.clone(), |view, cx| {
                        view.apply_progress_event(event);
                        cx.notify();
                    });
                });
            },
        ).await;

        weak.upgrade().map(|handle| {
            handle.update(&mut cx.clone(), |view, cx| {
                match result {
                    Ok(summary) => {
                        view.summary = Some(summary);
                        view.step = WizardStep::Complete;
                    }
                    Err(e) => view.error = Some(e),
                }
                view.is_running = false;
                cx.notify();
            });
        });
    }).detach();
}
```

---

## ConversionEvent enum (replaces all Tauri events)

```rust
// src/state/conv.rs
pub enum ConversionEvent {
    ScanProgress { current: u32, total: u32 },
    VolumeStart { volume_num: u32, volume_name: String, total_volumes: u32 },
    ImageProgress {
        volume_num: u32,
        current: u32,
        total: u32,
        elapsed_secs: f32,
        pages_per_sec: f32,
        estimated_remaining_secs: f32,
        input_bytes: u64,
        output_bytes: u64,
        passthrough_pages: u32,
        encoded_pages: u32,
        fetch_ms: u64,
        decode_ms: u64,
        transform_ms: u64,
        encode_ms: u64,
    },
    VolumeComplete { volume_num: u32, success: bool, error: Option<String>, output_path: Option<PathBuf> },
    Complete(ConversionSummary),
}
```

`apply_progress_event` in `ConvertView` matches on this enum to update
`self.progress`.

---

## Suwayomi lifecycle events

The SuwayomiManager emits state changes. Replace event emission with a channel:

```rust
// In DiscoverView::start_suwayomi():
let (tx, mut rx) = tokio::sync::mpsc::channel::<SuwayomiEvent>(32);
let weak = cx.weak_entity();

// Pass tx to manager start call:
manager.start(tx).await?;

// Listen loop:
cx.spawn(|cx| async move {
    while let Some(event) = rx.recv().await {
        weak.upgrade().map(|h| h.update(&mut cx.clone(), |view, cx| {
            view.apply_suwayomi_event(event);
            cx.notify();
        }));
    }
}).detach();
```

---

## Task cancellation

```rust
// Cancel button handler
fn cancel_conversion(&mut self, cx: &mut Context<Self>) {
    cx.update_global::<ConvState, _>(|s, _| {
        s.cancel_flag.store(true, Ordering::Relaxed);
    });
    // The running task checks the flag between volumes.
    // ConversionEvent::Complete (with partial results) will arrive and update UI.
}
```

---

## Background executor for CPU-bound work

For CPU-intensive image ops that must not block the UI thread:

```rust
cx.background_executor().spawn(async move {
    // runs on tokio blocking pool
    heavy_image_decode(bytes)
}).await
```

GPUI's `BackgroundExecutor` wraps tokio's blocking spawn for non-async CPU tasks.
