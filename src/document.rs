use crate::error::{CleanboxError, Result};

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

// Helper to parse today's date in YYYY-MM-DD format
pub fn today_date_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    // Simple date calculation (days since epoch)
    let days_since_epoch = now.as_secs() / (24 * 60 * 60);
    let days_since_1970 = days_since_epoch as i32;

    // Approximate date calculation (good enough for default values)
    let year = 1970 + (days_since_1970 / 365);
    let day_of_year = days_since_1970 % 365;
    let month = (day_of_year / 30) + 1;
    let day_of_month = (day_of_year % 30) + 1;

    format!(
        "{:04}-{:02}-{:02}",
        year,
        month.min(12),
        day_of_month.min(31)
    )
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
}
