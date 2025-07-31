use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    pub inbox_path: PathBuf,
    pub media_root: PathBuf,
    pub hash_length: usize,
    pub handle_duplicates: DuplicateHandling,
    pub skip_unsupported_files: bool,
    pub create_backup: bool,
}

#[derive(Debug, Clone)]
pub struct LifeConfig {
    pub life_path: PathBuf,
    pub hash_length: usize,
    pub handle_duplicates: DuplicateHandling,
    pub skip_unsupported_files: bool,
    pub create_backup: bool,
}

#[derive(Debug, Clone)]
pub enum DuplicateHandling {
    Skip,
    AppendHash,
    Overwrite,
    Error,
}

impl ProcessingConfig {
    pub fn new(inbox_path: PathBuf, media_root: PathBuf) -> Self {
        Self {
            inbox_path,
            media_root,
            hash_length: 6,
            handle_duplicates: DuplicateHandling::AppendHash,
            skip_unsupported_files: true,
            create_backup: false,
        }
    }

    pub fn with_hash_length(mut self, length: usize) -> Self {
        self.hash_length = length;
        self
    }

    pub fn with_duplicate_handling(mut self, handling: DuplicateHandling) -> Self {
        self.handle_duplicates = handling;
        self
    }

    pub fn with_backup(mut self, create_backup: bool) -> Self {
        self.create_backup = create_backup;
        self
    }

    pub fn skip_unsupported(mut self, skip: bool) -> Self {
        self.skip_unsupported_files = skip;
        self
    }
}

impl LifeConfig {
    pub fn new(life_path: PathBuf) -> Self {
        Self {
            life_path,
            hash_length: 6,
            handle_duplicates: DuplicateHandling::AppendHash,
            skip_unsupported_files: true,
            create_backup: false,
        }
    }

    pub fn inbox_path(&self) -> PathBuf {
        self.life_path.join("inbox")
    }

    pub fn media_root(&self) -> PathBuf {
        self.life_path.join("media")
    }

    pub fn documents_root(&self) -> PathBuf {
        self.life_path.join("documents")
    }

    pub fn tags_file(&self) -> PathBuf {
        self.documents_root().join("tags.txt")
    }

    pub fn with_hash_length(mut self, length: usize) -> Self {
        self.hash_length = length;
        self
    }

    pub fn with_duplicate_handling(mut self, handling: DuplicateHandling) -> Self {
        self.handle_duplicates = handling;
        self
    }

    pub fn with_backup(mut self, create_backup: bool) -> Self {
        self.create_backup = create_backup;
        self
    }

    pub fn skip_unsupported(mut self, skip: bool) -> Self {
        self.skip_unsupported_files = skip;
        self
    }

    // Convenience method to convert to ProcessingConfig for compatibility
    pub fn to_processing_config(&self) -> ProcessingConfig {
        ProcessingConfig {
            inbox_path: self.inbox_path(),
            media_root: self.media_root(),
            hash_length: self.hash_length,
            handle_duplicates: self.handle_duplicates.clone(),
            skip_unsupported_files: self.skip_unsupported_files,
            create_backup: self.create_backup,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_processing_config_new() {
        let inbox = PathBuf::from("/inbox");
        let media = PathBuf::from("/media");
        let config = ProcessingConfig::new(inbox.clone(), media.clone());

        assert_eq!(config.inbox_path, inbox);
        assert_eq!(config.media_root, media);
        assert_eq!(config.hash_length, 6);
        assert!(matches!(
            config.handle_duplicates,
            DuplicateHandling::AppendHash
        ));
        assert!(config.skip_unsupported_files);
        assert!(!config.create_backup);
    }

    #[test]
    fn test_processing_config_with_hash_length() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"))
            .with_hash_length(8);

        assert_eq!(config.hash_length, 8);
    }

    #[test]
    fn test_processing_config_with_duplicate_handling() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"))
            .with_duplicate_handling(DuplicateHandling::Skip);

        assert!(matches!(config.handle_duplicates, DuplicateHandling::Skip));
    }

    #[test]
    fn test_processing_config_with_backup() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"))
            .with_backup(true);

        assert!(config.create_backup);
    }

    #[test]
    fn test_processing_config_skip_unsupported() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"))
            .skip_unsupported(false);

        assert!(!config.skip_unsupported_files);
    }

    #[test]
    fn test_processing_config_chaining() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/media"))
            .with_hash_length(10)
            .with_duplicate_handling(DuplicateHandling::Overwrite)
            .with_backup(true)
            .skip_unsupported(false);

        assert_eq!(config.hash_length, 10);
        assert!(matches!(
            config.handle_duplicates,
            DuplicateHandling::Overwrite
        ));
        assert!(config.create_backup);
        assert!(!config.skip_unsupported_files);
    }

    #[test]
    fn test_duplicate_handling_variants() {
        // Test that all variants can be created and matched
        let skip = DuplicateHandling::Skip;
        let append = DuplicateHandling::AppendHash;
        let overwrite = DuplicateHandling::Overwrite;
        let error = DuplicateHandling::Error;

        match skip {
            DuplicateHandling::Skip => {}
            _ => panic!("Wrong variant"),
        }

        match append {
            DuplicateHandling::AppendHash => {}
            _ => panic!("Wrong variant"),
        }

        match overwrite {
            DuplicateHandling::Overwrite => {}
            _ => panic!("Wrong variant"),
        }

        match error {
            DuplicateHandling::Error => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_life_config_new() {
        let life_path = PathBuf::from("/home/user/life");
        let config = LifeConfig::new(life_path.clone());

        assert_eq!(config.life_path, life_path);
        assert_eq!(config.hash_length, 6);
        assert!(matches!(
            config.handle_duplicates,
            DuplicateHandling::AppendHash
        ));
        assert!(config.skip_unsupported_files);
        assert!(!config.create_backup);
    }

    #[test]
    fn test_life_config_derived_paths() {
        let life_path = PathBuf::from("/home/user/life");
        let config = LifeConfig::new(life_path);

        assert_eq!(config.inbox_path(), PathBuf::from("/home/user/life/inbox"));
        assert_eq!(config.media_root(), PathBuf::from("/home/user/life/media"));
        assert_eq!(config.documents_root(), PathBuf::from("/home/user/life/documents"));
        assert_eq!(config.tags_file(), PathBuf::from("/home/user/life/documents/tags.txt"));
    }

    #[test]
    fn test_life_config_builder_methods() {
        let config = LifeConfig::new(PathBuf::from("/life"))
            .with_hash_length(8)
            .with_duplicate_handling(DuplicateHandling::Skip)
            .with_backup(true)
            .skip_unsupported(false);

        assert_eq!(config.hash_length, 8);
        assert!(matches!(config.handle_duplicates, DuplicateHandling::Skip));
        assert!(config.create_backup);
        assert!(!config.skip_unsupported_files);
    }

    #[test]
    fn test_life_config_to_processing_config() {
        let life_config = LifeConfig::new(PathBuf::from("/home/user/life"))
            .with_hash_length(10)
            .with_duplicate_handling(DuplicateHandling::Overwrite);

        let processing_config = life_config.to_processing_config();

        assert_eq!(processing_config.inbox_path, PathBuf::from("/home/user/life/inbox"));
        assert_eq!(processing_config.media_root, PathBuf::from("/home/user/life/media"));
        assert_eq!(processing_config.hash_length, 10);
        assert!(matches!(
            processing_config.handle_duplicates,
            DuplicateHandling::Overwrite
        ));
    }
}
