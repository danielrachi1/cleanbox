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
        // First, detect MIME type using infer
        let mime_type = if let Some(kind) =
            infer::get_from_path(&file_path).map_err(CleanboxError::Io)?
        {
            kind.mime_type().to_string()
        } else {
            // Fallback to basic detection based on extension
            let path = file_path.as_ref();
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("txt") => "text/plain".to_string(),
                Some("pdf") => "application/pdf".to_string(),
                Some("doc") => "application/msword".to_string(),
                Some("docx") => {
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                        .to_string()
                }
                _ => "application/octet-stream".to_string(),
            }
        };

        // Create metadata with detected MIME type
        let mut metadata = FileMetadata::new(mime_type.clone());

        // Only attempt EXIF parsing for image/video files
        if mime_type.starts_with("image/") || mime_type.starts_with("video/") {
            // Try to extract datetime from EXIF data
            if let Ok(datetime) = self.extract_datetime(&file_path) {
                metadata = metadata.with_datetime(datetime);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::FileType;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_metadata_pdf_document() {
        let parser = RexifParser::new();

        // Create a temporary PDF-like file
        let mut temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        temp_file.write_all(b"%PDF-1.4\n%Test PDF content").unwrap();

        let result = parser.parse_metadata(temp_file.path()).unwrap();

        assert_eq!(result.file_type, FileType::Document);
        assert!(result.mime_type.contains("pdf") || result.mime_type == "application/pdf");
        assert!(result.datetime_original.is_none()); // Documents don't have EXIF datetime
    }

    #[test]
    fn test_parse_metadata_txt_document() {
        let parser = RexifParser::new();

        // Create a temporary TXT file
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        temp_file
            .write_all(b"This is a plain text document for testing")
            .unwrap();

        let result = parser.parse_metadata(temp_file.path()).unwrap();

        assert_eq!(result.file_type, FileType::Document);
        assert!(result.mime_type.contains("text") || result.mime_type == "text/plain");
        assert!(result.datetime_original.is_none()); // Documents don't have EXIF datetime
    }

    #[test]
    fn test_parse_metadata_docx_document() {
        let parser = RexifParser::new();

        // Create a temporary DOCX file with content that won't be recognized by infer
        // This tests the fallback extension-based detection
        let mut temp_file = NamedTempFile::with_suffix(".docx").unwrap();
        temp_file
            .write_all(b"not a real docx but has docx extension")
            .unwrap();

        let result = parser.parse_metadata(temp_file.path()).unwrap();

        // Should use fallback extension-based detection for .docx files
        assert_eq!(result.file_type, FileType::Document);
        assert_eq!(
            result.mime_type,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        );
    }

    #[test]
    fn test_parse_metadata_fallback_extension_detection() {
        let parser = RexifParser::new();

        // Create a file with document extension but no recognizable content
        let mut temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        temp_file
            .write_all(b"not actually a pdf but has pdf extension")
            .unwrap();

        let result = parser.parse_metadata(temp_file.path()).unwrap();

        // Should use fallback extension-based detection
        assert_eq!(result.file_type, FileType::Document);
    }

    #[test]
    fn test_parse_metadata_unknown_extension() {
        let parser = RexifParser::new();

        // Create a file with unknown extension
        let mut temp_file = NamedTempFile::with_suffix(".xyz").unwrap();
        temp_file.write_all(b"unknown file type").unwrap();

        let result = parser.parse_metadata(temp_file.path()).unwrap();

        // Should be classified as Unknown since it's not a recognized type
        assert_eq!(result.file_type, FileType::Unknown);
        assert_eq!(result.mime_type, "application/octet-stream");
    }

    #[test]
    fn test_image_files_still_work() {
        let parser = RexifParser::new();

        // Create a simple JPEG-like file (won't have valid EXIF but should be detected as image)
        let mut temp_file = NamedTempFile::with_suffix(".jpg").unwrap();
        temp_file.write_all(&[0xFF, 0xD8, 0xFF, 0xE0]).unwrap(); // JPEG header

        let result = parser.parse_metadata(temp_file.path()).unwrap();

        assert_eq!(result.file_type, FileType::Image);
        assert!(result.mime_type.starts_with("image/"));
        // datetime_original will be None since this isn't a real JPEG with EXIF
    }

    #[test]
    fn test_supports_file_type_unchanged() {
        let parser = RexifParser::new();

        assert!(parser.supports_file_type(&FileType::Image));
        assert!(parser.supports_file_type(&FileType::Video));
        assert!(!parser.supports_file_type(&FileType::Document));
        assert!(!parser.supports_file_type(&FileType::Unknown));
    }

    #[test]
    fn test_format_datetime_unchanged() {
        let result = RexifParser::format_datetime("2023:07:15 14:30:25").unwrap();
        assert_eq!(result, "2023-07-15_14-30-25");
    }
}
