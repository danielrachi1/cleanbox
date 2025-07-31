use crate::config::{DuplicateHandling, ProcessingConfig};
use crate::error::{CleanboxError, Result};
use crate::metadata::MetadataParser;
use crate::filesystem::{FileHasher, FileManager};
use crate::media::File;
use crate::naming::NamingStrategy;
use crate::organization::OrganizationStrategy;
use std::path::Path;

pub struct FileProcessor<E, F, N, O>
where
    E: MetadataParser,
    F: FileManager,
    N: NamingStrategy,
    O: OrganizationStrategy,
{
    exif_parser: E,
    file_manager: F,
    naming_strategy: N,
    organization_strategy: O,
    config: ProcessingConfig,
}

#[derive(Debug)]
pub struct ProcessingResult {
    pub processed_files: usize,
    pub skipped_files: usize,
    pub failed_files: usize,
    pub errors: Vec<String>,
}

impl ProcessingResult {
    fn new() -> Self {
        Self {
            processed_files: 0,
            skipped_files: 0,
            failed_files: 0,
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.failed_files += 1;
    }

    fn skip_file(&mut self) {
        self.skipped_files += 1;
    }

    fn process_file(&mut self) {
        self.processed_files += 1;
    }
}

impl<E, F, N, O> FileProcessor<E, F, N, O>
where
    E: MetadataParser,
    F: FileManager,
    N: NamingStrategy,
    O: OrganizationStrategy,
{
    pub fn new(
        exif_parser: E,
        file_manager: F,
        naming_strategy: N,
        organization_strategy: O,
        config: ProcessingConfig,
    ) -> Self {
        Self {
            exif_parser,
            file_manager,
            naming_strategy,
            organization_strategy,
            config,
        }
    }

    pub fn config(&self) -> &ProcessingConfig {
        &self.config
    }

    pub fn process_directory(&self) -> Result<ProcessingResult> {
        let mut result = ProcessingResult::new();

        let file_paths = self.file_manager.read_directory(&self.config.inbox_path)?;

        for file_path in file_paths {
            if !self.file_manager.is_file(&file_path) {
                continue;
            }

            match self.process_single_file(&file_path) {
                Ok(()) => result.process_file(),
                Err(e) => {
                    let error_msg = format!("{}: {}", file_path.display(), e);
                    if self.should_skip_error(&e) {
                        result.skip_file();
                        if !self.config.skip_unsupported_files {
                            result.add_error(error_msg);
                        }
                    } else {
                        result.add_error(error_msg);
                    }
                }
            }
        }

        Ok(result)
    }

    fn process_single_file(&self, file_path: &Path) -> Result<()> {
        let mut file = File::new(file_path);

        let metadata = self.exif_parser.parse_metadata(file_path)?;

        if !metadata.file_type.is_supported() {
            return Err(CleanboxError::UnsupportedFileType(
                metadata.mime_type.clone(),
            ));
        }

        file = file.with_metadata(metadata);

        let new_name = self.naming_strategy.generate_name(&file)?;
        let temp_path = file_path.with_file_name(&new_name);

        if file_path != temp_path {
            self.file_manager.rename_file(file_path, &temp_path)?;
        }

        let target_dir = self
            .organization_strategy
            .determine_target_directory(&file, &self.config.media_root)?;

        let mut target_path = target_dir.join(&new_name);

        if self.file_manager.file_exists(&target_path) {
            target_path = self.handle_duplicate(&temp_path, &target_path)?;
        }

        self.file_manager.move_file(&temp_path, &target_path)?;

        println!("Moved to {}", target_path.display());
        Ok(())
    }

    fn handle_duplicate(
        &self,
        source_path: &Path,
        target_path: &Path,
    ) -> Result<std::path::PathBuf> {
        match self.config.handle_duplicates {
            DuplicateHandling::Skip => Err(CleanboxError::FileAlreadyExists(
                target_path.display().to_string(),
            )),
            DuplicateHandling::Overwrite => Ok(target_path.to_path_buf()),
            DuplicateHandling::Error => Err(CleanboxError::FileAlreadyExists(
                target_path.display().to_string(),
            )),
            DuplicateHandling::AppendHash => {
                let hash = self.file_manager.calculate_file_hash(source_path)?;
                let hash_suffix = FileHasher::generate_hash_suffix(&hash, self.config.hash_length);

                let original_name = target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| CleanboxError::InvalidPath(target_path.display().to_string()))?;

                let new_name = FileHasher::append_hash_to_filename(original_name, &hash_suffix)?;
                Ok(target_path.with_file_name(new_name))
            }
        }
    }

    fn should_skip_error(&self, error: &CleanboxError) -> bool {
        matches!(
            error,
            CleanboxError::UnsupportedFileType(_) | CleanboxError::Exif(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DuplicateHandling;
    use crate::filesystem::MockFileManager;
    use crate::media::{File, FileMetadata};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    // Mock ExifParser for testing
    pub struct MockExifParser {
        pub results: HashMap<PathBuf, Result<FileMetadata>>,
    }

    impl MockExifParser {
        pub fn new() -> Self {
            Self {
                results: HashMap::new(),
            }
        }

        #[allow(dead_code)]
        pub fn add_result(&mut self, path: PathBuf, result: Result<FileMetadata>) {
            self.results.insert(path, result);
        }
    }

    impl crate::metadata::MetadataParser for MockExifParser {
        fn parse_metadata<P: AsRef<Path>>(&self, file_path: P) -> Result<FileMetadata> {
            let path_buf = file_path.as_ref().to_path_buf();
            if let Some(result) = self.results.get(&path_buf) {
                match result {
                    Ok(metadata) => Ok(metadata.clone()),
                    Err(err) => Err(CleanboxError::Exif(err.to_string())),
                }
            } else {
                Err(CleanboxError::Exif("No mock result configured".to_string()))
            }
        }

        fn extract_datetime<P: AsRef<Path>>(&self, _file_path: P) -> Result<String> {
            Ok("2023-12-01_14-30-00".to_string())
        }

        fn supports_file_type(&self, file_type: &crate::media::FileType) -> bool {
            matches!(file_type, crate::media::FileType::Image | crate::media::FileType::Video)
        }
    }

    // Mock NamingStrategy for testing
    pub struct MockNamingStrategy {
        pub name: String,
    }

    impl MockNamingStrategy {
        pub fn new(name: String) -> Self {
            Self { name }
        }
    }

    impl crate::naming::NamingStrategy for MockNamingStrategy {
        fn generate_name(&self, _file: &File) -> Result<String> {
            Ok(self.name.clone())
        }
    }

    // Mock OrganizationStrategy for testing
    pub struct MockOrganizationStrategy {
        pub directory: PathBuf,
    }

    impl MockOrganizationStrategy {
        pub fn new(directory: PathBuf) -> Self {
            Self { directory }
        }
    }

    impl crate::organization::OrganizationStrategy for MockOrganizationStrategy {
        fn determine_target_directory(
            &self,
            _file: &File,
            base_path: &Path,
        ) -> Result<PathBuf> {
            Ok(base_path.join(&self.directory))
        }
    }

    fn create_test_processor()
    -> FileProcessor<MockExifParser, MockFileManager, MockNamingStrategy, MockOrganizationStrategy>
    {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"));

        FileProcessor::new(
            MockExifParser::new(),
            MockFileManager::new(),
            MockNamingStrategy::new("test_file.jpg".to_string()),
            MockOrganizationStrategy::new(PathBuf::from("2023/12")),
            config,
        )
    }

    #[test]
    fn test_processing_result_new() {
        let result = ProcessingResult::new();
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.skipped_files, 0);
        assert_eq!(result.failed_files, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_processing_result_add_error() {
        let mut result = ProcessingResult::new();
        result.add_error("Test error".to_string());

        assert_eq!(result.failed_files, 1);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Test error");
    }

    #[test]
    fn test_processing_result_skip_file() {
        let mut result = ProcessingResult::new();
        result.skip_file();

        assert_eq!(result.skipped_files, 1);
    }

    #[test]
    fn test_processing_result_process_file() {
        let mut result = ProcessingResult::new();
        result.process_file();

        assert_eq!(result.processed_files, 1);
    }

    #[test]
    fn test_file_processor_creation() {
        let processor = create_test_processor();
        assert_eq!(processor.config.inbox_path, PathBuf::from("/inbox"));
        assert_eq!(processor.config.media_root, PathBuf::from("/media"));
    }

    #[test]
    fn test_should_skip_error() {
        let processor = create_test_processor();

        assert!(
            processor.should_skip_error(&CleanboxError::UnsupportedFileType(
                "text/plain".to_string()
            ))
        );
        assert!(processor.should_skip_error(&CleanboxError::Exif("No EXIF data".to_string())));
        assert!(
            !processor.should_skip_error(&CleanboxError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "test"
            )))
        );
        assert!(!processor.should_skip_error(&CleanboxError::InvalidPath("/invalid".to_string())));
    }

    #[test]
    fn test_handle_duplicate_skip() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"))
            .with_duplicate_handling(DuplicateHandling::Skip);

        let processor = FileProcessor::new(
            MockExifParser::new(),
            MockFileManager::new(),
            MockNamingStrategy::new("test.jpg".to_string()),
            MockOrganizationStrategy::new(PathBuf::from("2023")),
            config,
        );

        let source = Path::new("/temp/file.jpg");
        let target = Path::new("/media/target.jpg");
        let result = processor.handle_duplicate(source, target);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CleanboxError::FileAlreadyExists(_)
        ));
    }

    #[test]
    fn test_handle_duplicate_overwrite() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"))
            .with_duplicate_handling(DuplicateHandling::Overwrite);

        let processor = FileProcessor::new(
            MockExifParser::new(),
            MockFileManager::new(),
            MockNamingStrategy::new("test.jpg".to_string()),
            MockOrganizationStrategy::new(PathBuf::from("2023")),
            config,
        );

        let source = Path::new("/temp/file.jpg");
        let target = Path::new("/media/target.jpg");
        let result = processor.handle_duplicate(source, target).unwrap();

        assert_eq!(result, target);
    }

    #[test]
    fn test_handle_duplicate_append_hash() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"))
            .with_duplicate_handling(DuplicateHandling::AppendHash)
            .with_hash_length(8);

        let mut file_manager = MockFileManager::new();
        file_manager.add_file(PathBuf::from("/temp/file.jpg"), b"test content".to_vec());

        let processor = FileProcessor::new(
            MockExifParser::new(),
            file_manager,
            MockNamingStrategy::new("test.jpg".to_string()),
            MockOrganizationStrategy::new(PathBuf::from("2023")),
            config,
        );

        let source = Path::new("/temp/file.jpg");
        let target = Path::new("/media/target.jpg");
        let result = processor.handle_duplicate(source, target).unwrap();

        let filename = result.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("target_"));
        assert!(filename.ends_with(".jpg"));
        assert!(filename.len() > "target.jpg".len()); // Should have hash appended
    }
}
