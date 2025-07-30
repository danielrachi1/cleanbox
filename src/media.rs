use crate::error::{CleanboxError, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum MediaType {
    Image,
    Video,
    Unknown,
}

impl MediaType {
    pub fn from_mime(mime: &str) -> Self {
        let mime_lower = mime.to_lowercase();
        if mime_lower.starts_with("image/") {
            MediaType::Image
        } else if mime_lower.starts_with("video/") {
            MediaType::Video
        } else {
            MediaType::Unknown
        }
    }

    pub fn is_supported(&self) -> bool {
        matches!(self, MediaType::Image | MediaType::Video)
    }
}

#[derive(Debug, Clone)]
pub struct MediaMetadata {
    pub datetime_original: Option<String>,
    pub media_type: MediaType,
    pub mime_type: String,
    pub file_hash: Option<String>,
}

impl MediaMetadata {
    pub fn new(mime_type: String) -> Self {
        let media_type = MediaType::from_mime(&mime_type);
        Self {
            datetime_original: None,
            media_type,
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
pub struct MediaFile {
    pub path: PathBuf,
    pub metadata: Option<MediaMetadata>,
}

impl MediaFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: MediaMetadata) -> Self {
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
            .map(|m| m.media_type.is_supported())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_media_type_from_mime() {
        assert_eq!(MediaType::from_mime("image/jpeg"), MediaType::Image);
        assert_eq!(MediaType::from_mime("IMAGE/PNG"), MediaType::Image);
        assert_eq!(MediaType::from_mime("video/mp4"), MediaType::Video);
        assert_eq!(MediaType::from_mime("VIDEO/MOV"), MediaType::Video);
        assert_eq!(MediaType::from_mime("text/plain"), MediaType::Unknown);
        assert_eq!(MediaType::from_mime(""), MediaType::Unknown);
    }

    #[test]
    fn test_media_type_is_supported() {
        assert!(MediaType::Image.is_supported());
        assert!(MediaType::Video.is_supported());
        assert!(!MediaType::Unknown.is_supported());
    }

    #[test]
    fn test_media_metadata_creation() {
        let metadata = MediaMetadata::new("image/jpeg".to_string());
        assert_eq!(metadata.media_type, MediaType::Image);
        assert_eq!(metadata.mime_type, "image/jpeg");
        assert!(metadata.datetime_original.is_none());
        assert!(metadata.file_hash.is_none());
    }

    #[test]
    fn test_media_metadata_with_datetime() {
        let metadata = MediaMetadata::new("image/jpeg".to_string())
            .with_datetime("2023-12-01_14-30-00".to_string());
        assert_eq!(
            metadata.datetime_original,
            Some("2023-12-01_14-30-00".to_string())
        );
    }

    #[test]
    fn test_media_metadata_with_hash() {
        let metadata = MediaMetadata::new("image/jpeg".to_string()).with_hash("abc123".to_string());
        assert_eq!(metadata.file_hash, Some("abc123".to_string()));
    }

    #[test]
    fn test_media_file_creation() {
        let path = PathBuf::from("/test/image.jpg");
        let media_file = MediaFile::new(&path);
        assert_eq!(media_file.path, path);
        assert!(media_file.metadata.is_none());
    }

    #[test]
    fn test_media_file_with_metadata() {
        let path = PathBuf::from("/test/image.jpg");
        let metadata = MediaMetadata::new("image/jpeg".to_string());
        let media_file = MediaFile::new(&path).with_metadata(metadata.clone());
        assert!(media_file.metadata.is_some());
        assert_eq!(media_file.metadata.unwrap().mime_type, "image/jpeg");
    }

    #[test]
    fn test_media_file_file_name() {
        let path = PathBuf::from("/test/image.jpg");
        let media_file = MediaFile::new(&path);
        assert_eq!(media_file.file_name().unwrap(), "image.jpg");
    }

    #[test]
    fn test_media_file_file_stem() {
        let path = PathBuf::from("/test/image.jpg");
        let media_file = MediaFile::new(&path);
        assert_eq!(media_file.file_stem().unwrap(), "image");
    }

    #[test]
    fn test_media_file_extension() {
        let path = PathBuf::from("/test/image.jpg");
        let media_file = MediaFile::new(&path);
        assert_eq!(media_file.extension().unwrap(), "jpg");
    }

    #[test]
    fn test_media_file_is_supported_media() {
        let path = PathBuf::from("/test/image.jpg");
        let mut media_file = MediaFile::new(&path);
        assert!(!media_file.is_supported_media());

        let metadata = MediaMetadata::new("image/jpeg".to_string());
        media_file = media_file.with_metadata(metadata);
        assert!(media_file.is_supported_media());

        let unsupported_metadata = MediaMetadata::new("text/plain".to_string());
        media_file = MediaFile::new(&path).with_metadata(unsupported_metadata);
        assert!(!media_file.is_supported_media());
    }

    #[test]
    fn test_media_file_invalid_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Create an invalid UTF-8 path
        let invalid_bytes = b"\xFF\xFE";
        let invalid_os_str = OsStr::from_bytes(invalid_bytes);
        let invalid_path = PathBuf::from(invalid_os_str);
        let media_file = MediaFile::new(&invalid_path);

        assert!(media_file.file_name().is_err());
        assert!(media_file.file_stem().is_err());
        assert!(media_file.extension().is_err());
    }
}
