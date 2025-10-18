use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::Value;

pub struct JsonParser {
    code_block_regex: Regex,
}

impl JsonParser {
    pub fn new() -> Self {
        Self {
            code_block_regex: Regex::new(r"```(?:json)?\s*(.*?)\s*```")
                .expect("Failed to compile regex"),
        }
    }

    /// Parse JSON with multiple fallback strategies
    pub fn parse_robust(&self, raw: &str) -> Result<Value> {
        // Strategy 1: Direct parsing
        if let Ok(value) = serde_json::from_str::<Value>(raw) {
            tracing::debug!("JSON parsed successfully on first try");
            return Ok(value);
        }

        // Strategy 2: Extract from markdown code blocks
        if let Some(captures) = self.code_block_regex.captures(raw) {
            let json_str = &captures[1];
            if let Ok(value) = serde_json::from_str::<Value>(json_str) {
                tracing::debug!("JSON extracted from code block");
                return Ok(value);
            }
        }

        // Strategy 3: Fix common JSON errors
        let fixed = self.fix_common_errors(raw);
        if let Ok(value) = serde_json::from_str::<Value>(&fixed) {
            tracing::warn!("JSON required auto-fix. Original: {}", raw);
            tracing::warn!("Fixed version: {}", fixed);
            return Ok(value);
        }

        // All strategies failed
        Err(anyhow!(
            "Failed to parse JSON after all strategies.\nRaw input: {}\nFixed attempt: {}",
            raw,
            fixed
        ))
    }

    fn fix_common_errors(&self, json: &str) -> String {
        let mut fixed = json.to_string();

        // Single quotes to double quotes (but be careful not to break already-escaped quotes)
        fixed = fixed.replace("'", "\"");

        // Normalize whitespace (replace newlines with spaces, but preserve content)
        fixed = fixed.replace('\n', " ");
        fixed = fixed.replace('\r', " ");

        // Collapse multiple spaces into one
        while fixed.contains("  ") {
            fixed = fixed.replace("  ", " ");
        }

        // Trailing comma in object (after whitespace normalization)
        fixed = fixed.replace(",}", "}");
        fixed = fixed.replace(", }", "}");

        // Trailing comma in array (after whitespace normalization)
        fixed = fixed.replace(",]", "]");
        fixed = fixed.replace(", ]", "]");

        // Trim whitespace
        fixed = fixed.trim().to_string();

        fixed
    }
}

impl Default for JsonParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_json() {
        let parser = JsonParser::new();
        let result = parser.parse_robust(r#"{"key": "value"}"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["key"], "value");
    }

    #[test]
    fn test_parse_json_with_trailing_comma_object() {
        let parser = JsonParser::new();
        let result = parser.parse_robust(r#"{"key": "value",}"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["key"], "value");
    }

    #[test]
    fn test_parse_json_with_trailing_comma_array() {
        let parser = JsonParser::new();
        let result = parser.parse_robust(r#"{"items": [1, 2, 3,]}"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["items"][0], 1);
        assert_eq!(json["items"][2], 3);
    }

    #[test]
    fn test_parse_json_in_code_block() {
        let parser = JsonParser::new();
        let markdown = r#"
        Here's the JSON:
        ```json
        {"key": "value"}
        ```
        "#;
        let result = parser.parse_robust(markdown);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["key"], "value");
    }

    #[test]
    fn test_parse_json_in_code_block_no_language() {
        let parser = JsonParser::new();
        let markdown = r#"
        Here's the JSON:
        ```
        {"key": "value"}
        ```
        "#;
        let result = parser.parse_robust(markdown);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["key"], "value");
    }

    #[test]
    fn test_parse_json_with_single_quotes() {
        let parser = JsonParser::new();
        let result = parser.parse_robust(r#"{'key': 'value'}"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["key"], "value");
    }

    #[test]
    fn test_parse_json_with_newlines() {
        let parser = JsonParser::new();
        let json_with_newlines = r#"{
            "key": "value",
            "another": "field"
        }"#;
        let result = parser.parse_robust(json_with_newlines);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["key"], "value");
        assert_eq!(json["another"], "field");
    }

    #[test]
    fn test_parse_complex_malformed_json() {
        let parser = JsonParser::new();
        // Multiple issues: single quotes, trailing comma, newlines
        let malformed = r#"
        {
            'file_path': 'src/main.rs',
            'content': 'Hello world',
        }
        "#;
        let result = parser.parse_robust(malformed);
        if result.is_err() {
            eprintln!("Parse failed: {:?}", result);
        }
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["file_path"], "src/main.rs");
        assert_eq!(json["content"], "Hello world");
    }

    #[test]
    fn test_parse_completely_invalid_json() {
        let parser = JsonParser::new();
        let invalid = "This is not JSON at all!";
        let result = parser.parse_robust(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_object() {
        let parser = JsonParser::new();
        let result = parser.parse_robust("{}");
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.is_object());
        assert!(json.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_parse_nested_objects() {
        let parser = JsonParser::new();
        let nested = r#"{"outer": {"inner": "value",},}"#;
        let result = parser.parse_robust(nested);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["outer"]["inner"], "value");
    }

    #[test]
    fn test_parse_array_values() {
        let parser = JsonParser::new();
        let array = r#"{"items": ["a", "b", "c",]}"#;
        let result = parser.parse_robust(array);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["items"][0], "a");
        assert_eq!(json["items"][2], "c");
    }

    #[test]
    fn test_parse_number_values() {
        let parser = JsonParser::new();
        let numbers = r#"{"count": 42, "price": 19.99,}"#;
        let result = parser.parse_robust(numbers);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["count"], 42);
        assert_eq!(json["price"], 19.99);
    }

    #[test]
    fn test_parse_boolean_values() {
        let parser = JsonParser::new();
        let booleans = r#"{"active": true, "deleted": false,}"#;
        let result = parser.parse_robust(booleans);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["active"], true);
        assert_eq!(json["deleted"], false);
    }

    #[test]
    fn test_parse_null_value() {
        let parser = JsonParser::new();
        let with_null = r#"{"value": null,}"#;
        let result = parser.parse_robust(with_null);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json["value"].is_null());
    }
}
