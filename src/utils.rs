/// Strip JSONC comments (// and /* */) from input, preserving string contents.
/// Replaces comments with spaces to preserve character positions for error reporting.
pub fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            if bytes[i] == b'\\' && i + 1 < len {
                result.push(bytes[i] as char);
                result.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            result.push(bytes[i] as char);
            i += 1;
        } else if bytes[i] == b'"' {
            in_string = true;
            result.push('"');
            i += 1;
        } else if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            // Line comment: replace with spaces until newline
            i += 2;
            result.push(' ');
            result.push(' ');
            while i < len && bytes[i] != b'\n' {
                result.push(' ');
                i += 1;
            }
        } else if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            // Block comment: replace with spaces, preserve newlines
            result.push(' ');
            result.push(' ');
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    result.push(' ');
                    result.push(' ');
                    i += 2;
                    break;
                }
                if bytes[i] == b'\n' {
                    result.push('\n');
                } else {
                    result.push(' ');
                }
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_line_comments() {
        let input = "{\n  \"key\": \"value\" // comment\n}";
        let result = strip_jsonc_comments(input);
        assert!(!result.contains("//"));
        assert!(result.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_strip_block_comments() {
        let input = "{\n  /* block */\n  \"key\": \"value\"\n}";
        let result = strip_jsonc_comments(input);
        assert!(!result.contains("/*"));
        assert!(result.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_preserves_strings() {
        let input =
            r#"{"url": "https://example.com", "comment": "has // slashes and /* stars */"}"#;
        let result = strip_jsonc_comments(input);
        assert!(result.contains("https://example.com"));
        assert!(result.contains("has // slashes and /* stars */"));
    }

    #[test]
    fn test_preserves_newlines_in_block_comments() {
        let input = "{\n  /* line1\n     line2 */\n  \"key\": 1\n}";
        let result = strip_jsonc_comments(input);
        // Newlines should be preserved
        assert_eq!(result.matches('\n').count(), input.matches('\n').count());
    }
}
