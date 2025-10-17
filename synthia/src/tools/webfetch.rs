use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::time::Duration;

pub struct WebFetchTool {
    client: reqwest::Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Synthia/0.1.0")
                .build()
                .expect("Failed to create reqwest client"),
        }
    }

    fn parse_headers(&self, headers_json: Option<&Value>) -> Result<HeaderMap> {
        let mut header_map = HeaderMap::new();

        if let Some(headers) = headers_json {
            if let Some(obj) = headers.as_object() {
                for (key, value) in obj {
                    let header_name = HeaderName::try_from(key.as_str())
                        .map_err(|e| anyhow::anyhow!("Invalid header name '{}': {}", key, e))?;
                    let header_value = HeaderValue::from_str(value.as_str().unwrap_or(""))
                        .map_err(|e| anyhow::anyhow!("Invalid header value for '{}': {}", key, e))?;
                    header_map.insert(header_name, header_value);
                }
            }
        }

        Ok(header_map)
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "webfetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL (HTTP/HTTPS GET request)"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (must be http:// or https://)"
                },
                "headers": {
                    "type": "object",
                    "description": "Optional HTTP headers to include in the request (JSON object)"
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Request timeout in seconds (default: 30)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        // Validate URL scheme
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ToolResult {
                content: format!("Invalid URL: must start with http:// or https://"),
                is_error: true,
            });
        }

        let timeout_seconds = params["timeout_seconds"].as_u64().unwrap_or(30);
        let headers = self.parse_headers(params.get("headers"))?;

        // Build request
        let mut request = self.client.get(url);

        // Add custom headers
        if !headers.is_empty() {
            request = request.headers(headers);
        }

        // Set timeout
        request = request.timeout(Duration::from_secs(timeout_seconds));

        // Execute request
        let response = match request.send().await {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = if e.is_timeout() {
                    format!("Request timed out after {} seconds", timeout_seconds)
                } else if e.is_connect() {
                    format!("Connection failed: {}", e)
                } else if e.is_request() {
                    format!("Invalid request: {}", e)
                } else {
                    format!("Network error: {}", e)
                };
                return Ok(ToolResult {
                    content: error_msg,
                    is_error: true,
                });
            }
        };

        // Check HTTP status
        let status = response.status();
        if !status.is_success() {
            return Ok(ToolResult {
                content: format!("HTTP error {}: {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown error")),
                is_error: true,
            });
        }

        // Get response body
        let body = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                return Ok(ToolResult {
                    content: format!("Failed to read response body: {}", e),
                    is_error: true,
                });
            }
        };

        Ok(ToolResult {
            content: body,
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webfetch_invalid_scheme() {
        let tool = WebFetchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "url": "ftp://example.com"
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("Invalid URL"));
    }

    #[tokio::test]
    async fn test_webfetch_missing_url() {
        let tool = WebFetchTool::new();
        let result = tool.execute(serde_json::json!({})).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_webfetch_with_custom_headers() {
        // This test requires a mock server
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .match_header("X-Custom-Header", "test-value")
            .with_status(200)
            .with_body("success")
            .create_async()
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "url": format!("{}/test", server.url()),
                "headers": {
                    "X-Custom-Header": "test-value"
                }
            }))
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(!result.is_error);
        assert_eq!(result.content, "success");
    }

    #[tokio::test]
    async fn test_webfetch_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_body("Hello, World!")
            .create_async()
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "url": format!("{}/test", server.url())
            }))
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(!result.is_error);
        assert_eq!(result.content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_webfetch_http_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/notfound")
            .with_status(404)
            .create_async()
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "url": format!("{}/notfound", server.url())
            }))
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(result.is_error);
        assert!(result.content.contains("404"));
    }

    #[tokio::test]
    async fn test_webfetch_timeout() {
        // This test is complex with mockito - we'll test timeout behavior differently
        // by using a real non-responsive endpoint. We verify the tool handles timeout correctly.
        let tool = WebFetchTool::new();

        // Use a blackhole IP that won't respond (RFC 5737 TEST-NET-1)
        let result = tool
            .execute(serde_json::json!({
                "url": "http://192.0.2.1:9999/test",
                "timeout_seconds": 1
            }))
            .await
            .unwrap();

        // The exact error depends on OS/network, but should be an error
        assert!(result.is_error);
        // Could be timeout, connection failure, or network error
        assert!(
            result.content.contains("timed out")
            || result.content.contains("Connection failed")
            || result.content.contains("Network error")
        );
    }

    #[tokio::test]
    async fn test_webfetch_connection_error() {
        let tool = WebFetchTool::new();
        // Use a non-routable IP address to trigger connection error
        let result = tool
            .execute(serde_json::json!({
                "url": "http://192.0.2.1:9999/test",
                "timeout_seconds": 2
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        // Should contain either "timed out" or "Connection failed" or "Network error"
        assert!(
            result.content.contains("timed out")
                || result.content.contains("Connection failed")
                || result.content.contains("Network error")
        );
    }

    #[tokio::test]
    async fn test_webfetch_with_custom_timeout() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_body("response")
            .create_async()
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "url": format!("{}/test", server.url()),
                "timeout_seconds": 60
            }))
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(!result.is_error);
        assert_eq!(result.content, "response");
    }
}
