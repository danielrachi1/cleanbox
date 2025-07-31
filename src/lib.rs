pub mod config;
pub mod error;
pub mod metadata;
pub mod filesystem;
pub mod media;
pub mod naming;
pub mod organization;
pub mod paths;
pub mod processor;

pub use config::{DuplicateHandling, ProcessingConfig};
pub use error::{CleanboxError, Result};
pub use metadata::{MetadataParser, RexifParser};
pub use filesystem::{FileManager, StdFileManager};
pub use media::{File, FileMetadata, FileType};
pub use naming::{CustomNamingStrategy, NamingStrategy, TimestampNamingStrategy};
pub use organization::{
    CustomOrganizer, FlatOrganizer, MonthlyOrganizer, OrganizationStrategy, YearlyOrganizer,
};
pub use paths::{BasePathResolver, LifeDirectoryResolver};
pub use processor::{FileProcessor, ProcessingResult};

use std::path::Path;

pub fn create_default_processor(
    inbox_path: impl AsRef<Path>,
    media_root: impl AsRef<Path>,
) -> FileProcessor<RexifParser, StdFileManager, TimestampNamingStrategy, MonthlyOrganizer> {
    let config = ProcessingConfig::new(
        inbox_path.as_ref().to_path_buf(),
        media_root.as_ref().to_path_buf(),
    );

    FileProcessor::new(
        RexifParser::new(),
        StdFileManager::new(),
        TimestampNamingStrategy::new(),
        MonthlyOrganizer::new(),
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
}
