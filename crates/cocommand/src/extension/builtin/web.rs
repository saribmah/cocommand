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
const EXA_API_URL: &str = "https://mcp.exa.ai/mcp";
const EXA_TIMEOUT_SECS: u64 = 25;

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

                        let num_results = input
                            .get("numResults")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(8);

                        let api_key = std::env::var("EXA_API_KEY").map_err(|_| {
                            CoreError::Internal(
                                "EXA_API_KEY environment variable is not set".to_string(),
                            )
                        })?;

                        let client = reqwest::Client::builder()
                            .timeout(Duration::from_secs(EXA_TIMEOUT_SECS))
                            .build()
                            .map_err(|e| {
                                CoreError::Internal(format!("failed to build http client: {e}"))
                            })?;

                        let body = json!({
                            "jsonrpc": "2.0",
                            "method": "tools/call",
                            "params": {
                                "name": "web_search_exa",
                                "arguments": {
                                    "query": query,
                                    "numResults": num_results,
                                    "livecrawl": "fallback"
                                }
                            },
                            "id": 1
                        });

                        let response = client
                            .post(EXA_API_URL)
                            .header("Authorization", format!("Bearer {api_key}"))
                            .header("Content-Type", "application/json")
                            .json(&body)
                            .send()
                            .await
                            .map_err(|e| {
                                CoreError::Internal(format!("exa search request failed: {e}"))
                            })?;

                        let response_text = response.text().await.map_err(|e| {
                            CoreError::Internal(format!("failed to read exa response: {e}"))
                        })?;

                        let results = parse_exa_sse_response(&response_text);

                        Ok(json!({
                            "results": results,
                            "query": query
                        }))
                    })
                },
            ) as _,
        );

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self { manifest, tools }
    }
}

/// Parse Exa SSE response, extracting text content from `data:` lines.
///
/// The Exa MCP endpoint may respond with SSE (Server-Sent Events) format where
/// each event line starts with `data: `. We look for the JSON-RPC result in
/// those lines and extract `result.content[].text`.
///
/// Falls back to trying a plain JSON parse if no SSE lines are found.
pub fn parse_exa_sse_response(raw: &str) -> String {
    // Try SSE format first: collect `data: ` lines
    let data_lines: Vec<&str> = raw
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .collect();

    if !data_lines.is_empty() {
        for line in &data_lines {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(texts) = extract_content_texts(&parsed) {
                    return texts;
                }
            }
        }
    }

    // Fallback: try parsing the entire response as plain JSON
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(texts) = extract_content_texts(&parsed) {
            return texts;
        }
    }

    // Last resort: return raw (trimmed)
    raw.trim().to_string()
}

/// Extract `result.content[].text` from a parsed JSON-RPC response.
fn extract_content_texts(value: &serde_json::Value) -> Option<String> {
    let content = value.get("result")?.get("content")?.as_array()?;
    let texts: Vec<&str> = content
        .iter()
        .filter_map(|item| item.get("text")?.as_str())
        .collect();
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
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
    fn parse_exa_sse_response_extracts_text() {
        let sse = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"Result 1\"},{\"type\":\"text\",\"text\":\"Result 2\"}]}}\n\n";
        let result = parse_exa_sse_response(sse);
        assert_eq!(result, "Result 1\nResult 2");
    }

    #[test]
    fn parse_exa_sse_response_handles_plain_json() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Plain JSON result"}]}}"#;
        let result = parse_exa_sse_response(json);
        assert_eq!(result, "Plain JSON result");
    }

    #[test]
    fn parse_exa_sse_response_fallback_on_empty() {
        let garbage = "not json at all";
        let result = parse_exa_sse_response(garbage);
        assert_eq!(result, "not json at all");
    }
}
