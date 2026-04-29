use regex_lite::Regex;

/// Sanitize a user-supplied query string for safe use in FTS5 MATCH expressions.
///
/// Steps:
/// 1. Preserve quoted phrases (e.g. `"exact match"`)
/// 2. Strip FTS5 special characters
/// 3. Collapse consecutive stars
/// 4. Remove dangling boolean operators (AND/OR/NOT) at edges
/// 5. Restore preserved quoted phrases
pub fn sanitize_fts5_query(query: &str) -> String {
    // Step 1: Preserve quoted phrases
    let mut quoted: Vec<String> = Vec::new();
    let re_quote = Regex::new(r#""[^"]*""#).unwrap();
    let mut s = re_quote
        .replace_all(query, |c: &regex_lite::Captures| {
            quoted.push(c[0].to_string());
            format!("\x00Q{}\x00", quoted.len() - 1)
        })
        .to_string();

    // Step 2: Strip FTS5 special chars
    let special = Regex::new(r#"[+{}()\^"]"#).unwrap();
    s = special.replace_all(&s, " ").to_string();

    // Step 3: Collapse consecutive stars
    s = Regex::new(r"\*+").unwrap().replace_all(&s, "*").to_string();

    // Step 4: Remove dangling operators
    s = Regex::new(r"(?i)^(AND|OR|NOT)\s+").unwrap().replace_all(&s, "").to_string();
    s = Regex::new(r"(?i)\s+(AND|OR|NOT)$").unwrap().replace_all(&s, "").to_string();

    // Step 5: Restore quoted phrases
    for (i, q) in quoted.iter().enumerate() {
        s = s.replace(&format!("\x00Q{}\x00", i), q);
    }

    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_word() {
        assert_eq!(sanitize_fts5_query("hello"), "hello");
    }

    #[test]
    fn test_strips_special_chars() {
        assert_eq!(sanitize_fts5_query("hello+world"), "hello world");
        assert_eq!(sanitize_fts5_query("test{ing}"), "test ing");
    }

    #[test]
    fn test_collapses_stars() {
        assert_eq!(sanitize_fts5_query("hello***"), "hello*");
    }

    #[test]
    fn test_removes_dangling_operators() {
        assert_eq!(sanitize_fts5_query("AND hello"), "hello");
        assert_eq!(sanitize_fts5_query("hello OR"), "hello");
        assert_eq!(sanitize_fts5_query("NOT test"), "test");
    }

    #[test]
    fn test_preserves_quoted_phrases() {
        assert_eq!(sanitize_fts5_query(r#""exact phrase" rest"#), r#""exact phrase" rest"#);
    }

    #[test]
    fn test_empty_query() {
        assert_eq!(sanitize_fts5_query(""), "");
        assert_eq!(sanitize_fts5_query("   "), "");
    }

    #[test]
    fn test_combined_special() {
        let input = r#"AND "hello world" +test*** {}(boo)"#;
        let result = sanitize_fts5_query(input);
        assert!(result.contains(r#""hello world""#));
        assert!(result.contains("test*"));
        assert!(!result.contains("AND"));
        assert!(!result.contains("+"));
        assert!(!result.contains("{"));
    }
}
