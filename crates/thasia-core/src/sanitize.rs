use crate::{Result, ThasiaError};

const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn sanitize_filename_component(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ThasiaError::Fatal("Output name cannot be empty".into()));
    }
    if matches!(trimmed, "." | "..") || trimmed.contains("..") {
        return Err(ThasiaError::Fatal(format!(
            "Invalid output name component: {input}"
        )));
    }
    if trimmed.ends_with('.') || trimmed.ends_with(' ') {
        return Err(ThasiaError::Fatal(format!(
            "Output name cannot end with a dot or space: {input}"
        )));
    }
    if trimmed.chars().any(|c| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        return Err(ThasiaError::Fatal(format!(
            "Output name contains characters that are not safe for filenames: {input}"
        )));
    }

    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or(trimmed)
        .to_ascii_uppercase();
    if WINDOWS_RESERVED_NAMES.contains(&stem.as_str()) {
        return Err(ThasiaError::Fatal(format!(
            "Output name is reserved on Windows: {input}"
        )));
    }

    Ok(trimmed.to_string())
}

pub fn escape_xml_text(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_filename_component() {
        assert_eq!(
            sanitize_filename_component("Manga - 01").unwrap(),
            "Manga - 01"
        );
    }

    #[test]
    fn rejects_path_traversal_and_separators() {
        assert!(sanitize_filename_component("../out").is_err());
        assert!(sanitize_filename_component("foo/bar").is_err());
        assert!(sanitize_filename_component("foo\\bar").is_err());
    }

    #[test]
    fn rejects_windows_reserved_names() {
        assert!(sanitize_filename_component("CON").is_err());
        assert!(sanitize_filename_component("LPT1.txt").is_err());
    }

    #[test]
    fn escapes_xml_text() {
        assert_eq!(
            escape_xml_text("A&B <C> \"D\" 'E'"),
            "A&amp;B &lt;C&gt; &quot;D&quot; &apos;E&apos;"
        );
    }
}
