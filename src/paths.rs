use crate::config::{LifeConfig, ProcessingConfig};
use crate::media::FileType;
use std::path::PathBuf;

pub trait BasePathResolver {
    fn resolve_base_path(&self, file_type: &FileType, config: &ProcessingConfig) -> PathBuf;
}

pub trait LifePathResolver {
    fn resolve_base_path(&self, file_type: &FileType, config: &LifeConfig) -> PathBuf;
}

pub struct LifeDirectoryResolver;

impl LifeDirectoryResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LifeDirectoryResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BasePathResolver for LifeDirectoryResolver {
    fn resolve_base_path(&self, file_type: &FileType, config: &ProcessingConfig) -> PathBuf {
        match file_type.base_directory_name() {
            Some("media") => config.media_root.clone(),
            Some("documents") => {
                // For ProcessingConfig compatibility, derive from media_root parent
                config
                    .media_root
                    .parent()
                    .unwrap_or(&config.media_root)
                    .join("documents")
            }
            _ => config.media_root.clone(), // Fallback to media_root
        }
    }
}

impl LifePathResolver for LifeDirectoryResolver {
    fn resolve_base_path(&self, file_type: &FileType, config: &LifeConfig) -> PathBuf {
        match file_type.base_directory_name() {
            Some("media") => config.media_root(),
            Some("documents") => config.documents_root(),
            _ => config.media_root(), // Fallback to media_root
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProcessingConfig;
    use std::path::PathBuf;

    #[test]
    fn test_life_directory_resolver_media_files() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/life/media"));
        let resolver = LifeDirectoryResolver::new();

        assert_eq!(
            BasePathResolver::resolve_base_path(&resolver, &FileType::Image, &config),
            PathBuf::from("/life/media")
        );
        assert_eq!(
            BasePathResolver::resolve_base_path(&resolver, &FileType::Video, &config),
            PathBuf::from("/life/media")
        );
    }

    #[test]
    fn test_life_directory_resolver_document_files() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/life/media"));
        let resolver = LifeDirectoryResolver::new();

        assert_eq!(
            BasePathResolver::resolve_base_path(&resolver, &FileType::Document, &config),
            PathBuf::from("/life/documents")
        );
    }

    #[test]
    fn test_life_directory_resolver_unknown_files() {
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/life/media"));
        let resolver = LifeDirectoryResolver::new();

        // Unknown files fallback to media_root
        assert_eq!(
            BasePathResolver::resolve_base_path(&resolver, &FileType::Unknown, &config),
            PathBuf::from("/life/media")
        );
    }

    #[test]
    fn test_life_directory_resolver_default() {
        let resolver = LifeDirectoryResolver::default();
        let config = ProcessingConfig::new(PathBuf::from("/inbox"), PathBuf::from("/test/media"));

        assert_eq!(
            BasePathResolver::resolve_base_path(&resolver, &FileType::Image, &config),
            PathBuf::from("/test/media")
        );
    }

    #[test]
    fn test_life_path_resolver_media_files() {
        let resolver = LifeDirectoryResolver::new();
        let config = LifeConfig::new(PathBuf::from("/home/user/life"));

        assert_eq!(
            LifePathResolver::resolve_base_path(&resolver, &FileType::Image, &config),
            PathBuf::from("/home/user/life/media")
        );
        assert_eq!(
            LifePathResolver::resolve_base_path(&resolver, &FileType::Video, &config),
            PathBuf::from("/home/user/life/media")
        );
    }

    #[test]
    fn test_life_path_resolver_document_files() {
        let resolver = LifeDirectoryResolver::new();
        let config = LifeConfig::new(PathBuf::from("/home/user/life"));

        assert_eq!(
            LifePathResolver::resolve_base_path(&resolver, &FileType::Document, &config),
            PathBuf::from("/home/user/life/documents")
        );
    }

    #[test]
    fn test_life_path_resolver_unknown_files() {
        let resolver = LifeDirectoryResolver::new();
        let config = LifeConfig::new(PathBuf::from("/home/user/life"));

        // Unknown files fallback to media_root
        assert_eq!(
            LifePathResolver::resolve_base_path(&resolver, &FileType::Unknown, &config),
            PathBuf::from("/home/user/life/media")
        );
    }
}
