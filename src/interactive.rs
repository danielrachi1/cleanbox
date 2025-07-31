use crate::document::{DocumentInput, suggest_document_date, today_date_string};
use crate::error::{CleanboxError, Result};
use crate::filesystem::FileManager;
use crate::tags::{TagDictionary, TagResolution, TagResolutionFlow};
use std::io::{self, Write};
use std::path::Path;

pub trait UserPrompt {
    fn prompt_string(&self, message: &str, default: Option<&str>) -> Result<String>;
    fn prompt_confirmation(&self, message: &str, default: bool) -> Result<bool>;
    fn prompt_selection(&self, message: &str, options: &[&str]) -> Result<usize>;
}

#[derive(Clone)]
pub struct ConsolePrompt;

impl ConsolePrompt {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConsolePrompt {
    fn default() -> Self {
        Self::new()
    }
}

impl UserPrompt for ConsolePrompt {
    fn prompt_string(&self, message: &str, default: Option<&str>) -> Result<String> {
        loop {
            if let Some(default_val) = default {
                print!("{message} [{default_val}]: ");
            } else {
                print!("{message}: ");
            }
            io::stdout().flush().map_err(CleanboxError::Io)?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(CleanboxError::Io)?;
            let input = input.trim();

            if input.is_empty() {
                if let Some(default_val) = default {
                    return Ok(default_val.to_string());
                } else {
                    println!("Input cannot be empty. Please try again.");
                    continue;
                }
            }

            return Ok(input.to_string());
        }
    }

    fn prompt_confirmation(&self, message: &str, default: bool) -> Result<bool> {
        loop {
            let default_str = if default { "Y/n" } else { "y/N" };
            print!("{message} [{default_str}]: ");
            io::stdout().flush().map_err(CleanboxError::Io)?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(CleanboxError::Io)?;
            let input = input.trim().to_lowercase();

            match input.as_str() {
                "" => return Ok(default),
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => {
                    println!("Please enter 'y' for yes or 'n' for no.");
                    continue;
                }
            }
        }
    }

    fn prompt_selection(&self, message: &str, options: &[&str]) -> Result<usize> {
        loop {
            println!("{message}");
            for (i, option) in options.iter().enumerate() {
                println!("  {}. {}", i + 1, option);
            }
            print!("Select (1-{}): ", options.len());
            io::stdout().flush().map_err(CleanboxError::Io)?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(CleanboxError::Io)?;

            match input.trim().parse::<usize>() {
                Ok(choice) if choice >= 1 && choice <= options.len() => {
                    return Ok(choice - 1); // Convert to 0-based index
                }
                _ => {
                    println!(
                        "Invalid selection. Please enter a number between 1 and {}.",
                        options.len()
                    );
                    continue;
                }
            }
        }
    }
}

pub struct DatePrompt<P: UserPrompt, F: FileManager> {
    prompter: P,
    file_manager: F,
}

impl<P: UserPrompt, F: FileManager> DatePrompt<P, F> {
    pub fn new(prompter: P, file_manager: F) -> Self {
        Self { prompter, file_manager }
    }

    pub fn prompt_date(&self) -> Result<String> {
        let today = today_date_string();

        loop {
            let input = self
                .prompter
                .prompt_string("Date (YYYY-MM-DD)", Some(&today))?;

            // Validate date format
            if let Err(e) =
                DocumentInput::new(input.clone(), "temp".to_string(), vec!["temp".to_string()])
                    .validate_date()
            {
                println!("Invalid date format: {e}");
                continue;
            }

            return Ok(input);
        }
    }

    pub fn prompt_date_with_smart_suggestion<PA: AsRef<Path>>(&self, filename: PA) -> Result<String> {
        let suggested_date = suggest_document_date(&filename, &self.file_manager);

        loop {
            let input = self
                .prompter
                .prompt_string("Date (YYYY-MM-DD)", Some(&suggested_date))?;

            // Validate date format
            if let Err(e) =
                DocumentInput::new(input.clone(), "temp".to_string(), vec!["temp".to_string()])
                    .validate_date()
            {
                println!("Invalid date format: {e}");
                continue;
            }

            return Ok(input);
        }
    }
}

pub struct DescriptionPrompt<P: UserPrompt> {
    prompter: P,
}

impl<P: UserPrompt> DescriptionPrompt<P> {
    pub fn new(prompter: P) -> Self {
        Self { prompter }
    }

    pub fn prompt_description(&self) -> Result<String> {
        loop {
            let input = self
                .prompter
                .prompt_string("Description (kebab-case)", None)?;

            // Validate description format
            if let Err(e) = DocumentInput::new(
                "2025-01-01".to_string(),
                input.clone(),
                vec!["temp".to_string()],
            )
            .validate_description()
            {
                println!("Invalid description format: {e}");
                continue;
            }

            return Ok(input);
        }
    }
}

pub struct SmartTagSelector<P: UserPrompt> {
    prompter: P,
    flow: TagResolutionFlow,
}

impl<P: UserPrompt> SmartTagSelector<P> {
    pub fn new(prompter: P, tag_dictionary: TagDictionary) -> Self {
        Self {
            prompter,
            flow: TagResolutionFlow::new(tag_dictionary),
        }
    }

    pub fn prompt_tags(&mut self) -> Result<Vec<String>> {
        let mut selected_tags = Vec::new();

        println!(
            "Enter tags (comma-separated or one at a time). Press Enter with empty input when done:"
        );

        loop {
            let input = self.prompter.prompt_string("Tags", None)?;

            if input.is_empty() {
                if selected_tags.is_empty() {
                    println!("At least one tag is required.");
                    continue;
                } else {
                    break;
                }
            }

            // Parse comma-separated tags
            let input_tags: Vec<&str> = input
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            for tag in input_tags {
                if let Some(resolved_tag) = self.resolve_single_tag(tag)? {
                    if !selected_tags.contains(&resolved_tag) {
                        selected_tags.push(resolved_tag);
                        println!("Added tag: {}", selected_tags.last().unwrap());
                    } else {
                        println!("Tag '{resolved_tag}' already added.");
                    }
                }
            }

            if !selected_tags.is_empty() {
                println!("Current tags: {}", selected_tags.join(", "));
                if self.prompter.prompt_confirmation("Add more tags?", false)? {
                    continue;
                } else {
                    break;
                }
            }
        }

        Ok(selected_tags)
    }

    fn resolve_single_tag(&mut self, tag: &str) -> Result<Option<String>> {
        match self.flow.resolve_tag(tag) {
            TagResolution::ExactMatch(matched_tag) => Ok(Some(matched_tag)),
            TagResolution::SimilarFound {
                input,
                similar,
                can_create,
            } => {
                println!("Tag '{input}' not found. Similar tags:");

                let mut options: Vec<String> = similar.iter().map(|s| s.tag.clone()).collect();
                if can_create {
                    options.push(format!("Create new tag '{input}'"));
                }

                let option_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
                let selection = self
                    .prompter
                    .prompt_selection("Choose an option:", &option_refs)?;

                if can_create && selection == options.len() - 1 {
                    // User chose to create new tag
                    self.flow.dictionary_mut().add_tag(input.to_string())?;
                    println!("Created new tag: {input}");
                    Ok(Some(input.to_string()))
                } else {
                    // User chose an existing similar tag
                    Ok(Some(similar[selection].tag.clone()))
                }
            }
            TagResolution::NoMatch { input, can_create } => {
                if can_create {
                    if self
                        .prompter
                        .prompt_confirmation(&format!("Create new tag '{input}'?"), true)?
                    {
                        self.flow.dictionary_mut().add_tag(input.to_string())?;
                        println!("Created new tag: {input}");
                        Ok(Some(input.to_string()))
                    } else {
                        Ok(None)
                    }
                } else {
                    println!("Invalid tag format: {input}");
                    println!(
                        "Tags must be lowercase, kebab-case, and contain only ASCII characters."
                    );
                    Ok(None)
                }
            }
        }
    }

    pub fn save_dictionary(&self, file_path: &std::path::Path) -> Result<()> {
        self.flow.dictionary().save_to_file(file_path)
    }
}

pub struct DocumentInputCollector<P: UserPrompt, F: FileManager> {
    date_prompt: DatePrompt<P, F>,
    description_prompt: DescriptionPrompt<P>,
    tag_selector: SmartTagSelector<P>,
}

impl<F: FileManager + Clone> DocumentInputCollector<ConsolePrompt, F> {
    pub fn new_console(tag_dictionary: TagDictionary, file_manager: F) -> Self {
        let prompter = ConsolePrompt::new();
        Self {
            date_prompt: DatePrompt::new(ConsolePrompt::new(), file_manager.clone()),
            description_prompt: DescriptionPrompt::new(ConsolePrompt::new()),
            tag_selector: SmartTagSelector::new(prompter, tag_dictionary),
        }
    }
}

impl<P: UserPrompt + Clone, F: FileManager + Clone> DocumentInputCollector<P, F> {
    pub fn new(prompter: P, tag_dictionary: TagDictionary, file_manager: F) -> Self {
        Self {
            date_prompt: DatePrompt::new(prompter.clone(), file_manager),
            description_prompt: DescriptionPrompt::new(prompter.clone()),
            tag_selector: SmartTagSelector::new(prompter, tag_dictionary),
        }
    }

    pub fn new_separate(
        date_prompter: P,
        desc_prompter: P,
        tag_prompter: P,
        tag_dictionary: TagDictionary,
        file_manager: F,
    ) -> Self {
        Self {
            date_prompt: DatePrompt::new(date_prompter, file_manager),
            description_prompt: DescriptionPrompt::new(desc_prompter),
            tag_selector: SmartTagSelector::new(tag_prompter, tag_dictionary),
        }
    }

    pub fn collect_input(&mut self, filename: &str) -> Result<DocumentInput> {
        println!("\nProcessing document: {filename}");

        let date = self.date_prompt.prompt_date_with_smart_suggestion(filename)?;
        let description = self.description_prompt.prompt_description()?;
        let tags = self.tag_selector.prompt_tags()?;

        let input = DocumentInput::new(date, description, tags);
        input.validate()?; // Final validation

        Ok(input)
    }

    pub fn save_tag_dictionary(&self, file_path: &std::path::Path) -> Result<()> {
        self.tag_selector.save_dictionary(file_path)
    }
}

pub struct ProgressIndicator {
    current: usize,
    total: usize,
    task_name: String,
}

impl ProgressIndicator {
    pub fn new(total: usize, task_name: String) -> Self {
        Self {
            current: 0,
            total,
            task_name,
        }
    }

    pub fn start(&self) {
        println!("Starting {}: 0/{} files", self.task_name, self.total);
    }

    pub fn update(&mut self, current: usize) {
        self.current = current;
        let percentage = if self.total > 0 {
            (current * 100) / self.total
        } else {
            100
        };

        let bar_length = 40;
        let filled = (current * bar_length) / self.total.max(1);
        let bar = "█".repeat(filled) + &"░".repeat(bar_length - filled);

        print!(
            "\r{}: {} {}/{} ({}%)",
            self.task_name, bar, current, self.total, percentage
        );
        io::stdout().flush().unwrap_or(());
    }

    pub fn finish(&mut self) {
        self.update(self.total);
        println!(); // New line after progress bar
        println!(
            "Completed {}: {}/{} files",
            self.task_name, self.total, self.total
        );
    }

    pub fn increment(&mut self) {
        self.update(self.current + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags::TagDictionary;

    // Mock prompter for testing
    pub struct MockPrompt {
        pub string_responses: Vec<String>,
        pub confirmation_responses: Vec<bool>,
        pub selection_responses: Vec<usize>,
        string_index: std::cell::RefCell<usize>,
        confirmation_index: std::cell::RefCell<usize>,
        selection_index: std::cell::RefCell<usize>,
    }

    impl MockPrompt {
        pub fn new() -> Self {
            Self {
                string_responses: Vec::new(),
                confirmation_responses: Vec::new(),
                selection_responses: Vec::new(),
                string_index: std::cell::RefCell::new(0),
                confirmation_index: std::cell::RefCell::new(0),
                selection_index: std::cell::RefCell::new(0),
            }
        }

        pub fn with_strings(mut self, responses: Vec<String>) -> Self {
            self.string_responses = responses;
            self
        }

        pub fn with_confirmations(mut self, responses: Vec<bool>) -> Self {
            self.confirmation_responses = responses;
            self
        }

        pub fn with_selections(mut self, responses: Vec<usize>) -> Self {
            self.selection_responses = responses;
            self
        }
    }

    impl Clone for MockPrompt {
        fn clone(&self) -> Self {
            Self {
                string_responses: self.string_responses.clone(),
                confirmation_responses: self.confirmation_responses.clone(),
                selection_responses: self.selection_responses.clone(),
                string_index: std::cell::RefCell::new(*self.string_index.borrow()),
                confirmation_index: std::cell::RefCell::new(*self.confirmation_index.borrow()),
                selection_index: std::cell::RefCell::new(*self.selection_index.borrow()),
            }
        }
    }

    impl UserPrompt for MockPrompt {
        fn prompt_string(&self, _message: &str, default: Option<&str>) -> Result<String> {
            let mut index = self.string_index.borrow_mut();
            if *index < self.string_responses.len() {
                let response = self.string_responses[*index].clone();
                *index += 1;
                if response.is_empty() && default.is_some() {
                    Ok(default.unwrap().to_string())
                } else {
                    Ok(response)
                }
            } else {
                Err(CleanboxError::InvalidUserInput(
                    "No more mock responses".to_string(),
                ))
            }
        }

        fn prompt_confirmation(&self, _message: &str, default: bool) -> Result<bool> {
            let mut index = self.confirmation_index.borrow_mut();
            if *index < self.confirmation_responses.len() {
                let response = self.confirmation_responses[*index];
                *index += 1;
                Ok(response)
            } else {
                Ok(default)
            }
        }

        fn prompt_selection(&self, _message: &str, options: &[&str]) -> Result<usize> {
            let mut index = self.selection_index.borrow_mut();
            if *index < self.selection_responses.len() {
                let response = self.selection_responses[*index];
                *index += 1;
                if response < options.len() {
                    Ok(response)
                } else {
                    Err(CleanboxError::InvalidUserInput(
                        "Invalid selection".to_string(),
                    ))
                }
            } else {
                Err(CleanboxError::InvalidUserInput(
                    "No more mock responses".to_string(),
                ))
            }
        }
    }

    #[test]
    fn test_date_prompt_with_default() {
        use crate::filesystem::MockFileManager;
        
        let mock = MockPrompt::new().with_strings(vec!["".to_string()]); // Empty input, should use default
        let file_manager = MockFileManager::new();
        let date_prompt = DatePrompt::new(mock, file_manager);

        let result = date_prompt.prompt_date().unwrap();
        assert_eq!(result.len(), 10); // YYYY-MM-DD format
        assert!(result.contains("-"));
    }

    #[test]
    fn test_date_prompt_with_custom_date() {
        use crate::filesystem::MockFileManager;
        
        let mock = MockPrompt::new().with_strings(vec!["2025-06-15".to_string()]);
        let file_manager = MockFileManager::new();
        let date_prompt = DatePrompt::new(mock, file_manager);

        let result = date_prompt.prompt_date().unwrap();
        assert_eq!(result, "2025-06-15");
    }

    #[test]
    fn test_description_prompt() {
        let mock = MockPrompt::new().with_strings(vec!["quarterly-report".to_string()]);
        let description_prompt = DescriptionPrompt::new(mock);

        let result = description_prompt.prompt_description().unwrap();
        assert_eq!(result, "quarterly-report");
    }

    #[test]
    fn test_progress_indicator() {
        let mut progress = ProgressIndicator::new(10, "Test Task".to_string());

        assert_eq!(progress.current, 0);
        assert_eq!(progress.total, 10);

        progress.update(5);
        assert_eq!(progress.current, 5);

        progress.increment();
        assert_eq!(progress.current, 6);
    }

    #[test]
    fn test_smart_tag_selector_exact_match() {
        let mut dict = TagDictionary::new();
        dict.add_tag("finance".to_string()).unwrap();
        dict.add_tag("reports".to_string()).unwrap();

        let mock = MockPrompt::new().with_strings(vec!["finance".to_string(), "".to_string()]);
        let mut selector = SmartTagSelector::new(mock, dict);

        let result = selector.prompt_tags().unwrap();
        assert_eq!(result, vec!["finance"]);
    }

    #[test]
    fn test_document_input_collector_components() {
        use crate::filesystem::MockFileManager;
        
        let mut dict = TagDictionary::new();
        dict.add_tag("finance".to_string()).unwrap();

        // Test each component separately first
        let date_mock = MockPrompt::new().with_strings(vec!["2025-07-31".to_string()]);
        let file_manager = MockFileManager::new();
        let date_prompt = DatePrompt::new(date_mock, file_manager);
        let date_result = date_prompt.prompt_date().unwrap();
        assert_eq!(date_result, "2025-07-31");

        let desc_mock = MockPrompt::new().with_strings(vec!["quarterly-report".to_string()]);
        let desc_prompt = DescriptionPrompt::new(desc_mock);
        let desc_result = desc_prompt.prompt_description().unwrap();
        assert_eq!(desc_result, "quarterly-report");

        // Test tag selector separately
        let tag_mock = MockPrompt::new()
            .with_strings(vec!["finance".to_string(), "".to_string()])
            .with_confirmations(vec![false]);
        let mut tag_selector = SmartTagSelector::new(tag_mock, dict);
        let tag_result = tag_selector.prompt_tags().unwrap();
        assert_eq!(tag_result, vec!["finance"]);
    }

    #[test]
    fn test_document_input_collector_full() {
        use crate::filesystem::MockFileManager;
        
        let mut dict = TagDictionary::new();
        dict.add_tag("finance".to_string()).unwrap();

        // Create separate prompters for each component to avoid response conflicts
        let date_mock = MockPrompt::new().with_strings(vec!["2025-07-31".to_string()]);
        let desc_mock = MockPrompt::new().with_strings(vec!["quarterly-report".to_string()]);
        let tag_mock = MockPrompt::new()
            .with_strings(vec!["finance".to_string(), "".to_string()])
            .with_confirmations(vec![false]);
        let file_manager = MockFileManager::new();

        let mut collector =
            DocumentInputCollector::new_separate(date_mock, desc_mock, tag_mock, dict, file_manager);
        let result = collector.collect_input("test.pdf").unwrap();

        assert_eq!(result.date, "2025-07-31");
        assert_eq!(result.description, "quarterly-report");
        assert_eq!(result.tags, vec!["finance"]);
    }
}
