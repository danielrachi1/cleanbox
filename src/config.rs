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
}
