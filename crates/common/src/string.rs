//! String utilities for cross-platform compatibility.

/// Replace Unicode surrogate code points with the replacement character.
///
/// Rust `char` cannot represent surrogates (U+D800–U+DFFF), but JSON strings
/// decoded via UTF-16 paths (common on Windows) can contain them. This function
/// sanitizes such strings by replacing each surrogate byte pair with U+FFFD.
///
/// # Implementation
///
/// Since Rust `char` is never a surrogate, we operate on the raw UTF-8 bytes.
/// Surrogates encode as 3-byte UTF-8 sequences: `ED A0 80` – `ED BF BF`.
pub fn sanitize_surrogates(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = String::with_capacity(s.len());
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // Check for 3-byte UTF-8 sequences starting with 0xED (surrogate range)
        if b == 0xED && i + 2 < bytes.len() {
            let b1 = bytes[i + 1];
            let b2 = bytes[i + 2];
            // Surrogates: ED A0 80 (D800) through ED BF BF (DFFF)
            // b1 range: 0xA0–0xBF, b2 range: 0x80–0xBF
            if (0xA0..=0xBF).contains(&b1) && (0x80..=0xBF).contains(&b2) {
                result.push('\u{FFFD}');
                i += 3;
                continue;
            }
        }

        // Valid byte — decode the full codepoint and advance
        let (c, len) = match b {
            0x00..=0x7F => (bytes[i] as char, 1),
            0xC0..=0xDF if i + 1 < bytes.len() => {
                let cp = decode_two(bytes[i], bytes[i + 1]);
                (char_from_u32(cp), 2)
            }
            0xE0..=0xEF if i + 2 < bytes.len() => {
                let cp = decode_three(bytes[i], bytes[i + 1], bytes[i + 2]);
                (char_from_u32(cp), 3)
            }
            0xF0..=0xF7 if i + 3 < bytes.len() => {
                let cp = decode_four(bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]);
                (char_from_u32(cp), 4)
            }
            // Invalid UTF-8 lead byte — replace and skip
            _ => {
                result.push('\u{FFFD}');
                i += 1;
                continue;
            }
        };

        result.push(c);
        i += len;
    }

    result
}

#[inline]
fn decode_two(b0: u8, b1: u8) -> u32 {
    ((b0 as u32 & 0x1F) << 6) | (b1 as u32 & 0x3F)
}

#[inline]
fn decode_three(b0: u8, b1: u8, b2: u8) -> u32 {
    ((b0 as u32 & 0x0F) << 12) | ((b1 as u32 & 0x3F) << 6) | (b2 as u32 & 0x3F)
}

#[inline]
fn decode_four(b0: u8, b1: u8, b2: u8, b3: u8) -> u32 {
    ((b0 as u32 & 0x07) << 18)
        | ((b1 as u32 & 0x3F) << 12)
        | ((b2 as u32 & 0x3F) << 6)
        | (b3 as u32 & 0x3F)
}

/// Convert u32 to char, using replacement character for invalid values.
#[inline]
fn char_from_u32(cp: u32) -> char {
    char::from_u32(cp).unwrap_or('\u{FFFD}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_string_unchanged() {
        let input = "Hello, world! 你好 🦀";
        assert_eq!(sanitize_surrogates(input), input);
    }

    #[test]
    fn empty_string() {
        assert_eq!(sanitize_surrogates(""), "");
    }

    #[test]
    fn ascii_only() {
        let input = "ABC123xyz";
        assert_eq!(sanitize_surrogates(input), input);
    }

    #[test]
    fn valid_three_byte_utf8_preserved() {
        // U+D7FF = ED 9F BF — last valid codepoint before surrogate range
        let d7ff = "\u{D7FF}";
        assert_eq!(sanitize_surrogates(d7ff), d7ff);

        // U+E000 = EE 80 80 — first valid codepoint after surrogate range
        let e000 = "\u{E000}";
        assert_eq!(sanitize_surrogates(e000), e000);

        // Regular 3-byte chars like Chinese
        let chinese = "你好世界";
        assert_eq!(sanitize_surrogates(chinese), chinese);
    }

    #[test]
    fn valid_four_byte_utf8_preserved() {
        // Emoji and other 4-byte chars
        let emoji = "🦀🎉🚀";
        assert_eq!(sanitize_surrogates(emoji), emoji);
    }

    #[test]
    fn boundary_check_d7ff_not_replaced() {
        // ED 9F BF = U+D7FF, just below surrogate range (A0 starts surrogates)
        // This should NOT be replaced since b1=9F is not in A0-BF range
        let input = "\u{D7FF}";
        assert_eq!(sanitize_surrogates(input), "\u{D7FF}");
    }

    #[test]
    fn boundary_check_e000_not_replaced() {
        // EE 80 80 = U+E000, just above surrogate range (ED prefix different)
        // This should NOT be replaced since b0=EE is not ED
        let input = "\u{E000}";
        assert_eq!(sanitize_surrogates(input), "\u{E000}");
    }

    #[test]
    fn mixed_valid_content() {
        let input = "Hello 你好 🦀 world!";
        assert_eq!(sanitize_surrogates(input), input);
    }
}
