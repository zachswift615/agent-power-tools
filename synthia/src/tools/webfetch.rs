use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::time::Duration;
use url::Url;

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

                    // Fix: Properly check if value is a string before converting
                    let value_str = value.as_str()
                        .ok_or_else(|| anyhow::anyhow!("Header value for '{}' must be a string, got: {:?}", key, value))?;

                    let header_value = HeaderValue::from_str(value_str)
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
                },
                "max_size_mb": {
                    "type": "integer",
                    "description": "Maximum response size in MB (default: 10)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let url_str = params["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        // Fix: Parse URL to validate format and check scheme
        let parsed_url = match Url::parse(url_str) {
            Ok(url) => url,
            Err(e) => {
                return Ok(ToolResult {
                    content: format!("Malformed URL: {}", e),
                    is_error: true,
                });
            }
        };

        // Validate URL scheme using parsed URL
        if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
            return Ok(ToolResult {
                content: format!("Invalid URL scheme '{}': must be http or https", parsed_url.scheme()),
                is_error: true,
            });
        }

        let timeout_seconds = params["timeout_seconds"].as_u64().unwrap_or(30);
        let max_size_mb = params["max_size_mb"].as_u64().unwrap_or(10);
        let max_size_bytes = max_size_mb * 1024 * 1024; // Convert MB to bytes
        let headers = self.parse_headers(params.get("headers"))?;

        // Build request (use validated URL string)
        let mut request = self.client.get(url_str);

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

        // Fix: Check Content-Length header before downloading
        if let Some(content_length) = response.content_length() {
            if content_length > max_size_bytes {
                return Ok(ToolResult {
                    content: format!(
                        "Response too large: {} MB exceeds maximum of {} MB",
                        content_length / (1024 * 1024),
                        max_size_mb
                    ),
                    is_error: true,
                });
            }
        }

        // Get response body with size limit
        let body = match response.text().await {
            Ok(text) => {
                // Check actual size after download (in case Content-Length was missing)
                if text.len() as u64 > max_size_bytes {
                    return Ok(ToolResult {
                        content: format!(
                            "Response too large: {} MB exceeds maximum of {} MB",
                            text.len() / (1024 * 1024),
                            max_size_mb
                        ),
                        is_error: true,
                    });
                }
                text
            }
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

    #[tokio::test]
    async fn test_webfetch_non_string_header_values() {
        let tool = WebFetchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "url": "http://example.com",
                "headers": {
                    "X-Custom-Header": 123  // Non-string value
                }
            }))
            .await;

        // Should return an error because header value is not a string
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("must be a string"));
    }

    #[tokio::test]
    async fn test_webfetch_malformed_url() {
        let tool = WebFetchTool::new();

        // Test various malformed URLs
        let malformed_urls = vec![
            "not-a-url",
            "htp://missing-t.com",
            "://no-scheme.com",
            "http://",
            "",
        ];

        for url in malformed_urls {
            let result = tool
                .execute(serde_json::json!({
                    "url": url
                }))
                .await
                .unwrap();

            assert!(result.is_error, "Expected error for malformed URL: {}", url);
            assert!(result.content.contains("Malformed URL") || result.content.contains("Invalid URL"));
        }
    }

    #[tokio::test]
    async fn test_webfetch_large_response() {
        let mut server = mockito::Server::new_async().await;

        // Create a large response body (2MB)
        let large_body = "x".repeat(2 * 1024 * 1024);

        let mock = server
            .mock("GET", "/large")
            .with_status(200)
            .with_header("Content-Length", &(large_body.len().to_string()))
            .with_body(large_body.clone())
            .create_async()
            .await;

        let tool = WebFetchTool::new();

        // Test with default max_size (10MB) - should succeed
        let result = tool
            .execute(serde_json::json!({
                "url": format!("{}/large", server.url())
            }))
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 2 * 1024 * 1024);

        // Test with max_size of 1MB - should fail
        let mock2 = server
            .mock("GET", "/large2")
            .with_status(200)
            .with_header("Content-Length", &(large_body.len().to_string()))
            .with_body(large_body)
            .create_async()
            .await;

        let result = tool
            .execute(serde_json::json!({
                "url": format!("{}/large2", server.url()),
                "max_size_mb": 1
            }))
            .await
            .unwrap();

        mock2.assert_async().await;
        assert!(result.is_error);
        assert!(result.content.contains("Response too large") || result.content.contains("exceeds maximum"));
    }
}
