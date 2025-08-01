use crate::config::{DuplicateHandling, LifeConfig, ProcessingConfig};
use crate::error::{CleanboxError, Result};
use crate::filesystem::{FileHasher, FileManager, StdFileManager};
use crate::interactive::{DocumentInputCollector, UserPrompt};
use crate::media::{File, FileType};
use crate::metadata::{MetadataParser, RexifParser};
use crate::naming::{DocumentNamingStrategy, NamingStrategy, TimestampNamingStrategy};
use crate::organization::{DocumentOrganizer, MonthlyOrganizer, OrganizationStrategy};
use crate::paths::{BasePathResolver, LifeDirectoryResolver};
use crate::tags::TagDictionary;
use std::path::{Path, PathBuf};

pub struct FileProcessor<E, F, N, O, R>
where
    E: MetadataParser,
    F: FileManager,
    N: NamingStrategy,
    O: OrganizationStrategy,
    R: BasePathResolver,
{
    exif_parser: E,
    file_manager: F,
    naming_strategy: N,
    organization_strategy: O,
    base_path_resolver: R,
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

impl<E, F, N, O, R> FileProcessor<E, F, N, O, R>
where
    E: MetadataParser,
    F: FileManager,
    N: NamingStrategy,
    O: OrganizationStrategy,
    R: BasePathResolver,
{
    pub fn new(
        exif_parser: E,
        file_manager: F,
        naming_strategy: N,
        organization_strategy: O,
        base_path_resolver: R,
        config: ProcessingConfig,
    ) -> Self {
        Self {
            exif_parser,
            file_manager,
            naming_strategy,
            organization_strategy,
            base_path_resolver,
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

        // Use processing behavior methods for intelligent routing
        if metadata.file_type.should_skip() {
            return Err(CleanboxError::UnsupportedFileType(
                metadata.mime_type.clone(),
            ));
        }

        // Documents need interactive processing - skip for now in basic processor
        if metadata.file_type.needs_interactive_processing() {
            return Err(CleanboxError::UnsupportedFileType(format!(
                "Document files require interactive processing: {}",
                metadata.mime_type
            )));
        }

        // Only process auto-processable files (Images and Videos)
        if !metadata.file_type.is_auto_processable() {
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

        // Use BasePathResolver to determine correct base path (media/ vs documents/)
        let base_path = self
            .base_path_resolver
            .resolve_base_path(&file.metadata.as_ref().unwrap().file_type, &self.config);
        let target_dir = self
            .organization_strategy
            .determine_target_directory(&file, &base_path)?;

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

/// Categorized files from inbox scan
#[derive(Debug)]
pub struct CategorizedFiles {
    pub media_files: Vec<PathBuf>,
    pub document_files: Vec<PathBuf>,
    pub unknown_files: Vec<PathBuf>,
}

impl Default for CategorizedFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl CategorizedFiles {
    pub fn new() -> Self {
        Self {
            media_files: Vec::new(),
            document_files: Vec::new(),
            unknown_files: Vec::new(),
        }
    }

    pub fn total_count(&self) -> usize {
        self.media_files.len() + self.document_files.len() + self.unknown_files.len()
    }
}

/// Result of unified processing
#[derive(Debug)]
pub struct UnifiedProcessingResult {
    pub media_processed: usize,
    pub documents_processed: usize,
    pub files_skipped: usize,
    pub files_failed: usize,
    pub errors: Vec<String>,
}

impl Default for UnifiedProcessingResult {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedProcessingResult {
    pub fn new() -> Self {
        Self {
            media_processed: 0,
            documents_processed: 0,
            files_skipped: 0,
            files_failed: 0,
            errors: Vec::new(),
        }
    }

    pub fn total_processed(&self) -> usize {
        self.media_processed + self.documents_processed
    }
}

/// Unified processor that handles both media and documents intelligently
pub struct UnifiedProcessor<E, F, P>
where
    E: MetadataParser,
    F: FileManager + Clone,
    P: UserPrompt + Clone,
{
    metadata_parser: E,
    file_manager: F,
    prompter: P,
    life_config: LifeConfig,
}

impl<E, F, P> UnifiedProcessor<E, F, P>
where
    E: MetadataParser,
    F: FileManager + Clone,
    P: UserPrompt + Clone,
{
    pub fn new(metadata_parser: E, file_manager: F, prompter: P, life_config: LifeConfig) -> Self {
        Self {
            metadata_parser,
            file_manager,
            prompter,
            life_config,
        }
    }

    /// Process all files in the life directory inbox with unified workflow
    pub fn process_life_directory(&self) -> Result<UnifiedProcessingResult> {
        println!("Scanning inbox...");

        // Step 1: Scan and categorize files
        let categorized = self.categorize_files()?;

        println!(
            "Found {} media files, {} documents, {} unrecognized files",
            categorized.media_files.len(),
            categorized.document_files.len(),
            categorized.unknown_files.len()
        );

        let mut result = UnifiedProcessingResult::new();

        // Step 2: Process media files automatically
        if !categorized.media_files.is_empty() {
            println!("\nProcessing media files...");
            self.process_media_files(&categorized.media_files, &mut result)?;
        }

        // Step 3: Process documents interactively
        if !categorized.document_files.is_empty() {
            println!("\nProcessing documents:");
            self.process_document_files(&categorized.document_files, &mut result)?;
        }

        // Step 4: Report results
        result.files_skipped = categorized.unknown_files.len();
        if !categorized.unknown_files.is_empty() {
            println!(
                "\n{} unrecognized files remain in inbox.",
                categorized.unknown_files.len()
            );
        }

        Ok(result)
    }

    /// Scan inbox and categorize files by type
    fn categorize_files(&self) -> Result<CategorizedFiles> {
        let mut categorized = CategorizedFiles::new();
        let inbox_path = self.life_config.inbox_path();

        let file_paths = self.file_manager.read_directory(&inbox_path)?;

        for file_path in file_paths {
            if !self.file_manager.is_file(&file_path) {
                continue;
            }

            // Try to parse metadata to determine file type
            match self.metadata_parser.parse_metadata(&file_path) {
                Ok(metadata) => match metadata.file_type {
                    FileType::Image | FileType::Video => {
                        categorized.media_files.push(file_path);
                    }
                    FileType::Document => {
                        categorized.document_files.push(file_path);
                    }
                    FileType::Unknown => {
                        categorized.unknown_files.push(file_path);
                    }
                },
                Err(_) => {
                    // If we can't parse metadata, treat as unknown
                    categorized.unknown_files.push(file_path);
                }
            }
        }

        Ok(categorized)
    }

    /// Process media files using the standard media processing pipeline
    fn process_media_files(
        &self,
        media_files: &[PathBuf],
        result: &mut UnifiedProcessingResult,
    ) -> Result<()> {
        if media_files.is_empty() {
            return Ok(());
        }

        // Create a FileProcessor with appropriate strategies for media processing
        // Note: We create new instances since FileProcessor takes ownership
        let media_processor = FileProcessor::new(
            RexifParser::new(),
            StdFileManager::new(),
            TimestampNamingStrategy::new(),
            MonthlyOrganizer::new(),
            LifeDirectoryResolver::new(),
            self.life_config.to_processing_config(),
        );

        // Process each media file through the standard pipeline
        for (i, file_path) in media_files.iter().enumerate() {
            print!(
                "\r  Processing media file {} of {}...",
                i + 1,
                media_files.len()
            );

            match media_processor.process_single_file(file_path) {
                Ok(()) => {
                    result.media_processed += 1;
                }
                Err(e) => {
                    result.files_failed += 1;
                    let error_msg = format!("{}: {}", file_path.display(), e);
                    result.errors.push(error_msg.clone());

                    // Only log error if it's not something we should skip
                    if !self.should_skip_media_error(&e) {
                        eprintln!("\n  Error processing {}: {}", file_path.display(), e);
                    }
                }
            }
        }

        println!("\r  ✓ Processed {} media files", result.media_processed);

        if result.files_failed > 0 {
            println!("  {} files failed to process", result.files_failed);
        }

        Ok(())
    }

    /// Check if media processing error should be skipped (not logged as error)
    fn should_skip_media_error(&self, error: &CleanboxError) -> bool {
        matches!(
            error,
            CleanboxError::UnsupportedFileType(_) | CleanboxError::Exif(_)
        ) && self.life_config.skip_unsupported_files
    }

    /// Process document files using interactive workflow
    fn process_document_files(
        &self,
        document_files: &[PathBuf],
        result: &mut UnifiedProcessingResult,
    ) -> Result<()> {
        let document_naming = DocumentNamingStrategy::new();
        let document_organizer = DocumentOrganizer::new();

        // Load tag dictionary
        let tag_dict = TagDictionary::load_from_file(self.life_config.tags_file())?;

        // Create document input collector
        let mut document_collector = DocumentInputCollector::new(
            self.prompter.clone(),
            tag_dict,
            self.file_manager.clone(),
            self.life_config.tags_file().to_path_buf(),
        );

        for file_path in document_files.iter() {
            println!(
                "\nFile: {}",
                file_path.file_name().unwrap_or_default().to_string_lossy()
            );

            // Get document input from user
            let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
            let document_input = match document_collector.collect_input(&filename) {
                Ok(input) => input,
                Err(CleanboxError::UserCancelled) => {
                    println!("Processing cancelled by user.");
                    break;
                }
                Err(e) => {
                    result.files_failed += 1;
                    result
                        .errors
                        .push(format!("{}: {}", file_path.display(), e));
                    continue;
                }
            };

            // Process the document
            match self.process_single_document(
                file_path,
                &document_input,
                &document_naming,
                &document_organizer,
            ) {
                Ok(()) => {
                    result.documents_processed += 1;
                }
                Err(e) => {
                    result.files_failed += 1;
                    result
                        .errors
                        .push(format!("{}: {}", file_path.display(), e));
                }
            }
        }

        // Save updated tag dictionary after processing all documents
        document_collector.save_tag_dictionary(&self.life_config.tags_file())?;

        Ok(())
    }

    /// Process a single document file
    fn process_single_document(
        &self,
        file_path: &Path,
        document_input: &crate::document::DocumentInput,
        naming_strategy: &DocumentNamingStrategy,
        organizer: &DocumentOrganizer,
    ) -> Result<()> {
        // Get file extension
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                CleanboxError::InvalidFileExtension(format!(
                    "File has no extension: {}",
                    file_path.display()
                ))
            })?;

        // Generate new filename
        let new_name = naming_strategy.generate_name_from_input(document_input, extension)?;

        // Determine target directory
        let documents_base = self.life_config.documents_root();
        let target_dir =
            organizer.determine_target_directory_from_input(document_input, &documents_base)?;

        // Ensure target directory exists
        self.file_manager.create_directories(&target_dir)?;

        let mut target_path = target_dir.join(&new_name);

        // Handle duplicates if file already exists
        if self.file_manager.file_exists(&target_path) {
            target_path = self.handle_document_duplicate(file_path, &target_path)?;
        }

        // Move the file
        self.file_manager.move_file(file_path, &target_path)?;
        println!("  → {}", target_path.display());

        Ok(())
    }

    /// Handle duplicate document files by appending hash
    fn handle_document_duplicate(&self, source_path: &Path, target_path: &Path) -> Result<PathBuf> {
        match self.life_config.handle_duplicates {
            DuplicateHandling::Skip => Err(CleanboxError::FileAlreadyExists(
                target_path.display().to_string(),
            )),
            DuplicateHandling::Overwrite => Ok(target_path.to_path_buf()),
            DuplicateHandling::Error => Err(CleanboxError::FileAlreadyExists(
                target_path.display().to_string(),
            )),
            DuplicateHandling::AppendHash => {
                let hash = self.file_manager.calculate_file_hash(source_path)?;
                let hash_suffix =
                    FileHasher::generate_hash_suffix(&hash, self.life_config.hash_length);

                let original_name = target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| CleanboxError::InvalidPath(target_path.display().to_string()))?;

                let new_name = FileHasher::append_hash_to_filename(original_name, &hash_suffix)?;
                Ok(target_path.with_file_name(new_name))
            }
        }
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
            matches!(
                file_type,
                crate::media::FileType::Image | crate::media::FileType::Video
            )
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
        fn determine_target_directory(&self, _file: &File, base_path: &Path) -> Result<PathBuf> {
            Ok(base_path.join(&self.directory))
        }
    }

    fn create_test_processor() -> FileProcessor<
        MockExifParser,
        MockFileManager,
        MockNamingStrategy,
        MockOrganizationStrategy,
        crate::paths::LifeDirectoryResolver,
    > {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"));

        FileProcessor::new(
            MockExifParser::new(),
            MockFileManager::new(),
            MockNamingStrategy::new("test_file.jpg".to_string()),
            MockOrganizationStrategy::new(PathBuf::from("2023/12")),
            crate::paths::LifeDirectoryResolver::new(),
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
            crate::paths::LifeDirectoryResolver::new(),
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
            crate::paths::LifeDirectoryResolver::new(),
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
            crate::paths::LifeDirectoryResolver::new(),
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
