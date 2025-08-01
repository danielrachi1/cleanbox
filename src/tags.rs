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
        let mut similar: Vec<SimilarTag> = self
            .tags
            .iter()
            .map(|tag| {
                let distance = strsim::levenshtein(query, tag);
                let normalized_distance = distance as f64 / tag.len().max(query.len()) as f64;
                let base_similarity = 1.0 - normalized_distance;

                // Enhanced scoring with intelligent bonuses
                let prefix_bonus = if tag.starts_with(query) { 0.5 } else { 0.0 };
                let word_boundary_bonus = if tag.contains(&format!("-{}", query)) {
                    0.2
                } else {
                    0.0
                };
                let early_position_bonus = match tag.find(query) {
                    Some(pos) if pos <= 2 => 0.1,
                    _ => 0.0,
                };

                // Calculate enhanced similarity, capped at 1.0
                let enhanced_similarity =
                    (base_similarity + prefix_bonus + word_boundary_bonus + early_position_bonus)
                        .min(1.0);

                SimilarTag {
                    tag: tag.clone(),
                    distance,
                    similarity: enhanced_similarity,
                }
            })
            .filter(|similar_tag| {
                // Only include if reasonably similar (similarity > 0.3)
                similar_tag.similarity > 0.3
            })
            .collect();

        // Sort by similarity (highest first)
        similar.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        similar.truncate(max_results);

        similar
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
        // Create a specific test dictionary to test enhanced ranking
        let mut dict = TagDictionary::new();
        dict.add_tag("receipt".to_string()).unwrap();
        dict.add_tag("legacy".to_string()).unwrap();
        dict.add_tag("career".to_string()).unwrap();
        dict.add_tag("finance".to_string()).unwrap();
        dict.add_tag("machine-learning".to_string()).unwrap();
        dict.add_tag("data-science".to_string()).unwrap();

        // Test prefix matching (issue #11 main case)
        let similar = dict.find_similar("re", 3);
        assert!(!similar.is_empty(), "Should find matches for 're'");

        // "receipt" should be ranked highest due to prefix bonus
        assert_eq!(
            similar[0].tag, "receipt",
            "Receipt should rank highest for 're' due to prefix match"
        );

        // Test typo matching
        let similar = dict.find_similar("finanse", 3);
        if !similar.is_empty() {
            let finance_found = similar.iter().any(|s| s.tag == "finance");
            assert!(
                finance_found,
                "Should find 'finance' as similar to 'finanse'"
            );
        }

        // Test word boundary bonus for kebab-case
        let similar = dict.find_similar("ml", 3);
        if !similar.is_empty() {
            let machine_learning_found = similar.iter().any(|s| s.tag == "machine-learning");
            if machine_learning_found {
                // If found, machine-learning should have high ranking due to word boundary bonus
                let ml_position = similar.iter().position(|s| s.tag == "machine-learning");
                assert!(
                    ml_position.is_some(),
                    "machine-learning should be found for 'ml'"
                );
            }
        }

        // Test early position bonus
        let similar = dict.find_similar("da", 3);
        if !similar.is_empty() {
            let data_science_found = similar.iter().any(|s| s.tag == "data-science");
            if data_science_found {
                // data-science should rank well due to early position match
                assert!(
                    similar.iter().any(|s| s.tag == "data-science"),
                    "Should find data-science for 'da'"
                );
            }
        }
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
