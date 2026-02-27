/// Strip JSONC comments (// and /* */) from input, preserving string contents.
/// Replaces comments with spaces to preserve character positions for error reporting.
/// Works at the byte level to correctly handle multi-byte UTF-8 characters.
/// All JSON/JSONC structural characters are ASCII, so byte-level scanning is safe;
/// non-ASCII bytes are copied through unchanged.
pub fn strip_jsonc_comments(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut result: Vec<u8> = Vec::with_capacity(len);
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            if bytes[i] == b'\\' && i + 1 < len {
                result.push(bytes[i]);
                result.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            result.push(bytes[i]);
            i += 1;
        } else if bytes[i] == b'"' {
            in_string = true;
            result.push(b'"');
            i += 1;
        } else if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            // Line comment: replace with spaces until newline
            i += 2;
            result.push(b' ');
            result.push(b' ');
            while i < len && bytes[i] != b'\n' {
                result.push(b' ');
                i += 1;
            }
        } else if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            // Block comment: replace with spaces, preserve newlines
            result.push(b' ');
            result.push(b' ');
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    result.push(b' ');
                    result.push(b' ');
                    i += 2;
                    break;
                }
                if bytes[i] == b'\n' {
                    result.push(b'\n');
                } else {
                    result.push(b' ');
                }
                i += 1;
            }
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }

    // SAFETY: input is valid UTF-8, and we only copy original bytes or substitute
    // ASCII spaces/newlines for ASCII comment characters, so the result is valid UTF-8.
    String::from_utf8(result).expect("output should be valid UTF-8")
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

    #[test]
    fn test_utf8_multibyte_characters() {
        // Non-ASCII characters in string values and comments should be handled correctly
        let input = "{\n  \"name\": \"\u{4f60}\u{597d}\u{4e16}\u{754c}\" // \u{8fd9}\u{662f}\u{6ce8}\u{91ca}\n}";
        let result = strip_jsonc_comments(input);
        assert!(result.contains("\"\u{4f60}\u{597d}\u{4e16}\u{754c}\""));
        assert!(!result.contains("//"));
        assert!(!result.contains("\u{6ce8}\u{91ca}"));

        // Block comment with non-ASCII content
        let input2 = "{\n  /* \u{5757}\u{6ce8}\u{91ca} */\n  \"key\": \"\u{503c}\"\n}";
        let result2 = strip_jsonc_comments(input2);
        assert!(!result2.contains("\u{5757}\u{6ce8}\u{91ca}"));
        assert!(result2.contains("\"\u{503c}\""));
    }
}
