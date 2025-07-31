use crate::error::{CleanboxError, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Image,
    Video,
    Document,
    Unknown,
}

impl FileType {
    pub fn from_mime(mime: &str) -> Self {
        let mime_lower = mime.to_lowercase();
        if mime_lower.starts_with("image/") {
            FileType::Image
        } else if mime_lower.starts_with("video/") {
            FileType::Video
        } else if mime_lower.starts_with("application/pdf")
            || mime_lower.starts_with("application/msword")
            || mime_lower.starts_with("application/vnd.openxmlformats")
            || mime_lower.starts_with("text/") {
            FileType::Document
        } else {
            FileType::Unknown
        }
    }

    pub fn is_supported(&self) -> bool {
        matches!(self, FileType::Image | FileType::Video | FileType::Document)
    }

    pub fn needs_interactive_processing(&self) -> bool {
        matches!(self, FileType::Document)
    }

    pub fn is_auto_processable(&self) -> bool {
        matches!(self, FileType::Image | FileType::Video)
    }

    pub fn should_skip(&self) -> bool {
        matches!(self, FileType::Unknown)
    }

    pub fn base_directory_name(&self) -> Option<&'static str> {
        match self {
            FileType::Image | FileType::Video => Some("media"),
            FileType::Document => Some("documents"),
            FileType::Unknown => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub datetime_original: Option<String>,
    pub file_type: FileType,
    pub mime_type: String,
    pub file_hash: Option<String>,
}

impl FileMetadata {
    pub fn new(mime_type: String) -> Self {
        let file_type = FileType::from_mime(&mime_type);
        Self {
            datetime_original: None,
            file_type,
            mime_type,
            file_hash: None,
        }
    }

    pub fn with_datetime(mut self, datetime: String) -> Self {
        self.datetime_original = Some(datetime);
        self
    }

    pub fn with_hash(mut self, hash: String) -> Self {
        self.file_hash = Some(hash);
        self
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub metadata: Option<FileMetadata>,
}

impl File {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: FileMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn file_name(&self) -> Result<&str> {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| CleanboxError::InvalidPath(self.path.display().to_string()))
    }

    pub fn file_stem(&self) -> Result<&str> {
        self.path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| CleanboxError::InvalidFileStem(self.path.display().to_string()))
    }

    pub fn extension(&self) -> Result<&str> {
        self.path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| CleanboxError::InvalidFileExtension(self.path.display().to_string()))
    }

    pub fn parent_dir(&self) -> Option<&Path> {
        self.path.parent()
    }

    pub fn is_supported_media(&self) -> bool {
        self.metadata
            .as_ref()
            .map(|m| m.file_type.is_supported())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_file_type_from_mime() {
        assert_eq!(FileType::from_mime("image/jpeg"), FileType::Image);
        assert_eq!(FileType::from_mime("IMAGE/PNG"), FileType::Image);
        assert_eq!(FileType::from_mime("video/mp4"), FileType::Video);
        assert_eq!(FileType::from_mime("VIDEO/MOV"), FileType::Video);
        assert_eq!(FileType::from_mime("application/pdf"), FileType::Document);
        assert_eq!(FileType::from_mime("application/msword"), FileType::Document);
        assert_eq!(FileType::from_mime("application/vnd.openxmlformats-wordprocessingml.document"), FileType::Document);
        assert_eq!(FileType::from_mime("text/plain"), FileType::Document);
        assert_eq!(FileType::from_mime("text/csv"), FileType::Document);
        assert_eq!(FileType::from_mime("application/unknown"), FileType::Unknown);
        assert_eq!(FileType::from_mime(""), FileType::Unknown);
    }

    #[test]
    fn test_file_type_is_supported() {
        assert!(FileType::Image.is_supported());
        assert!(FileType::Video.is_supported());
        assert!(FileType::Document.is_supported());
        assert!(!FileType::Unknown.is_supported());
    }

    #[test]
    fn test_file_type_needs_interactive_processing() {
        assert!(!FileType::Image.needs_interactive_processing());
        assert!(!FileType::Video.needs_interactive_processing());
        assert!(FileType::Document.needs_interactive_processing());
        assert!(!FileType::Unknown.needs_interactive_processing());
    }

    #[test]
    fn test_file_type_is_auto_processable() {
        assert!(FileType::Image.is_auto_processable());
        assert!(FileType::Video.is_auto_processable());
        assert!(!FileType::Document.is_auto_processable());
        assert!(!FileType::Unknown.is_auto_processable());
    }

    #[test]
    fn test_file_type_should_skip() {
        assert!(!FileType::Image.should_skip());
        assert!(!FileType::Video.should_skip());
        assert!(!FileType::Document.should_skip());
        assert!(FileType::Unknown.should_skip());
    }

    #[test]
    fn test_file_type_base_directory_name() {
        assert_eq!(FileType::Image.base_directory_name(), Some("media"));
        assert_eq!(FileType::Video.base_directory_name(), Some("media"));
        assert_eq!(FileType::Document.base_directory_name(), Some("documents"));
        assert_eq!(FileType::Unknown.base_directory_name(), None);
    }

    #[test]
    fn test_file_metadata_creation() {
        let metadata = FileMetadata::new("image/jpeg".to_string());
        assert_eq!(metadata.file_type, FileType::Image);
        assert_eq!(metadata.mime_type, "image/jpeg");
        assert!(metadata.datetime_original.is_none());
        assert!(metadata.file_hash.is_none());
    }

    #[test]
    fn test_file_metadata_with_datetime() {
        let metadata = FileMetadata::new("image/jpeg".to_string())
            .with_datetime("2023-12-01_14-30-00".to_string());
        assert_eq!(
            metadata.datetime_original,
            Some("2023-12-01_14-30-00".to_string())
        );
    }

    #[test]
    fn test_file_metadata_with_hash() {
        let metadata = FileMetadata::new("image/jpeg".to_string()).with_hash("abc123".to_string());
        assert_eq!(metadata.file_hash, Some("abc123".to_string()));
    }

    #[test]
    fn test_file_creation() {
        let path = PathBuf::from("/test/image.jpg");
        let file = File::new(&path);
        assert_eq!(file.path, path);
        assert!(file.metadata.is_none());
    }

    #[test]
    fn test_file_with_metadata() {
        let path = PathBuf::from("/test/image.jpg");
        let metadata = FileMetadata::new("image/jpeg".to_string());
        let file = File::new(&path).with_metadata(metadata.clone());
        assert!(file.metadata.is_some());
        assert_eq!(file.metadata.unwrap().mime_type, "image/jpeg");
    }

    #[test]
    fn test_file_file_name() {
        let path = PathBuf::from("/test/image.jpg");
        let file = File::new(&path);
        assert_eq!(file.file_name().unwrap(), "image.jpg");
    }

    #[test]
    fn test_file_file_stem() {
        let path = PathBuf::from("/test/image.jpg");
        let file = File::new(&path);
        assert_eq!(file.file_stem().unwrap(), "image");
    }

    #[test]
    fn test_file_extension() {
        let path = PathBuf::from("/test/image.jpg");
        let file = File::new(&path);
        assert_eq!(file.extension().unwrap(), "jpg");
    }

    #[test]
    fn test_file_is_supported_media() {
        let path = PathBuf::from("/test/image.jpg");
        let mut file = File::new(&path);
        assert!(!file.is_supported_media());

        let metadata = FileMetadata::new("image/jpeg".to_string());
        file = file.with_metadata(metadata);
        assert!(file.is_supported_media());

        let document_metadata = FileMetadata::new("application/pdf".to_string());
        file = File::new(&path).with_metadata(document_metadata);
        assert!(file.is_supported_media());

        let unsupported_metadata = FileMetadata::new("application/unknown".to_string());
        file = File::new(&path).with_metadata(unsupported_metadata);
        assert!(!file.is_supported_media());
    }

    #[test]
    fn test_file_invalid_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Create an invalid UTF-8 path
        let invalid_bytes = b"\xFF\xFE";
        let invalid_os_str = OsStr::from_bytes(invalid_bytes);
        let invalid_path = PathBuf::from(invalid_os_str);
        let file = File::new(&invalid_path);

        assert!(file.file_name().is_err());
        assert!(file.file_stem().is_err());
        assert!(file.extension().is_err());
    }
}
