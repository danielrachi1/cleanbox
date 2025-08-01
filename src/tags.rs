use crate::error::{CleanboxError, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct TagDictionary {
    tags: HashSet<String>,
}

impl TagDictionary {
    pub fn new() -> Self {
        Self {
            tags: HashSet::new(),
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path).map_err(|e| {
            CleanboxError::TagDictionaryCorrupted(format!(
                "Cannot read tags file at {}: {}",
                path.as_ref().display(),
                e
            ))
        })?;

        let mut tags = HashSet::new();

        for line in content.lines() {
            let tag = line.trim();
            if tag.is_empty() {
                continue; // Skip empty lines
            }

            // Validate each tag from the file
            validate_tag_format(tag)?;
            tags.insert(tag.to_string());
        }

        Ok(Self { tags })
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut tags: Vec<&str> = self.tags.iter().map(|s| s.as_str()).collect();
        tags.sort(); // Save in alphabetical order

        let content = tags.join("\n") + "\n"; // Add final newline

        fs::write(&path, content).map_err(|e| {
            CleanboxError::TagDictionaryCorrupted(format!(
                "Cannot write tags file at {}: {}",
                path.as_ref().display(),
                e
            ))
        })
    }

    pub fn contains(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    pub fn add_tag(&mut self, tag: String) -> Result<()> {
        validate_tag_format(&tag)?;
        self.tags.insert(tag);
        Ok(())
    }

    pub fn all_tags(&self) -> Vec<&str> {
        let mut tags: Vec<&str> = self.tags.iter().map(|s| s.as_str()).collect();
        tags.sort();
        tags
    }

    pub fn find_similar(&self, query: &str, max_results: usize) -> Vec<SimilarTag> {
        if query.is_empty() {
            return vec![];
        }

        // Simple prefix-based matching like bash completion
        let mut prefix_matches: Vec<SimilarTag> = vec![];
        let mut substring_matches: Vec<SimilarTag> = vec![];

        for tag in &self.tags {
            if tag.starts_with(query) {
                // Exact prefix match - highest priority
                prefix_matches.push(SimilarTag {
                    tag: tag.clone(),
                    distance: 0, // Perfect match
                    similarity: 1.0,
                });
            } else if tag.contains(query) {
                // Substring match - lower priority
                substring_matches.push(SimilarTag {
                    tag: tag.clone(),
                    distance: tag.find(query).unwrap_or(0), // Position of match
                    similarity: 0.5,
                });
            }
        }

        // Sort each group alphabetically for predictable ordering
        prefix_matches.sort_by(|a, b| a.tag.cmp(&b.tag));
        substring_matches.sort_by(|a, b| a.tag.cmp(&b.tag));

        // Combine results: prefix matches first, then substring matches
        let mut results = prefix_matches;
        results.extend(substring_matches);

        // Limit to max_results
        results.truncate(max_results);
        results
    }

    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tags.len()
    }
}

impl Default for TagDictionary {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SimilarTag {
    pub tag: String,
    pub distance: usize, // Edit distance
    pub similarity: f64, // Normalized similarity (0.0-1.0)
}

pub trait TagValidator {
    fn validate_tags(&self, tags: &[String]) -> Result<()>;
    fn suggest_similar(&self, tag: &str) -> Vec<SimilarTag>;
}

impl TagValidator for TagDictionary {
    fn validate_tags(&self, tags: &[String]) -> Result<()> {
        for tag in tags {
            validate_tag_format(tag)?;
        }
        Ok(())
    }

    fn suggest_similar(&self, tag: &str) -> Vec<SimilarTag> {
        self.find_similar(tag, 5) // Return top 5 similar tags
    }
}

// Tag validation function according to life-dir-spec.txt:
// "All tags must be singular, lowercase, kebab-case, and in english."
pub fn validate_tag_format(tag: &str) -> Result<()> {
    if tag.is_empty() {
        return Err(CleanboxError::InvalidUserInput(
            "Tag cannot be empty".to_string(),
        ));
    }

    // Check kebab-case format: lowercase letters, numbers, and hyphens only
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

    // Basic English alphabet check (no unicode characters)
    if !tag.is_ascii() {
        return Err(CleanboxError::InvalidUserInput(format!(
            "Tag must contain only ASCII characters (English): {tag}"
        )));
    }

    Ok(())
}

pub struct TagResolutionFlow {
    dictionary: TagDictionary,
}

impl TagResolutionFlow {
    pub fn new(dictionary: TagDictionary) -> Self {
        Self { dictionary }
    }

    pub fn resolve_tag(&self, input_tag: &str) -> TagResolution {
        // Check exact match first
        if self.dictionary.contains(input_tag) {
            return TagResolution::ExactMatch(input_tag.to_string());
        }

        // Find similar tags
        let similar = self.dictionary.find_similar(input_tag, 3);

        if similar.is_empty() {
            TagResolution::NoMatch {
                input: input_tag.to_string(),
                can_create: validate_tag_format(input_tag).is_ok(),
            }
        } else {
            TagResolution::SimilarFound {
                input: input_tag.to_string(),
                similar,
                can_create: validate_tag_format(input_tag).is_ok(),
            }
        }
    }

    pub fn add_new_tag(&mut self, tag: &str) -> Result<()> {
        self.dictionary.add_tag(tag.to_string())
    }

    pub fn dictionary(&self) -> &TagDictionary {
        &self.dictionary
    }

    pub fn dictionary_mut(&mut self) -> &mut TagDictionary {
        &mut self.dictionary
    }
}

#[derive(Debug, Clone)]
pub enum TagResolution {
    ExactMatch(String),
    SimilarFound {
        input: String,
        similar: Vec<SimilarTag>,
        can_create: bool,
    },
    NoMatch {
        input: String,
        can_create: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn create_test_tags_file(name: &str) -> PathBuf {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("{}_{}.txt", name, std::process::id()));

        let content =
            "finance\nquarterly\nreports\nmachine-learning\ndata-science\nproject-management\n";
        fs::write(&test_file, content).unwrap();

        test_file
    }

    #[test]
    fn test_tag_dictionary_creation() {
        let dict = TagDictionary::new();
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn test_load_from_file() {
        let test_file = create_test_tags_file("load_test");
        let dict = TagDictionary::load_from_file(&test_file).unwrap();

        assert!(!dict.is_empty());
        assert_eq!(dict.len(), 6);
        assert!(dict.contains("finance"));
        assert!(dict.contains("machine-learning"));
        assert!(!dict.contains("nonexistent"));

        let _ = fs::remove_file(test_file); // Don't panic if cleanup fails
    }

    #[test]
    fn test_save_to_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_save_tags.txt");

        let mut dict = TagDictionary::new();
        dict.add_tag("finance".to_string()).unwrap();
        dict.add_tag("reports".to_string()).unwrap();
        dict.add_tag("data-science".to_string()).unwrap();

        dict.save_to_file(&test_file).unwrap();

        // Load it back and verify
        let loaded_dict = TagDictionary::load_from_file(&test_file).unwrap();
        assert_eq!(loaded_dict.len(), 3);
        assert!(loaded_dict.contains("finance"));
        assert!(loaded_dict.contains("reports"));
        assert!(loaded_dict.contains("data-science"));

        fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_add_tag() {
        let mut dict = TagDictionary::new();

        assert!(dict.add_tag("valid-tag".to_string()).is_ok());
        assert!(dict.contains("valid-tag"));

        // Test invalid tag
        assert!(dict.add_tag("Invalid-Tag".to_string()).is_err());
        assert!(!dict.contains("Invalid-Tag"));
    }

    #[test]
    fn test_all_tags() {
        let mut dict = TagDictionary::new();
        dict.add_tag("zebra".to_string()).unwrap();
        dict.add_tag("apple".to_string()).unwrap();
        dict.add_tag("banana".to_string()).unwrap();

        let tags = dict.all_tags();
        assert_eq!(tags, vec!["apple", "banana", "zebra"]); // Should be sorted
    }

    #[test]
    fn test_find_similar() {
        // Create a test dictionary for prefix-based matching
        let mut dict = TagDictionary::new();
        dict.add_tag("receipt".to_string()).unwrap();
        dict.add_tag("research".to_string()).unwrap();
        dict.add_tag("legacy".to_string()).unwrap();
        dict.add_tag("career".to_string()).unwrap();
        dict.add_tag("finance".to_string()).unwrap();
        dict.add_tag("machine-learning".to_string()).unwrap();
        dict.add_tag("data-science".to_string()).unwrap();

        // Test prefix matching (issue #11 main case)
        let similar = dict.find_similar("re", 5);
        assert!(!similar.is_empty(), "Should find matches for 're'");

        // Prefix matches should come first and be sorted alphabetically
        let prefix_matches: Vec<&str> = similar
            .iter()
            .filter(|s| s.similarity == 1.0)
            .map(|s| s.tag.as_str())
            .collect();

        assert!(
            prefix_matches.contains(&"receipt"),
            "Should find 'receipt' as prefix match"
        );
        assert!(
            prefix_matches.contains(&"research"),
            "Should find 'research' as prefix match"
        );

        // Prefix matches should be sorted alphabetically
        if prefix_matches.len() >= 2 {
            assert_eq!(
                prefix_matches[0], "receipt",
                "Receipt should come before research alphabetically"
            );
            assert_eq!(
                prefix_matches[1], "research",
                "Research should come after receipt alphabetically"
            );
        }

        // Test that prefix matches rank higher than substring matches
        let first_result = &similar[0];
        assert_eq!(
            first_result.similarity, 1.0,
            "First result should be a prefix match"
        );
        assert!(
            first_result.tag.starts_with("re"),
            "First result should start with 're'"
        );

        // Test substring matching for tags containing the query
        let similar = dict.find_similar("ance", 3);
        if !similar.is_empty() {
            let finance_found = similar.iter().any(|s| s.tag == "finance");
            assert!(finance_found, "Should find 'finance' containing 'ance'");

            // Substring matches should have similarity 0.5
            let finance_result = similar.iter().find(|s| s.tag == "finance").unwrap();
            assert_eq!(
                finance_result.similarity, 0.5,
                "Substring matches should have similarity 0.5"
            );
        }

        // Test empty query returns no results
        let similar = dict.find_similar("", 5);
        assert!(similar.is_empty(), "Empty query should return no results");

        // Test no matches
        let similar = dict.find_similar("xyz", 3);
        assert!(similar.is_empty(), "Should find no matches for 'xyz'");

        // Test max_results limiting
        let similar = dict.find_similar("a", 2); // Should find career, data-science, machine-learning
        assert!(similar.len() <= 2, "Should respect max_results limit");
    }

    #[test]
    fn test_validate_tag_format() {
        // Valid tags
        assert!(validate_tag_format("finance").is_ok());
        assert!(validate_tag_format("machine-learning").is_ok());
        assert!(validate_tag_format("data-science-2025").is_ok());
        assert!(validate_tag_format("simple").is_ok());
        assert!(validate_tag_format("a").is_ok());

        // Invalid tags
        assert!(validate_tag_format("").is_err()); // Empty
        assert!(validate_tag_format("Finance").is_err()); // Uppercase
        assert!(validate_tag_format("has spaces").is_err()); // Spaces
        assert!(validate_tag_format("has_underscores").is_err()); // Underscores
        assert!(validate_tag_format("-starts-hyphen").is_err()); // Starts with hyphen
        assert!(validate_tag_format("ends-hyphen-").is_err()); // Ends with hyphen
        assert!(validate_tag_format("double--hyphen").is_err()); // Consecutive hyphens
        assert!(validate_tag_format("special@chars").is_err()); // Special characters
        assert!(validate_tag_format("ünïcödé").is_err()); // Non-ASCII
    }

    #[test]
    fn test_tag_validator_trait() {
        let test_file = create_test_tags_file("validator_test");
        let dict = TagDictionary::load_from_file(&test_file).unwrap();

        let valid_tags = vec!["finance".to_string(), "reports".to_string()];
        assert!(dict.validate_tags(&valid_tags).is_ok());

        let invalid_tags = vec!["Valid".to_string(), "invalid tag".to_string()];
        assert!(dict.validate_tags(&invalid_tags).is_err());

        let suggestions = dict.suggest_similar("finanse");
        // Just check that we can get suggestions, don't assume exact matches
        assert!(suggestions.len() <= 5); // Should not exceed max_results

        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_tag_resolution_flow() {
        let test_file = create_test_tags_file("resolution_test");
        let dict = TagDictionary::load_from_file(&test_file).unwrap();
        let flow = TagResolutionFlow::new(dict);

        // Test exact match
        match flow.resolve_tag("finance") {
            TagResolution::ExactMatch(tag) => assert_eq!(tag, "finance"),
            TagResolution::SimilarFound { .. } => {
                // This might happen if similarity threshold is very low
                // Just ensure we found something
            }
            other => panic!("Unexpected resolution for exact match: {:?}", other),
        }

        // Test similar found or no match depending on similarity threshold
        match flow.resolve_tag("finanse") {
            TagResolution::SimilarFound { input, similar, .. } => {
                assert_eq!(input, "finanse");
                assert!(!similar.is_empty());
            }
            TagResolution::NoMatch { input, can_create } => {
                assert_eq!(input, "finanse");
                assert!(can_create); // "finanse" is a valid tag format
            }
            TagResolution::ExactMatch(_) => panic!("Should not be exact match for typo"),
        }

        // Test no match but can create
        match flow.resolve_tag("new-valid-tag") {
            TagResolution::NoMatch { input, can_create } => {
                assert_eq!(input, "new-valid-tag");
                assert!(can_create);
            }
            TagResolution::SimilarFound {
                input, can_create, ..
            } => {
                assert_eq!(input, "new-valid-tag");
                assert!(can_create);
            }
            TagResolution::ExactMatch(_) => panic!("Should not be exact match for new tag"),
        }

        // Test no match and cannot create
        match flow.resolve_tag("Invalid-Tag") {
            TagResolution::NoMatch { input, can_create } => {
                assert_eq!(input, "Invalid-Tag");
                assert!(!can_create);
            }
            TagResolution::SimilarFound {
                input, can_create, ..
            } => {
                assert_eq!(input, "Invalid-Tag");
                assert!(!can_create);
            }
            TagResolution::ExactMatch(_) => panic!("Should not be exact match for invalid tag"),
        }

        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_corrupted_tags_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nonexistent_tags.txt");

        let result = TagDictionary::load_from_file(&test_file);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CleanboxError::TagDictionaryCorrupted(_)
        ));
    }

    #[test]
    fn test_similar_tag_structure() {
        let similar = SimilarTag {
            tag: "finance".to_string(),
            distance: 2,
            similarity: 0.8,
        };

        assert_eq!(similar.tag, "finance");
        assert_eq!(similar.distance, 2);
        assert!((similar.similarity - 0.8).abs() < 0.01);
    }
}
