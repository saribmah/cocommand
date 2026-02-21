use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use crate::error::CoreError;
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};

use super::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};

const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024; // 5 MB
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DDG_HTML_URL: &str = "https://html.duckduckgo.com/html/";
const DDG_TIMEOUT_SECS: u64 = 15;

pub struct WebExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl std::fmt::Debug for WebExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebExtension").finish()
    }
}

impl Default for WebExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl WebExtension {
    pub fn new() -> Self {
        let manifest = parse_builtin_manifest(include_str!("web_manifest.json"));

        let mut execute_map = HashMap::new();

        // ── web_fetch ──────────────────────────────────────────────
        execute_map.insert(
            "web_fetch",
            Arc::new(
                |input: serde_json::Value, _context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let url = input
                            .get("url")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| CoreError::Internal("missing url".to_string()))?;

                        if !url.starts_with("http://") && !url.starts_with("https://") {
                            return Err(CoreError::Internal(
                                "url must start with http:// or https://".to_string(),
                            ));
                        }

                        let format = input
                            .get("format")
                            .and_then(|v| v.as_str())
                            .unwrap_or("text");

                        let timeout_secs = input
                            .get("timeoutSeconds")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(DEFAULT_TIMEOUT_SECS)
                            .min(120);

                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                            .timeout(Duration::from_secs(timeout_secs))
                            .build()
                            .map_err(|e| CoreError::Internal(format!("failed to build http client: {e}")))?;

                        let response = client
                            .get(url)
                            .send()
                            .await
                            .map_err(|e| CoreError::Internal(format!("fetch failed: {e}")))?;

                        let status = response.status().as_u16();
                        let content_type = response
                            .headers()
                            .get("content-type")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("unknown")
                            .to_string();

                        let bytes = response
                            .bytes()
                            .await
                            .map_err(|e| CoreError::Internal(format!("failed to read body: {e}")))?;

                        if bytes.len() > MAX_RESPONSE_BYTES {
                            return Err(CoreError::Internal(format!(
                                "response too large: {} bytes (max {})",
                                bytes.len(),
                                MAX_RESPONSE_BYTES
                            )));
                        }

                        let content = match format {
                            "html" => String::from_utf8_lossy(&bytes).to_string(),
                            "text" => html2text::from_read(&bytes[..], 80)
                                .map_err(|e| CoreError::Internal(format!("html2text error: {e}")))?,
                            "markdown" => html2text::from_read(&bytes[..], 120)
                                .map_err(|e| CoreError::Internal(format!("html2text error: {e}")))?,
                            _ => html2text::from_read(&bytes[..], 80)
                                .map_err(|e| CoreError::Internal(format!("html2text error: {e}")))?,
                        };

                        let content_length = content.len();

                        Ok(json!({
                            "url": url,
                            "status": status,
                            "contentType": content_type,
                            "content": content,
                            "format": format,
                            "contentLength": content_length
                        }))
                    })
                },
            ) as _,
        );

        // ── web_search ─────────────────────────────────────────────
        execute_map.insert(
            "web_search",
            Arc::new(
                |input: serde_json::Value, _context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let query = input
                            .get("query")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| CoreError::Internal("missing query".to_string()))?;

                        let max_results = input
                            .get("numResults")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(10) as usize;

                        let results = ddg_search(query, max_results).await?;
                        let count = results.len();

                        Ok(json!({
                            "results": results,
                            "query": query,
                            "count": count
                        }))
                    })
                },
            ) as _,
        );

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self { manifest, tools }
    }
}

/// Search DuckDuckGo via the HTML lite endpoint and parse results.
async fn ddg_search(query: &str, max_results: usize) -> Result<Vec<serde_json::Value>, CoreError> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(DDG_TIMEOUT_SECS))
        .build()
        .map_err(|e| CoreError::Internal(format!("failed to build http client: {e}")))?;

    let response = client
        .post(DDG_HTML_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("q={}&b=", urlencoding::encode(query)))
        .send()
        .await
        .map_err(|e| CoreError::Internal(format!("DuckDuckGo search failed: {e}")))?;

    let html = response
        .text()
        .await
        .map_err(|e| CoreError::Internal(format!("failed to read DuckDuckGo response: {e}")))?;

    Ok(parse_ddg_html(&html, max_results))
}

/// Parse DuckDuckGo HTML lite response into structured results.
///
/// The HTML lite page contains result blocks with:
///   - `<a class="result__a" href="...">TITLE</a>`
///   - `<a class="result__snippet" ...>SNIPPET</a>`
///   - `<a class="result__url" href="...">DISPLAY_URL</a>`
fn parse_ddg_html(html: &str, max_results: usize) -> Vec<serde_json::Value> {
    let mut results = Vec::new();
    let mut pos = 0;

    while results.len() < max_results {
        // Find next result link: class="result__a"
        let marker = "class=\"result__a\"";
        let marker_pos = match html[pos..].find(marker) {
            Some(i) => pos + i,
            None => break,
        };

        // Backtrack to find the <a that contains this class
        let a_start = match html[..marker_pos].rfind("<a ") {
            Some(i) => i,
            None => {
                pos = marker_pos + marker.len();
                continue;
            }
        };

        // Extract href from the <a> tag
        let a_tag_end = match html[a_start..].find('>') {
            Some(i) => a_start + i,
            None => {
                pos = marker_pos + marker.len();
                continue;
            }
        };
        let a_tag = &html[a_start..a_tag_end];
        let href = match extract_attr(a_tag, "href") {
            Some(h) => h,
            None => {
                pos = a_tag_end;
                continue;
            }
        };

        // Extract title text between > and </a>
        let title_start = a_tag_end + 1;
        let title_end = match html[title_start..].find("</a>") {
            Some(i) => title_start + i,
            None => {
                pos = title_start;
                continue;
            }
        };
        let title = strip_html_tags(&html[title_start..title_end]);

        // Look for snippet nearby: class="result__snippet"
        let search_region_end = (title_end + 2000).min(html.len());
        let snippet = html[title_end..search_region_end]
            .find("class=\"result__snippet\"")
            .and_then(|snippet_class_pos| {
                let abs = title_end + snippet_class_pos;
                let tag_end = html[abs..search_region_end].find('>')? + abs + 1;
                let close = html[tag_end..search_region_end].find("</a>")? + tag_end;
                Some(strip_html_tags(&html[tag_end..close]))
            })
            .unwrap_or_default();

        // Resolve the URL — DDG wraps links through a redirect
        let url = resolve_ddg_url(&href);

        if !title.is_empty() && !url.is_empty() {
            results.push(json!({
                "url": url,
                "title": title,
                "snippet": snippet
            }));
        }

        pos = title_end + 4;
    }

    results
}

/// Resolve DuckDuckGo redirect URLs to the actual destination.
///
/// DDG lite wraps links as `//duckduckgo.com/l/?uddg=ENCODED_URL&...`
fn resolve_ddg_url(href: &str) -> String {
    if let Some(rest) = href
        .strip_prefix("//duckduckgo.com/l/?uddg=")
        .or_else(|| href.strip_prefix("/l/?uddg="))
    {
        // Extract the encoded URL (up to the next & or end)
        let encoded = rest.split('&').next().unwrap_or(rest);
        urlencoding::decode(encoded)
            .map(|s| s.into_owned())
            .unwrap_or_else(|_| href.to_string())
    } else if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else {
        String::new()
    }
}

/// Extract the value of an HTML attribute from a tag string.
fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = tag.find(&pattern)? + pattern.len();
    let end = tag[start..].find('"')? + start;
    Some(html_decode(&tag[start..end]))
}

/// Strip HTML tags from a string, returning plain text.
fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    html_decode(out.trim())
}

/// Decode common HTML entities.
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

#[async_trait::async_trait]
impl Extension for WebExtension {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        self.manifest
            .routing
            .as_ref()
            .and_then(|r| r.keywords.clone())
            .unwrap_or_default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        self.tools.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_extension_has_expected_tools() {
        let ext = WebExtension::new();
        let tools = ext.tools();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].id, "web_fetch");
        assert_eq!(tools[1].id, "web_search");
    }

    #[test]
    fn parse_ddg_html_extracts_results() {
        let html = r##"
        <div class="result">
            <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com">Example Site</a>
            <a class="result__snippet" href="#">This is a snippet about example.</a>
        </div>
        <div class="result">
            <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org&amp;rut=abc">Rust &amp; Language</a>
            <a class="result__snippet" href="#">A <b>systems</b> programming language.</a>
        </div>
        "##;
        let results = parse_ddg_html(html, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["url"], "https://example.com");
        assert_eq!(results[0]["title"], "Example Site");
        assert_eq!(results[0]["snippet"], "This is a snippet about example.");
        assert_eq!(results[1]["url"], "https://rust-lang.org");
        assert_eq!(results[1]["title"], "Rust & Language");
        assert_eq!(results[1]["snippet"], "A systems programming language.");
    }

    #[test]
    fn parse_ddg_html_respects_max_results() {
        let html = r##"
        <a class="result__a" href="https://a.com">A</a><a class="result__snippet" href="#">a</a>
        <a class="result__a" href="https://b.com">B</a><a class="result__snippet" href="#">b</a>
        <a class="result__a" href="https://c.com">C</a><a class="result__snippet" href="#">c</a>
        "##;
        let results = parse_ddg_html(html, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn parse_ddg_html_handles_empty() {
        let html = "<html><body>No results found</body></html>";
        let results = parse_ddg_html(html, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn resolve_ddg_url_decodes_redirect() {
        let href = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpath&rut=abc";
        assert_eq!(resolve_ddg_url(href), "https://example.com/path");
    }

    #[test]
    fn resolve_ddg_url_passes_direct_urls() {
        assert_eq!(
            resolve_ddg_url("https://example.com"),
            "https://example.com"
        );
    }

    #[test]
    fn resolve_ddg_url_rejects_relative() {
        assert_eq!(resolve_ddg_url("/some/path"), "");
    }

    #[test]
    fn parse_ddg_html_handles_real_response_structure() {
        // Mimics the actual DDG HTML lite response structure
        let html = r##"
        <div class="result results_links results_links_deep web-result">
          <div class="links_main links_deep result__body">
            <h2 class="result__title">
              <a rel="nofollow" class="result__a" href="https://rust-lang.org/">Rust Programming Language</a>
            </h2>
            <div class="result__extras">
              <div class="result__extras__url">
                <a class="result__url" href="https://rust-lang.org/">rust-lang.org</a>
              </div>
            </div>
            <a class="result__snippet" href="https://rust-lang.org/"><b>Rust</b> is a fast, reliable, and productive <b>programming</b> <b>language</b>.</a>
          </div>
        </div>
        <div class="result results_links results_links_deep web-result">
          <div class="links_main links_deep result__body">
            <h2 class="result__title">
              <a rel="nofollow" class="result__a" href="https://en.wikipedia.org/wiki/Rust_(programming_language)">Rust (programming language) - Wikipedia</a>
            </h2>
            <div class="result__extras">
              <div class="result__extras__url">
                <a class="result__url" href="https://en.wikipedia.org/wiki/Rust_(programming_language)">en.wikipedia.org</a>
              </div>
            </div>
            <a class="result__snippet" href="https://en.wikipedia.org/wiki/Rust_(programming_language)">It&#x27;s noted for its emphasis on performance and memory safety.</a>
          </div>
        </div>
        "##;
        let results = parse_ddg_html(html, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["url"], "https://rust-lang.org/");
        assert_eq!(results[0]["title"], "Rust Programming Language");
        assert_eq!(
            results[0]["snippet"],
            "Rust is a fast, reliable, and productive programming language."
        );
        assert_eq!(
            results[1]["url"],
            "https://en.wikipedia.org/wiki/Rust_(programming_language)"
        );
        assert_eq!(
            results[1]["title"],
            "Rust (programming language) - Wikipedia"
        );
        assert_eq!(
            results[1]["snippet"],
            "It's noted for its emphasis on performance and memory safety."
        );
    }
}
