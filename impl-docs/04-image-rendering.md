# Image Rendering (replacing `thasia://` URI scheme)

## Current approach

Tauri registers a custom URI scheme `thasia://` so the WebView can load local files:

```
<img src="thasia://image?path=%2Ftmp%2Fmanga%2Fpage.jpg" />
```

The handler (`src-tauri/src/protocol.rs`) reads the file from disk, detects MIME type,
and returns bytes with CORS headers. This is necessary because WebViews cannot access
arbitrary filesystem paths.

---

## GPUI has no WebView — no scheme needed

GPUI renders to a native GPU surface. There is no WebView, no same-origin policy, and
no MIME type negotiation. Images are rendered directly.

GPUI provides two mechanisms:

### 1. `img()` with a file path (simplest)

```rust
use gpui::img;

// Renders a local JPEG, PNG, WebP, or AVIF directly from disk.
img(ImageSource::File(path.into()))
    .w(px(400.0))
    .h(px(600.0))
    .object_fit(ObjectFit::Contain)
```

GPUI decodes the image on a background thread and caches the GPU texture.
This is the direct replacement for `<img src="thasia://image?path=..." />`.

### 2. `img()` with `ImageData` (in-memory bytes)

If the image is already decoded or fetched from Suwayomi (HTTP), pass bytes directly:

```rust
let bytes: Arc<[u8]> = fetch_chapter_page(url).await?.into();
let image_data = ImageData::new(bytes); // GPUI handles decode
img(ImageSource::Data(image_data.clone()))
    .w_full()
    .h_full()
    .object_fit(ObjectFit::Contain)
```

Cache `ImageData` handles inside the view entity to avoid re-decoding on every render.

---

## Preview pane implementation

The Convert wizard shows thumbnail previews of scanned pages. In GPUI:

```rust
// src/views/convert/preview.rs
pub struct ImagePreviewPane {
    pub images: Vec<Option<ImageData>>,   // None = loading
    pub loading: HashSet<usize>,
}

impl ImagePreviewPane {
    pub fn load_images(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let handle = cx.entity_handle();
        for (i, path) in paths.into_iter().enumerate() {
            cx.spawn(|cx| async move {
                let bytes = tokio::fs::read(&path).await.ok()?;
                let data = ImageData::new(bytes.into());
                handle.update(&mut cx, |pane, cx| {
                    pane.images[i] = Some(data);
                    pane.loading.remove(&i);
                    cx.notify();
                });
            }).detach();
        }
    }
}

impl Render for ImagePreviewPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        uniform_list(cx.entity_handle(), "previews", self.images.len(), |pane, range, _, cx| {
            range.map(|i| {
                if let Some(data) = &pane.read(cx).images[i] {
                    img(ImageSource::Data(data.clone()))
                        .w(px(180.0))
                        .h(px(260.0))
                        .object_fit(ObjectFit::Cover)
                        .into_any_element()
                } else {
                    Spinner::new().into_any_element()
                }
            }).collect()
        })
        .h_full()
        .w(px(200.0))
    }
}
```

---

## Online images (Suwayomi chapter pages)

Suwayomi serves pages over HTTP (localhost). Use `reqwest` (already in the dependency
tree via thasia-source) to fetch bytes, then hand to `ImageData`:

```rust
let response = client.get(page_url).send().await?;
let bytes: Arc<[u8]> = response.bytes().await?.to_vec().into();
let data = ImageData::new(bytes);
```

Cache these per chapter in `DiscoverView` to avoid re-fetching on re-render.

---

## Summary of changes

| Current                               | GPUI                                 |
| ------------------------------------- | ------------------------------------ |
| `protocol.rs` custom URI handler      | **deleted**                          |
| CSP `img-src thasia:`                 | **deleted**                          |
| `<img src="thasia://image?path=...">` | `img(ImageSource::File(path))`       |
| `<img src="blob:...">` (in-memory)    | `img(ImageSource::Data(image_data))` |
| WebView texture upload                | GPUI GPU texture cache (automatic)   |
