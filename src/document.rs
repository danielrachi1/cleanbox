use crate::error::{CleanboxError, Result};
use crate::filesystem::FileManager;
use regex::Regex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref DATE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(\d{4})[-_]?(\d{2})[-_]?(\d{2})").unwrap(), // YYYYMMDD, YYYY-MM-DD, YYYY_MM_DD
        Regex::new(r"(\d{4})(\d{2})(\d{2})").unwrap(),           // Pure YYYYMMDD
    ];
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentInput {
    pub date: String,        // YYYY-MM-DD format
    pub description: String, // kebab-case format
    pub tags: Vec<String>,   // validated tags from user input
}

impl DocumentInput {
    pub fn new(date: String, description: String, tags: Vec<String>) -> Self {
        Self {
            date,
            description,
            tags,
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_date()?;
        self.validate_description()?;
        self.validate_tags()?;
        Ok(())
    }

    pub fn validate_date(&self) -> Result<()> {
        // Check if date matches YYYY-MM-DD format
        if self.date.len() != 10 {
            return Err(CleanboxError::InvalidUserInput(format!(
                "Date must be in YYYY-MM-DD format, got: {}",
                self.date
            )));
        }

        let parts: Vec<&str> = self.date.split('-').collect();
        if parts.len() != 3 {
            return Err(CleanboxError::InvalidUserInput(format!(
                "Date must be in YYYY-MM-DD format, got: {}",
                self.date
            )));
        }

        // Validate year (4 digits)
        let year = parts[0]
            .parse::<u32>()
            .map_err(|_| CleanboxError::InvalidUserInput(format!("Invalid year: {}", parts[0])))?;
        if !(1900..=2100).contains(&year) {
            return Err(CleanboxError::InvalidUserInput(format!(
                "Year must be between 1900-2100, got: {year}"
            )));
        }

        // Validate month (01-12)
        let month = parts[1]
            .parse::<u32>()
            .map_err(|_| CleanboxError::InvalidUserInput(format!("Invalid month: {}", parts[1])))?;
        if !(1..=12).contains(&month) {
            return Err(CleanboxError::InvalidUserInput(format!(
                "Month must be between 01-12, got: {month:02}"
            )));
        }

        // Validate day (01-31, basic validation)
        let day = parts[2]
            .parse::<u32>()
            .map_err(|_| CleanboxError::InvalidUserInput(format!("Invalid day: {}", parts[2])))?;
        if !(1..=31).contains(&day) {
            return Err(CleanboxError::InvalidUserInput(format!(
                "Day must be between 01-31, got: {day:02}"
            )));
        }

        Ok(())
    }

    pub fn validate_description(&self) -> Result<()> {
        if self.description.is_empty() {
            return Err(CleanboxError::InvalidUserInput(
                "Description cannot be empty".to_string(),
            ));
        }

        // Check kebab-case format: lowercase letters, numbers, and hyphens only
        if !self
            .description
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(CleanboxError::InvalidUserInput(format!(
                "Description must be in kebab-case (lowercase, numbers, hyphens only): {}",
                self.description
            )));
        }

        // Must not start or end with hyphen
        if self.description.starts_with('-') || self.description.ends_with('-') {
            return Err(CleanboxError::InvalidUserInput(format!(
                "Description cannot start or end with hyphen: {}",
                self.description
            )));
        }

        // Must not have consecutive hyphens
        if self.description.contains("--") {
            return Err(CleanboxError::InvalidUserInput(format!(
                "Description cannot contain consecutive hyphens: {}",
                self.description
            )));
        }

        Ok(())
    }

    fn validate_tags(&self) -> Result<()> {
        if self.tags.is_empty() {
            return Err(CleanboxError::InvalidUserInput(
                "At least one tag is required".to_string(),
            ));
        }

        for tag in &self.tags {
            if tag.is_empty() {
                return Err(CleanboxError::InvalidUserInput(
                    "Tag cannot be empty".to_string(),
                ));
            }

            // Check kebab-case format for tags too
            if !tag
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return Err(CleanboxError::InvalidUserInput(format!(
                    "Tag must be in kebab-case (lowercase, numbers, hyphens only): {tag}"
                )));
            }

            // Must not start or end with hyphen
            if tag.starts_with('-') || tag.ends_with('-') {
                return Err(CleanboxError::InvalidUserInput(format!(
                    "Tag cannot start or end with hyphen: {tag}"
                )));
            }

            // Must not have consecutive hyphens
            if tag.contains("--") {
                return Err(CleanboxError::InvalidUserInput(format!(
                    "Tag cannot contain consecutive hyphens: {tag}"
                )));
            }
        }

        Ok(())
    }

    // Convert to filename format: YYYY-MM-DD_description@@tag1,tag2
    pub fn to_filename_stem(&self) -> String {
        format!("{}_{}", self.date, self.format_tags())
    }

    fn format_tags(&self) -> String {
        format!("{}@@{}", self.description, self.tags.join(","))
    }
}

/// Returns today's date in YYYY-MM-DD format.
///
/// Uses proper date/time handling with leap year and month length support.
///
/// # Returns
/// * `String` - Today's date in YYYY-MM-DD format
pub fn today_date_string() -> String {
    use chrono::Utc;

    Utc::now().format("%Y-%m-%d").to_string()
}

/// Extracts date from filename in YYYY-MM-DD format.
///
/// Searches for date patterns in the filename and returns the first valid date found.
/// Supports YYYYMMDD, YYYY-MM-DD, and YYYY_MM_DD formats.
///
/// # Arguments
/// * `filename` - Path to extract date from (only filename portion is used)
///
/// # Returns
/// * `Some(String)` - Date in YYYY-MM-DD format if found and valid
/// * `None` - If no valid date pattern is found
///
/// # Examples
/// ```
/// # use cleanbox::document::extract_date_from_filename;
/// assert_eq!(extract_date_from_filename("report_20240315.pdf"), Some("2024-03-15".to_string()));
/// assert_eq!(extract_date_from_filename("2024-03-15_meeting.docx"), Some("2024-03-15".to_string()));
/// assert_eq!(extract_date_from_filename("no_date.txt"), None);
/// ```
pub fn extract_date_from_filename<P: AsRef<Path>>(filename: P) -> Option<String> {
    let filename_str = filename.as_ref().file_name()?.to_str()?;

    for regex in DATE_PATTERNS.iter() {
        if let Some(captures) = regex.captures(filename_str) {
            if captures.len() >= 4 {
                let year = captures.get(1)?.as_str().parse::<u32>().ok()?;
                let month = captures.get(2)?.as_str().parse::<u32>().ok()?;
                let day = captures.get(3)?.as_str().parse::<u32>().ok()?;

                // Basic validation
                if (1900..=2100).contains(&year)
                    && (1..=12).contains(&month)
                    && (1..=31).contains(&day)
                {
                    return Some(format!("{year:04}-{month:02}-{day:02}"));
                }
            }
        }
    }

    None
}

/// Converts a SystemTime to YYYY-MM-DD format string.
///
/// Uses proper date/time handling with leap year and month length support.
///
/// # Arguments
/// * `system_time` - The SystemTime to convert
///
/// # Returns
/// * `Some(String)` - Date in YYYY-MM-DD format
/// * `None` - If conversion fails
pub fn format_system_time_to_date(system_time: std::time::SystemTime) -> Option<String> {
    use chrono::{DateTime, Utc};

    let datetime: DateTime<Utc> = system_time.into();
    Some(datetime.format("%Y-%m-%d").to_string())
}

/// Suggests a document date using intelligent fallback chain.
///
/// Attempts to determine the most appropriate date for a document by trying multiple sources
/// in priority order:
/// 1. Extract date from filename patterns (YYYYMMDD, YYYY-MM-DD, YYYY_MM_DD)
/// 2. Use file's last modified time from filesystem metadata
/// 3. Fall back to current date as final default
///
/// # Arguments
/// * `filename` - Path to the document file
/// * `file_manager` - File manager implementation for accessing filesystem metadata
///
/// # Returns
/// * `String` - Date in YYYY-MM-DD format (always returns a valid date)
///
/// # Examples
/// ```
/// # use cleanbox::document::suggest_document_date;
/// # use cleanbox::filesystem::StdFileManager;
/// let file_manager = StdFileManager::new();
/// let date = suggest_document_date("report_20240315.pdf", &file_manager);
/// assert_eq!(date, "2024-03-15");
/// ```
pub fn suggest_document_date<P: AsRef<Path>, F: FileManager>(
    filename: P,
    file_manager: &F,
) -> String {
    // Priority 1: Try to extract date from filename
    if let Some(date_from_filename) = extract_date_from_filename(&filename) {
        return date_from_filename;
    }

    // Priority 2: Try to get filesystem modified time
    if let Ok(modified_time) = file_manager.get_file_modified_time(&filename) {
        if let Some(date_from_filesystem) = format_system_time_to_date(modified_time) {
            return date_from_filesystem;
        }
    }

    // Priority 3: Fall back to current date
    today_date_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_input_creation() {
        let input = DocumentInput::new(
            "2025-01-15".to_string(),
            "quarterly-report".to_string(),
            vec!["finance".to_string(), "quarterly".to_string()],
        );

        assert_eq!(input.date, "2025-01-15");
        assert_eq!(input.description, "quarterly-report");
        assert_eq!(input.tags, vec!["finance", "quarterly"]);
    }

    #[test]
    fn test_valid_date_validation() {
        let input = DocumentInput::new(
            "2025-01-15".to_string(),
            "test".to_string(),
            vec!["tag".to_string()],
        );
        assert!(input.validate_date().is_ok());
    }

    #[test]
    fn test_invalid_date_formats() {
        let test_cases = vec![
            ("2025-1-15", "Month must be 2 digits"),
            ("25-01-15", "Year must be 4 digits"),
            ("2025/01/15", "Must use hyphens"),
            ("2025-13-15", "Invalid month"),
            ("2025-01-32", "Invalid day"),
            ("1899-01-15", "Year too early"),
            ("2101-01-15", "Year too late"),
        ];

        for (invalid_date, _reason) in test_cases {
            let input = DocumentInput::new(
                invalid_date.to_string(),
                "test".to_string(),
                vec!["tag".to_string()],
            );
            assert!(
                input.validate_date().is_err(),
                "Should reject: {}",
                invalid_date
            );
        }
    }

    #[test]
    fn test_valid_description_validation() {
        let valid_descriptions = vec![
            "quarterly-report",
            "test-document",
            "simple",
            "document-with-numbers-123",
            "a",
        ];

        for desc in valid_descriptions {
            let input = DocumentInput::new(
                "2025-01-15".to_string(),
                desc.to_string(),
                vec!["tag".to_string()],
            );
            assert!(
                input.validate_description().is_ok(),
                "Should accept: {}",
                desc
            );
        }
    }

    #[test]
    fn test_invalid_description_formats() {
        let invalid_descriptions = vec![
            ("", "Empty description"),
            ("Capitalized", "Contains uppercase"),
            ("has spaces", "Contains spaces"),
            ("has_underscores", "Contains underscores"),
            ("-starts-with-hyphen", "Starts with hyphen"),
            ("ends-with-hyphen-", "Ends with hyphen"),
            ("double--hyphen", "Consecutive hyphens"),
            ("special@chars", "Special characters"),
        ];

        for (invalid_desc, _reason) in invalid_descriptions {
            let input = DocumentInput::new(
                "2025-01-15".to_string(),
                invalid_desc.to_string(),
                vec!["tag".to_string()],
            );
            assert!(
                input.validate_description().is_err(),
                "Should reject: {}",
                invalid_desc
            );
        }
    }

    #[test]
    fn test_valid_tags_validation() {
        let input = DocumentInput::new(
            "2025-01-15".to_string(),
            "test".to_string(),
            vec![
                "finance".to_string(),
                "quarterly".to_string(),
                "report-2025".to_string(),
            ],
        );
        assert!(input.validate_tags().is_ok());
    }

    #[test]
    fn test_invalid_tags() {
        let test_cases = vec![
            (vec![], "Empty tags list"),
            (vec!["".to_string()], "Empty tag"),
            (vec!["Capital".to_string()], "Uppercase in tag"),
            (vec!["has spaces".to_string()], "Spaces in tag"),
            (vec!["-starts-hyphen".to_string()], "Starts with hyphen"),
            (vec!["ends-hyphen-".to_string()], "Ends with hyphen"),
            (vec!["double--hyphen".to_string()], "Consecutive hyphens"),
        ];

        for (invalid_tags, _reason) in test_cases {
            let input =
                DocumentInput::new("2025-01-15".to_string(), "test".to_string(), invalid_tags);
            assert!(input.validate_tags().is_err());
        }
    }

    #[test]
    fn test_complete_validation() {
        let valid_input = DocumentInput::new(
            "2025-01-15".to_string(),
            "quarterly-report".to_string(),
            vec!["finance".to_string(), "quarterly".to_string()],
        );
        assert!(valid_input.validate().is_ok());

        let invalid_input = DocumentInput::new(
            "invalid-date".to_string(),
            "Invalid Description".to_string(),
            vec!["Bad Tag".to_string()],
        );
        assert!(invalid_input.validate().is_err());
    }

    #[test]
    fn test_filename_generation() {
        let input = DocumentInput::new(
            "2025-01-15".to_string(),
            "quarterly-report".to_string(),
            vec!["finance".to_string(), "reports".to_string()],
        );

        assert_eq!(
            input.to_filename_stem(),
            "2025-01-15_quarterly-report@@finance,reports"
        );
    }

    #[test]
    fn test_today_date_string() {
        let today = today_date_string();
        // Should be in YYYY-MM-DD format
        assert_eq!(today.len(), 10);
        assert_eq!(today.chars().nth(4).unwrap(), '-');
        assert_eq!(today.chars().nth(7).unwrap(), '-');

        // Should be parseable as a valid DocumentInput date
        let input = DocumentInput::new(today, "test".to_string(), vec!["tag".to_string()]);
        assert!(input.validate_date().is_ok());
    }

    #[test]
    fn test_extract_date_from_filename_yyyymmdd() {
        // Test pure YYYYMMDD format
        assert_eq!(
            extract_date_from_filename("20250731_quarterly_report.pdf"),
            Some("2025-07-31".to_string())
        );

        // Test YYYYMMDD at start
        assert_eq!(
            extract_date_from_filename("20251225_christmas_plan.docx"),
            Some("2025-12-25".to_string())
        );

        // Test YYYYMMDD in middle
        assert_eq!(
            extract_date_from_filename("report_20250101_final.pdf"),
            Some("2025-01-01".to_string())
        );
    }

    #[test]
    fn test_extract_date_from_filename_formatted() {
        // Test YYYY-MM-DD format
        assert_eq!(
            extract_date_from_filename("2025-07-31_quarterly_report.pdf"),
            Some("2025-07-31".to_string())
        );

        // Test YYYY_MM_DD format
        assert_eq!(
            extract_date_from_filename("2025_12_25_christmas_plan.docx"),
            Some("2025-12-25".to_string())
        );

        // Test mixed separators
        assert_eq!(
            extract_date_from_filename("invoice-2025-01-15.pdf"),
            Some("2025-01-15".to_string())
        );
    }

    #[test]
    fn test_extract_date_from_filename_invalid() {
        // Test invalid year
        assert_eq!(
            extract_date_from_filename("1899-01-01_old_document.pdf"),
            None
        );

        // Test invalid month
        assert_eq!(
            extract_date_from_filename("2025-13-01_invalid_month.pdf"),
            None
        );

        // Test invalid day
        assert_eq!(
            extract_date_from_filename("2025-01-32_invalid_day.pdf"),
            None
        );

        // Test no date pattern
        assert_eq!(extract_date_from_filename("some_document.pdf"), None);

        // Test malformed date
        assert_eq!(
            extract_date_from_filename("202507_incomplete_date.pdf"),
            None
        );
    }

    #[test]
    fn test_extract_date_from_filename_edge_cases() {
        // Test multiple dates (should match first valid one)
        assert_eq!(
            extract_date_from_filename("20250101_report_20251231.pdf"),
            Some("2025-01-01".to_string())
        );

        // Test date with extensions
        assert_eq!(
            extract_date_from_filename("20250731.backup.pdf"),
            Some("2025-07-31".to_string())
        );

        // Test only filename (no path)
        use std::path::Path;
        assert_eq!(
            extract_date_from_filename(Path::new("/long/path/to/20250731_file.pdf")),
            Some("2025-07-31".to_string())
        );
    }

    #[test]
    fn test_format_system_time_to_date() {
        // Test a known timestamp for July 31, 2025
        use chrono::{TimeZone, Utc};
        let test_date = Utc.with_ymd_and_hms(2025, 7, 31, 12, 0, 0).unwrap();
        let test_time = test_date.into();

        let result = format_system_time_to_date(test_time);
        assert!(result.is_some());

        let date_str = result.unwrap();
        assert_eq!(date_str, "2025-07-31");
        assert_eq!(date_str.len(), 10); // YYYY-MM-DD format

        // Test current time
        let now = std::time::SystemTime::now();
        let result = format_system_time_to_date(now);
        assert!(result.is_some());
    }

    #[test]
    fn test_suggest_document_date_filename_priority() {
        use crate::filesystem::MockFileManager;
        use std::time::{Duration, UNIX_EPOCH};

        let mut file_manager = MockFileManager::new();
        let old_time = UNIX_EPOCH + Duration::from_secs(1000000); // Some old timestamp

        // Add file with modified time that differs from filename date
        file_manager.add_file_with_modified_time(
            std::path::PathBuf::from("20250731_report.pdf"),
            vec![1, 2, 3],
            old_time,
        );

        // Should prioritize filename date over filesystem date
        let result = suggest_document_date("20250731_report.pdf", &file_manager);
        assert_eq!(result, "2025-07-31");
    }

    #[test]
    fn test_suggest_document_date_filesystem_fallback() {
        use crate::filesystem::MockFileManager;

        let mut file_manager = MockFileManager::new();
        // Use chrono to calculate a proper timestamp for June 15, 2024
        use chrono::{TimeZone, Utc};
        let test_date = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        let known_time = test_date.into();

        // Add file without date in filename but with known modified time
        file_manager.add_file_with_modified_time(
            std::path::PathBuf::from("report_without_date.pdf"),
            vec![1, 2, 3],
            known_time,
        );

        // Should use filesystem modified time
        let result = suggest_document_date("report_without_date.pdf", &file_manager);
        assert_eq!(result, "2024-06-15");
        assert_eq!(result.len(), 10); // YYYY-MM-DD format
    }

    #[test]
    fn test_suggest_document_date_today_fallback() {
        use crate::filesystem::MockFileManager;

        let file_manager = MockFileManager::new(); // Empty manager

        // File doesn't exist, should fall back to today
        let result = suggest_document_date("nonexistent_file.pdf", &file_manager);
        let today = today_date_string();
        assert_eq!(result, today);
    }

    #[test]
    fn test_suggest_document_date_comprehensive_fallback() {
        use crate::filesystem::MockFileManager;
        use std::time::{Duration, UNIX_EPOCH};

        let mut file_manager = MockFileManager::new();

        // Test 1: Filename date exists -> use it
        file_manager.add_file(std::path::PathBuf::from("20250101_test.pdf"), vec![1, 2, 3]);
        let result = suggest_document_date("20250101_test.pdf", &file_manager);
        assert_eq!(result, "2025-01-01");

        // Test 2: No filename date, but filesystem time exists -> use filesystem
        let fs_time = UNIX_EPOCH + Duration::from_secs(1600000000); // Known timestamp
        file_manager.add_file_with_modified_time(
            std::path::PathBuf::from("no_date_file.pdf"),
            vec![4, 5, 6],
            fs_time,
        );
        let result = suggest_document_date("no_date_file.pdf", &file_manager);
        // Should be a valid date format from filesystem time
        assert_eq!(result.len(), 10);
        assert!(result.contains("-"));

        // Test 3: Neither filename nor filesystem -> use today
        let result = suggest_document_date("completely_missing.pdf", &file_manager);
        let today = today_date_string();
        assert_eq!(result, today);
    }
}
