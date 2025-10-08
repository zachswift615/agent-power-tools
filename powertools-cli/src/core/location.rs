use anyhow::{anyhow, Result};
use std::path::PathBuf;
use crate::core::types::Location;

/// Parse location strings in the format "file:line:column" or "file:line"
pub fn parse_location(location_str: &str) -> Result<Location> {
    let parts: Vec<&str> = location_str.split(':').collect();

    match parts.len() {
        2 => {
            // file:line format
            let file_path = PathBuf::from(parts[0]);
            let line = parts[1].parse::<usize>()
                .map_err(|_| anyhow!("Invalid line number: {}", parts[1]))?;

            Ok(Location {
                file_path,
                line,
                column: 1, // Default to column 1
                end_line: None,
                end_column: None,
            })
        }
        3 => {
            // file:line:column format
            let file_path = PathBuf::from(parts[0]);
            let line = parts[1].parse::<usize>()
                .map_err(|_| anyhow!("Invalid line number: {}", parts[1]))?;
            let column = parts[2].parse::<usize>()
                .map_err(|_| anyhow!("Invalid column number: {}", parts[2]))?;

            Ok(Location {
                file_path,
                line,
                column,
                end_line: None,
                end_column: None,
            })
        }
        _ => {
            Err(anyhow!(
                "Invalid location format: '{}'. Expected 'file:line' or 'file:line:column'",
                location_str
            ))
        }
    }
}

/// Format a location for display
#[allow(dead_code)]
pub fn format_location(location: &Location) -> String {
    format!(
        "{}:{}:{}",
        location.file_path.display(),
        location.line,
        location.column
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_location_with_column() {
        let result = parse_location("src/main.rs:10:5").unwrap();
        assert_eq!(result.file_path, PathBuf::from("src/main.rs"));
        assert_eq!(result.line, 10);
        assert_eq!(result.column, 5);
    }

    #[test]
    fn test_parse_location_without_column() {
        let result = parse_location("src/main.rs:10").unwrap();
        assert_eq!(result.file_path, PathBuf::from("src/main.rs"));
        assert_eq!(result.line, 10);
        assert_eq!(result.column, 1);
    }

    #[test]
    fn test_parse_location_invalid() {
        assert!(parse_location("src/main.rs").is_err());
        assert!(parse_location("src/main.rs:abc:5").is_err());
        assert!(parse_location("src/main.rs:10:xyz").is_err());
    }
}