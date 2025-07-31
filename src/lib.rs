pub mod config;
pub mod document;
pub mod error;
pub mod interactive;
pub mod metadata;
pub mod filesystem;
pub mod media;
pub mod naming;
pub mod organization;
pub mod paths;
pub mod processor;
pub mod tags;

pub use config::{DuplicateHandling, LifeConfig, ProcessingConfig};
pub use document::{DocumentInput, today_date_string};
pub use error::{CleanboxError, Result};
pub use interactive::{ConsolePrompt, DatePrompt, DescriptionPrompt, DocumentInputCollector, ProgressIndicator, SmartTagSelector, UserPrompt};
pub use metadata::{MetadataParser, RexifParser};
pub use filesystem::{FileManager, StdFileManager};
pub use media::{File, FileMetadata, FileType};
pub use naming::{CustomNamingStrategy, DocumentNamingStrategy, NamingStrategy, TimestampNamingStrategy};
pub use organization::{
    CustomOrganizer, DocumentOrganizer, FlatOrganizer, MonthlyOrganizer, OrganizationStrategy, YearlyOrganizer,
};
pub use paths::{BasePathResolver, LifeDirectoryResolver, LifePathResolver};
pub use processor::{CategorizedFiles, FileProcessor, ProcessingResult, UnifiedProcessor, UnifiedProcessingResult};
pub use tags::{SimilarTag, TagDictionary, TagResolution, TagResolutionFlow, TagValidator, validate_tag_format};

use std::path::Path;

pub fn create_default_processor(
    inbox_path: impl AsRef<Path>,
    media_root: impl AsRef<Path>,
) -> FileProcessor<RexifParser, StdFileManager, TimestampNamingStrategy, MonthlyOrganizer, LifeDirectoryResolver> {
    let config = ProcessingConfig::new(
        inbox_path.as_ref().to_path_buf(),
        media_root.as_ref().to_path_buf(),
    );

    FileProcessor::new(
        RexifParser::new(),
        StdFileManager::new(),
        TimestampNamingStrategy::new(),
        MonthlyOrganizer::new(),
        LifeDirectoryResolver::new(),
        config,
    )
}

pub fn process_media_directory(
    inbox_path: impl AsRef<Path>,
    media_root: impl AsRef<Path>,
) -> Result<ProcessingResult> {
    let processor = create_default_processor(inbox_path, media_root);
    processor.process_directory()
}

pub fn process_life_directory(life_path: impl AsRef<Path>) -> Result<ProcessingResult> {
    let life_config = LifeConfig::new(life_path.as_ref().to_path_buf());
    let processing_config = life_config.to_processing_config();
    
    let processor = FileProcessor::new(
        RexifParser::new(),
        StdFileManager::new(),
        TimestampNamingStrategy::new(),
        MonthlyOrganizer::new(),
        LifeDirectoryResolver::new(),
        processing_config,
    );
    
    processor.process_directory()
}

/// Process life directory with unified workflow for both media and documents
pub fn process_life_directory_unified(life_path: impl AsRef<Path>) -> Result<UnifiedProcessingResult> {
    let life_config = LifeConfig::new(life_path.as_ref().to_path_buf());
    
    let unified_processor = UnifiedProcessor::new(
        RexifParser::new(),
        StdFileManager::new(),
        interactive::ConsolePrompt::new(),
        life_config,
    );
    
    unified_processor.process_life_directory()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_create_default_processor() {
        let inbox = PathBuf::from("/test/inbox");
        let media = PathBuf::from("/test/media");
        let processor = create_default_processor(&inbox, &media);

        assert_eq!(processor.config().inbox_path, inbox);
        assert_eq!(processor.config().media_root, media);
        assert_eq!(processor.config().hash_length, 6);
        assert!(matches!(
            processor.config().handle_duplicates,
            DuplicateHandling::AppendHash
        ));
    }

    #[test]
    fn test_all_types_exported() {
        // This test ensures all important types are properly exported
        // and can be used together

        let _config: ProcessingConfig =
            ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"));

        let _parser: RexifParser = RexifParser::new();
        let _file_manager: StdFileManager = StdFileManager::new();
        let _naming: TimestampNamingStrategy = TimestampNamingStrategy::new();
        let _org: MonthlyOrganizer = MonthlyOrganizer::new();
        let _custom_naming: CustomNamingStrategy =
            CustomNamingStrategy::new("{datetime}.{ext}".to_string());
        let _yearly_org: YearlyOrganizer = YearlyOrganizer::new();
        let _flat_org: FlatOrganizer = FlatOrganizer::new();
        let _custom_org: CustomOrganizer = CustomOrganizer::new("{year}/{month}".to_string());

        let _duplicate_handling: DuplicateHandling = DuplicateHandling::AppendHash;

        // Test that Result type alias works
        let _result: Result<String> = Ok("test".to_string());
        let _error: CleanboxError = CleanboxError::InvalidPath("test".to_string());
    }

    #[test]
    fn test_media_types_integration() {
        let file = File::new("/test/image.jpg");
        let metadata = FileMetadata::new("image/jpeg".to_string())
            .with_datetime("2023-12-01_14-30-00".to_string())
            .with_hash("abc123".to_string());

        let file = file.with_metadata(metadata);
        assert!(file.is_supported_media());
        assert_eq!(file.extension().unwrap(), "jpg");
    }

    #[test]
    fn test_error_integration() {
        use std::io;

        // Test error conversion
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test");
        let cleanbox_error: CleanboxError = io_error.into();

        match cleanbox_error {
            CleanboxError::Io(_) => {} // Expected
            _ => panic!("Wrong error type"),
        }

        // Test Result type alias
        let result: Result<i32> = Err(CleanboxError::InvalidPath("test".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_process_media_directory_integration() {
        // This test doesn't actually process files, just verifies the API works
        let result = process_media_directory("/nonexistent/inbox", "/nonexistent/media");
        // Should fail due to nonexistent paths, but the API should be callable
        assert!(result.is_err());
    }

    #[test]
    fn test_process_life_directory_integration() {
        // This test doesn't actually process files, just verifies the API works
        let result = process_life_directory("/nonexistent/life");
        // Should fail due to nonexistent paths, but the API should be callable
        assert!(result.is_err());
    }

    #[test]
    fn test_life_config_integration() {
        let life_config = LifeConfig::new(PathBuf::from("/home/user/life"))
            .with_hash_length(8)
            .with_duplicate_handling(DuplicateHandling::Skip);
            
        assert_eq!(life_config.life_path, PathBuf::from("/home/user/life"));
        assert_eq!(life_config.inbox_path(), PathBuf::from("/home/user/life/inbox"));
        assert_eq!(life_config.media_root(), PathBuf::from("/home/user/life/media"));
        assert_eq!(life_config.documents_root(), PathBuf::from("/home/user/life/documents"));
        assert_eq!(life_config.tags_file(), PathBuf::from("/home/user/life/documents/tags.txt"));
        assert_eq!(life_config.hash_length, 8);
        assert!(matches!(life_config.handle_duplicates, DuplicateHandling::Skip));
    }

    #[test]
    fn test_unified_processing_api() {
        // Test that the unified processing API is callable
        let result = process_life_directory_unified("/nonexistent/life");
        // Should fail due to nonexistent paths, but the API should be callable
        assert!(result.is_err());
    }

    #[test]
    fn test_unified_processing_result() {
        let mut result = UnifiedProcessingResult::new();
        assert_eq!(result.total_processed(), 0);
        
        result.media_processed = 5;
        result.documents_processed = 3;
        assert_eq!(result.total_processed(), 8);
        
        assert_eq!(result.files_skipped, 0);
        assert_eq!(result.files_failed, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_categorized_files() {
        let mut categorized = CategorizedFiles::new();
        assert_eq!(categorized.total_count(), 0);
        
        categorized.media_files.push(PathBuf::from("/test/image.jpg"));
        categorized.document_files.push(PathBuf::from("/test/doc.pdf"));
        categorized.unknown_files.push(PathBuf::from("/test/unknown.xyz"));
        
        assert_eq!(categorized.total_count(), 3);
        assert_eq!(categorized.media_files.len(), 1);
        assert_eq!(categorized.document_files.len(), 1);
        assert_eq!(categorized.unknown_files.len(), 1);
    }
}
