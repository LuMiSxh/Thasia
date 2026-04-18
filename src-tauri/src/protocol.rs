use std::path::PathBuf;
use tauri::http::{Request, Response};

/// Build a `thasia://image?path={url_encoded_path}` URL for a given absolute path.
pub fn image_url(absolute_path: &std::path::Path) -> String {
    let encoded = urlencoding::encode(absolute_path.to_string_lossy().as_ref()).into_owned();
    format!("thasia://image?path={encoded}")
}

/// Synchronous handler registered with tauri::Builder::register_uri_scheme_protocol.
/// Reads the file at the decoded path and returns it with the appropriate MIME type.
pub fn handle(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();

    // Extract the `path` query parameter
    let path_str = match extract_path_param(&uri) {
        Some(p) => p,
        None => return error_response(400, "missing path param"),
    };

    let path = PathBuf::from(&path_str);
    if !path.exists() {
        return error_response(404, "file not found");
    }

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return error_response(500, "read error"),
    };

    let mime = mime_for_path(&path);

    Response::builder()
        .status(200)
        .header("Content-Type", mime)
        .header("Access-Control-Allow-Origin", "*")
        .body(bytes)
        .unwrap()
}

fn extract_path_param(uri: &str) -> Option<String> {
    // URI looks like: thasia://image?path=%2Ftmp%2Ffoo.jpg
    let query = uri.split('?').nth(1)?;
    for pair in query.split('&') {
        if let Some(val) = pair.strip_prefix("path=") {
            return urlencoding::decode(val).ok().map(|s| s.into_owned());
        }
    }
    None
}

fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        _ => "application/octet-stream",
    }
}

fn error_response(status: u16, msg: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .body(msg.as_bytes().to_vec())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_url_encodes_path() {
        let url = image_url(std::path::Path::new("/tmp/my file.jpg"));
        assert!(url.starts_with("thasia://image?path="));
        assert!(url.contains("%2Ftmp%2Fmy%20file.jpg") || url.contains("%2Ftmp%2Fmy+file.jpg"));
    }

    #[test]
    fn extract_path_param_decodes() {
        let uri = "thasia://image?path=%2Ftmp%2Ffoo.jpg";
        assert_eq!(extract_path_param(uri), Some("/tmp/foo.jpg".into()));
    }

    #[test]
    fn extract_path_param_missing_returns_none() {
        assert!(extract_path_param("thasia://image").is_none());
    }

    #[test]
    fn mime_for_jpeg() {
        assert_eq!(mime_for_path(std::path::Path::new("x.jpg")), "image/jpeg");
        assert_eq!(mime_for_path(std::path::Path::new("x.JPG")), "image/jpeg");
    }

    #[test]
    fn mime_for_png() {
        assert_eq!(mime_for_path(std::path::Path::new("x.png")), "image/png");
    }
}
