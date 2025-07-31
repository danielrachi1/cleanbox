use crate::error::{CleanboxError, Result};
use crate::media::{FileMetadata, FileType};
use std::path::Path;

pub trait MetadataParser {
    fn parse_metadata<P: AsRef<Path>>(&self, file_path: P) -> Result<FileMetadata>;
    fn extract_datetime<P: AsRef<Path>>(&self, file_path: P) -> Result<String>;
    fn supports_file_type(&self, file_type: &FileType) -> bool;
}

pub struct RexifParser;

impl RexifParser {
    pub fn new() -> Self {
        Self
    }

    fn format_datetime(raw_datetime: &str) -> Result<String> {
        let parts: Vec<&str> = raw_datetime.trim().split(' ').collect();
        if parts.len() != 2 {
            return Err(CleanboxError::InvalidDateTime(
                "Expected format 'YYYY:MM:DD HH:MM:SS'".to_string(),
            ));
        }

        let date = parts[0].replace(":", "-");
        let time = parts[1].replace(":", "-");
        Ok(format!("{date}_{time}"))
    }
}

impl Default for RexifParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataParser for RexifParser {
    fn parse_metadata<P: AsRef<Path>>(&self, file_path: P) -> Result<FileMetadata> {
        let path_str = file_path
            .as_ref()
            .to_str()
            .ok_or_else(|| CleanboxError::InvalidPath(file_path.as_ref().display().to_string()))?;

        let exif = rexif::parse_file(path_str)?;

        let mut metadata = FileMetadata::new(exif.mime.to_string());

        if let Ok(datetime) = self.extract_datetime(&file_path) {
            metadata = metadata.with_datetime(datetime);
        }

        Ok(metadata)
    }

    fn extract_datetime<P: AsRef<Path>>(&self, file_path: P) -> Result<String> {
        let path_str = file_path
            .as_ref()
            .to_str()
            .ok_or_else(|| CleanboxError::InvalidPath(file_path.as_ref().display().to_string()))?;

        let exif = rexif::parse_file(path_str)?;

        for entry in &exif.entries {
            if entry.tag == rexif::ExifTag::DateTimeOriginal {
                return Self::format_datetime(&entry.value_more_readable);
            }
        }

        Err(CleanboxError::Exif(
            "DateTimeOriginal tag not found".to_string(),
        ))
    }

    fn supports_file_type(&self, file_type: &FileType) -> bool {
        matches!(file_type, FileType::Image | FileType::Video)
    }
}
