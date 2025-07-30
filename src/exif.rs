use crate::error::{CleanboxError, Result};
use crate::media::MediaMetadata;
use std::path::Path;

pub trait ExifParser {
    fn parse_metadata<P: AsRef<Path>>(&self, file_path: P) -> Result<MediaMetadata>;
    fn extract_datetime<P: AsRef<Path>>(&self, file_path: P) -> Result<String>;
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

impl ExifParser for RexifParser {
    fn parse_metadata<P: AsRef<Path>>(&self, file_path: P) -> Result<MediaMetadata> {
        let path_str = file_path
            .as_ref()
            .to_str()
            .ok_or_else(|| CleanboxError::InvalidPath(file_path.as_ref().display().to_string()))?;

        let exif = rexif::parse_file(path_str)?;

        let mut metadata = MediaMetadata::new(exif.mime.to_string());

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
}
