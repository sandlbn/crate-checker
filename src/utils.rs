//! Utility functions for the crate checker application

use crate::error::{CrateCheckerError, Result};
use crate::types::BatchInput;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tracing::{debug, error, info};

/// Parse JSON input for batch operations
pub fn parse_json_input(json: &str) -> Result<BatchInput> {
    debug!("Parsing JSON input: {}", json);

    // First, parse as generic JSON to inspect structure
    let value: Value = serde_json::from_str(json).map_err(|e| {
        error!("Failed to parse JSON: {}", e);
        CrateCheckerError::InvalidBatchInput(format!("Invalid JSON: {}", e))
    })?;

    // Try to deserialize as BatchInput
    match serde_json::from_value::<BatchInput>(value.clone()) {
        Ok(batch_input) => {
            info!("Successfully parsed batch input");
            Ok(batch_input)
        }
        Err(e) => {
            error!("Failed to deserialize batch input: {}", e);
            // Provide helpful error message based on the JSON structure
            let error_msg = match &value {
                Value::Object(obj) => {
                    if obj.contains_key("operations") {
                        "Invalid operations format. Expected array of operation objects."
                    } else if obj.contains_key("crates") {
                        "Invalid crates list format. Expected array of strings."
                    } else if obj
                        .keys()
                        .all(|k| !["operations", "crates"].contains(&k.as_str()))
                    {
                        "Looks like a crate-version map, but some values may be invalid."
                    } else {
                        "Unknown JSON structure for batch input."
                    }
                }
                _ => "Expected JSON object for batch input.",
            };

            Err(CrateCheckerError::InvalidBatchInput(format!(
                "{} Original error: {}",
                error_msg, e
            )))
        }
    }
}

/// Parse JSON input from a file
pub fn parse_json_file<P: AsRef<Path>>(path: P) -> Result<BatchInput> {
    let path = path.as_ref();
    info!("Reading JSON file: {}", path.display());

    let content = fs::read_to_string(path).map_err(|e| {
        error!("Failed to read file {}: {}", path.display(), e);
        CrateCheckerError::IoError(e)
    })?;

    parse_json_input(&content)
}

/// Validate a batch input structure
pub fn validate_batch_input(input: &BatchInput) -> Result<()> {
    match input {
        BatchInput::CrateVersionMap(map) => {
            if map.is_empty() {
                return Err(CrateCheckerError::ValidationError(
                    "Crate version map cannot be empty".to_string(),
                ));
            }

            for (crate_name, version) in map {
                if crate_name.is_empty() {
                    return Err(CrateCheckerError::ValidationError(
                        "Crate name cannot be empty".to_string(),
                    ));
                }
                if version.is_empty() {
                    return Err(CrateCheckerError::ValidationError(format!(
                        "Version for crate '{}' cannot be empty",
                        crate_name
                    )));
                }
            }
        }
        BatchInput::CrateList { crates } => {
            if crates.is_empty() {
                return Err(CrateCheckerError::ValidationError(
                    "Crates list cannot be empty".to_string(),
                ));
            }

            for crate_name in crates {
                if crate_name.is_empty() {
                    return Err(CrateCheckerError::ValidationError(
                        "Crate name cannot be empty".to_string(),
                    ));
                }
            }
        }
        BatchInput::Operations { operations } => {
            if operations.is_empty() {
                return Err(CrateCheckerError::ValidationError(
                    "Operations list cannot be empty".to_string(),
                ));
            }

            for operation in operations {
                if operation.operation.is_empty() {
                    return Err(CrateCheckerError::ValidationError(
                        "Operation type cannot be empty".to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Format duration in human-readable form
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if total_secs == 0 {
        format!("{}ms", millis)
    } else if total_secs < 60 {
        format!("{}.{}s", total_secs, millis / 100)
    } else {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}m {}s", mins, secs)
    }
}

/// Format file size in human-readable form
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Format download count in human-readable form
pub fn format_download_count(count: u64) -> String {
    if count < 1_000 {
        count.to_string()
    } else if count < 1_000_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else if count < 1_000_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else {
        format!("{:.1}B", count as f64 / 1_000_000_000.0)
    }
}

/// Sanitize crate name for safe usage
pub fn sanitize_crate_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// Check if a version string looks like a semver version
pub fn is_semver_like(version: &str) -> bool {
    // Basic check for semver-like pattern: X.Y.Z with optional pre-release/build
    let parts: Vec<&str> = version.split(&['.', '-', '+'][..]).collect();
    parts.len() >= 3 && parts.iter().take(3).all(|part| part.parse::<u32>().is_ok())
}

/// Extract the major.minor.patch part from a version string
pub fn extract_version_core(version: &str) -> Option<String> {
    let parts: Vec<&str> = version.split(&['-', '+'][..]).next()?.split('.').collect();
    if parts.len() >= 3 {
        Some(format!("{}.{}.{}", parts[0], parts[1], parts[2]))
    } else {
        None
    }
}

/// Create example batch inputs for help/documentation
pub fn create_example_batch_inputs() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Crate version map",
            r#"{"serde": "1.0.0", "tokio": "1.28.0", "reqwest": "latest"}"#,
        ),
        (
            "Crates list",
            r#"{"crates": ["serde", "tokio", "reqwest", "clap"]}"#,
        ),
        (
            "Advanced operations",
            r#"{
  "operations": [
    {"crate": "serde", "version": "1.0.0", "operation": "check_version"},
    {"crate": "tokio", "operation": "info"},
    {"crates": ["tokio", "reqwest"], "operation": "batch_check"}
  ]
}"#,
        ),
    ]
}

/// Truncate text to a maximum length with ellipsis
pub fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length.saturating_sub(3)])
    }
}

/// Create a progress indicator string
pub fn progress_indicator(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "".to_string();
    }

    let progress = (current as f64 / total as f64 * width as f64) as usize;
    let filled = "=".repeat(progress.min(width));
    let empty = " ".repeat(width.saturating_sub(progress));

    format!("[{}{}] {}/{}", filled, empty, current, total)
}

/// Parse a timeout string (e.g., "30s", "2m", "1h")
pub fn parse_timeout(input: &str) -> Result<std::time::Duration> {
    let input = input.trim().to_lowercase();

    if let Ok(secs) = input.parse::<u64>() {
        return Ok(std::time::Duration::from_secs(secs));
    }

    if input.ends_with('s') {
        let num_str = &input[..input.len() - 1];
        if let Ok(secs) = num_str.parse::<u64>() {
            return Ok(std::time::Duration::from_secs(secs));
        }
    } else if input.ends_with('m') {
        let num_str = &input[..input.len() - 1];
        if let Ok(mins) = num_str.parse::<u64>() {
            return Ok(std::time::Duration::from_secs(mins * 60));
        }
    } else if input.ends_with('h') {
        let num_str = &input[..input.len() - 1];
        if let Ok(hours) = num_str.parse::<u64>() {
            return Ok(std::time::Duration::from_secs(hours * 3600));
        }
    }

    Err(CrateCheckerError::ValidationError(format!(
        "Invalid timeout format: '{}'. Use formats like '30s', '5m', '1h'",
        input
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_input_crate_version_map() {
        let json = r#"{"serde": "1.0.0", "tokio": "latest"}"#;
        let result = parse_json_input(json).unwrap();

        match result {
            BatchInput::CrateVersionMap(map) => {
                assert_eq!(map.len(), 2);
                assert_eq!(map.get("serde"), Some(&"1.0.0".to_string()));
                assert_eq!(map.get("tokio"), Some(&"latest".to_string()));
            }
            _ => panic!("Expected CrateVersionMap"),
        }
    }

    #[test]
    fn test_parse_json_input_crates_list() {
        let json = r#"{"crates": ["serde", "tokio"]}"#;
        let result = parse_json_input(json).unwrap();

        match result {
            BatchInput::CrateList { crates } => {
                assert_eq!(crates, vec!["serde", "tokio"]);
            }
            _ => panic!("Expected CrateList"),
        }
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_format_download_count() {
        assert_eq!(format_download_count(500), "500");
        assert_eq!(format_download_count(1500), "1.5K");
        assert_eq!(format_download_count(1500000), "1.5M");
        assert_eq!(format_download_count(2500000000), "2.5B");
    }

    #[test]
    fn test_is_semver_like() {
        assert!(is_semver_like("1.0.0"));
        assert!(is_semver_like("2.1.3-beta"));
        assert!(is_semver_like("0.9.12+build.1"));
        assert!(!is_semver_like("invalid"));
        assert!(!is_semver_like("1.0"));
    }

    #[test]
    fn test_parse_timeout() {
        assert_eq!(
            parse_timeout("30").unwrap(),
            std::time::Duration::from_secs(30)
        );
        assert_eq!(
            parse_timeout("45s").unwrap(),
            std::time::Duration::from_secs(45)
        );
        assert_eq!(
            parse_timeout("2m").unwrap(),
            std::time::Duration::from_secs(120)
        );
        assert_eq!(
            parse_timeout("1h").unwrap(),
            std::time::Duration::from_secs(3600)
        );
        assert!(parse_timeout("invalid").is_err());
    }
}
