use crate::{Result, ThasiaError};

const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn sanitize_filename_component(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ThasiaError::EmptyFilename);
    }
    if matches!(trimmed, "." | "..") || trimmed.contains("..") {
        return Err(ThasiaError::InvalidFilenameComponent {
            value: input.to_string(),
        });
    }

    // Strip characters that are illegal on Windows or act as path separators.
    // Control characters and path separators are removed silently so manga
    // titles with punctuation like "?" don't hard-fail the conversion.
    let cleaned: String = trimmed
        .chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        .collect();

    let cleaned = cleaned.trim().trim_end_matches('.').trim_end_matches(' ');

    if cleaned.is_empty() {
        return Err(ThasiaError::EmptyFilename);
    }

    let stem = cleaned
        .split('.')
        .next()
        .unwrap_or(cleaned)
        .to_ascii_uppercase();
    if WINDOWS_RESERVED_NAMES.contains(&stem.as_str()) {
        return Err(ThasiaError::WindowsReservedFilename {
            value: input.to_string(),
        });
    }

    Ok(cleaned.to_string())
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
    fn rejects_path_traversal() {
        assert!(sanitize_filename_component("../out").is_err());
    }

    #[test]
    fn strips_unsafe_chars_silently() {
        // path separators stripped
        assert_eq!(sanitize_filename_component("foo/bar").unwrap(), "foobar");
        assert_eq!(sanitize_filename_component("foo\\bar").unwrap(), "foobar");
        // question mark and other Windows-illegal chars stripped
        assert_eq!(
            sanitize_filename_component("Jirai nandesu ka? Chihara-san").unwrap(),
            "Jirai nandesu ka Chihara-san"
        );
        assert_eq!(sanitize_filename_component("foo:bar*baz").unwrap(), "foobarbaz");
    }

    #[test]
    fn rejects_name_that_becomes_empty_after_stripping() {
        assert!(sanitize_filename_component("???").is_err());
        assert!(sanitize_filename_component("   ").is_err());
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
