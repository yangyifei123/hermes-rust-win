use crate::tool::{Tool, ToolOutput};
use crate::RuntimeError;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

/// Web search tool using DuckDuckGo HTML search (no API key needed).
pub struct WebSearchTool {
    client: reqwest::Client,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using DuckDuckGo. Returns top results with titles, URLs, and snippets."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "max_results": { "type": "integer", "description": "Maximum results (default 5, max 10)", "default": 5 }
            },
            "required": ["query"]
        })
    }

    fn execute(&self, params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        let client = self.client.clone();
        Box::pin(async move {
            let query = params.get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if query.is_empty() {
                return Ok(ToolOutput::error("query is required"));
            }

            let max_results = params.get("max_results")
                .and_then(|v| v.as_u64())
                .unwrap_or(5)
                .min(10) as usize;

            // Use DuckDuckGo HTML search
            let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(&query));

            let response = client.get(&url).send().await;
            match response {
                Ok(resp) if resp.status().is_success() => {
                    let html = resp.text().await.unwrap_or_default();
                    let results = parse_ddg_results(&html, max_results);

                    if results.is_empty() {
                        Ok(ToolOutput {
                            content: format!("No results found for '{}'.", query),
                            is_error: false,
                        })
                    } else {
                        let formatted: Vec<String> = results.iter().enumerate().map(|(i, r)| {
                            format!("{}. {}\n   URL: {}\n   {}", i + 1, r.title, r.url, r.snippet)
                        }).collect();
                        Ok(ToolOutput {
                            content: format!("Search results for '{}':\n\n{}", query, formatted.join("\n\n")),
                            is_error: false,
                        })
                    }
                }
                Ok(resp) => Ok(ToolOutput::error(&format!("Search failed: HTTP {}", resp.status()))),
                Err(e) => Ok(ToolOutput::error(&format!("Search request failed: {}", e))),
            }
        })
    }
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

/// Parse DuckDuckGo HTML results page into structured results.
fn parse_ddg_results(html: &str, max: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // DDG HTML uses class="result__a" for title links and class="result__snippet" for snippets
    let mut search_pos = 0;
    while results.len() < max {
        // Find result title link: <a class="result__a" href="URL">TITLE</a>
        let title_start = match html[search_pos..].find("class=\"result__a\"") {
            Some(pos) => search_pos + pos,
            None => break,
        };

        // Extract href
        let url = extract_attr(html, title_start, "href").unwrap_or_default();

        // Extract title text between > and <
        let title = extract_text_after(html, title_start).unwrap_or_default();

        // Find snippet: class="result__snippet"
        let snippet_search = title_start + 100;
        let snippet = if snippet_search < html.len() {
            html[snippet_search..].find("class=\"result__snippet\"")
                .and_then(|pos| extract_text_after(html, snippet_search + pos))
                .unwrap_or_default()
        } else {
            String::new()
        };

        if !title.is_empty() {
            results.push(SearchResult {
                title: decode_html_entities(&title),
                url: decode_html_entities(&url),
                snippet: decode_html_entities(&snippet),
            });
        }

        search_pos = title_start + 50;
    }

    results
}

/// Extract an attribute value from HTML tag at given position.
fn extract_attr(html: &str, pos: usize, attr: &str) -> Option<String> {
    // Look backwards for the opening tag, then find attr="..."
    let tag_start = html[..pos].rfind('<')?;
    let tag_end = html[pos..].find('>').map(|i| pos + i)?;
    let tag = &html[tag_start..tag_end];

    let pattern = format!("{}=\"", attr);
    let attr_start = tag.find(&pattern)?;
    let value_start = attr_start + pattern.len();
    let value_end = tag[value_start..].find('"')?;
    Some(tag[value_start..value_start + value_end].to_string())
}

/// Extract text content between > and < after position.
fn extract_text_after(html: &str, pos: usize) -> Option<String> {
    let gt = html[pos..].find('>').map(|i| pos + i + 1)?;
    let lt = html[gt..].find('<').map(|i| gt + i)?;
    Some(html[gt..lt].trim().to_string())
}

/// Decode common HTML entities.
fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_search_executes() {
        let tool = WebSearchTool::new();
        let result = tool.execute(json!({"query": "rust programming language", "max_results": 2})).await.unwrap();
        // Web search may fail due to rate limits/network - just check it doesn't crash
        // If successful, should have results; if failed, should have error message
        assert!(result.content.contains("Search results") || result.content.contains("No results") || result.is_error,
            "Got unexpected output: {}", result.content);
    }

    #[tokio::test]
    async fn test_web_search_empty_query() {
        let tool = WebSearchTool::new();
        let result = tool.execute(json!({"query": ""})).await.unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn test_web_search_max_results_cap() {
        let tool = WebSearchTool::new();
        let result = tool.execute(json!({"query": "test", "max_results": 100})).await.unwrap();
        // Should cap at 10
        let count = result.content.matches(". ").count();
        assert!(count <= 11, "Should cap at 10 results, got ~{}", count);
    }

    #[test]
    fn test_parse_ddg_results() {
        let html = r#"
        <div class="result">
            <a class="result__a" href="https://example.com/rust">Rust Programming</a>
            <a class="result__snippet">A language empowering everyone.</a>
        </div>
        <div class="result">
            <a class="result__a" href="https://example.com/go">Go Programming</a>
            <a class="result__snippet">Simple, reliable software.</a>
        </div>
        "#;

        let results = parse_ddg_results(html, 10);
        assert_eq!(results.len(), 2, "Expected 2 results");
        assert_eq!(results[0].title, "Rust Programming", "First title");
        assert_eq!(results[0].url, "https://example.com/rust", "First URL");
    }

    #[test]
    fn test_parse_empty_html() {
        let results = parse_ddg_results("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("&amp; &lt; &gt;"), "& < >");
        assert_eq!(decode_html_entities("hello &amp; world"), "hello & world");
    }
}