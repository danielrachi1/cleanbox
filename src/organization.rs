use crate::error::{CleanboxError, Result};
use crate::media::MediaFile;
use std::path::{Path, PathBuf};

pub trait OrganizationStrategy {
    fn determine_target_directory(
        &self,
        media_file: &MediaFile,
        base_path: &Path,
    ) -> Result<PathBuf>;
}

pub struct MonthlyOrganizer;

impl MonthlyOrganizer {
    pub fn new() -> Self {
        Self
    }

    fn parse_datetime_parts(datetime: &str) -> Result<(String, String)> {
        let date_part = datetime
            .split('_')
            .next()
            .ok_or_else(|| CleanboxError::InvalidDateTime("Invalid datetime format".to_string()))?;

        let mut date_split = date_part.split('-');
        let year = date_split.next().ok_or_else(|| {
            CleanboxError::InvalidDateTime("Invalid date format - missing year".to_string())
        })?;
        let month = date_split.next().ok_or_else(|| {
            CleanboxError::InvalidDateTime("Invalid date format - missing month".to_string())
        })?;

        Ok((year.to_string(), month.to_string()))
    }
}

impl Default for MonthlyOrganizer {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationStrategy for MonthlyOrganizer {
    fn determine_target_directory(
        &self,
        media_file: &MediaFile,
        base_path: &Path,
    ) -> Result<PathBuf> {
        let datetime = media_file
            .metadata
            .as_ref()
            .and_then(|m| m.datetime_original.as_ref())
            .ok_or_else(|| {
                CleanboxError::Exif("No datetime available for organization".to_string())
            })?;

        let (year, month) = Self::parse_datetime_parts(datetime)?;
        Ok(base_path.join(year).join(month))
    }
}

pub struct FlatOrganizer;

impl FlatOrganizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FlatOrganizer {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationStrategy for FlatOrganizer {
    fn determine_target_directory(
        &self,
        _media_file: &MediaFile,
        base_path: &Path,
    ) -> Result<PathBuf> {
        Ok(base_path.to_path_buf())
    }
}

pub struct YearlyOrganizer;

impl YearlyOrganizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for YearlyOrganizer {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationStrategy for YearlyOrganizer {
    fn determine_target_directory(
        &self,
        media_file: &MediaFile,
        base_path: &Path,
    ) -> Result<PathBuf> {
        let datetime = media_file
            .metadata
            .as_ref()
            .and_then(|m| m.datetime_original.as_ref())
            .ok_or_else(|| {
                CleanboxError::Exif("No datetime available for organization".to_string())
            })?;

        let (year, _) = MonthlyOrganizer::parse_datetime_parts(datetime)?;
        Ok(base_path.join(year))
    }
}

pub struct CustomOrganizer {
    pattern: String,
}

impl CustomOrganizer {
    pub fn new(pattern: String) -> Self {
        Self { pattern }
    }

    fn replace_placeholders(&self, media_file: &MediaFile, base_path: &Path) -> Result<PathBuf> {
        let mut result = self.pattern.clone();

        if let Some(metadata) = &media_file.metadata {
            if let Some(datetime) = &metadata.datetime_original {
                result = result.replace("{datetime}", datetime);

                let parts: Vec<&str> = datetime.split('_').collect();
                if parts.len() == 2 {
                    let date_parts: Vec<&str> = parts[0].split('-').collect();
                    if date_parts.len() == 3 {
                        result = result.replace("{year}", date_parts[0]);
                        result = result.replace("{month}", date_parts[1]);
                        result = result.replace("{day}", date_parts[2]);
                    }
                }
            }

            result = result.replace(
                "{media_type}",
                &format!("{:?}", metadata.media_type).to_lowercase(),
            );
        }

        Ok(base_path.join(result))
    }
}

impl OrganizationStrategy for CustomOrganizer {
    fn determine_target_directory(
        &self,
        media_file: &MediaFile,
        base_path: &Path,
    ) -> Result<PathBuf> {
        self.replace_placeholders(media_file, base_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{MediaFile, MediaMetadata};
    use std::path::PathBuf;

    fn create_test_media_file_with_datetime(datetime: &str, media_type: &str) -> MediaFile {
        let path = PathBuf::from("/test/image.jpg");
        let metadata =
            MediaMetadata::new(media_type.to_string()).with_datetime(datetime.to_string());
        MediaFile::new(&path).with_metadata(metadata)
    }

    #[test]
    fn test_monthly_organizer() {
        let organizer = MonthlyOrganizer::new();
        let media_file = create_test_media_file_with_datetime("2023-12-01_14-30-00", "image/jpeg");
        let base_path = Path::new("/media");

        let result = organizer
            .determine_target_directory(&media_file, base_path)
            .unwrap();
        assert_eq!(result, PathBuf::from("/media/2023/12"));
    }

    #[test]
    fn test_monthly_organizer_different_date() {
        let organizer = MonthlyOrganizer::new();
        let media_file = create_test_media_file_with_datetime("2024-01-15_09-45-30", "video/mp4");
        let base_path = Path::new("/storage");

        let result = organizer
            .determine_target_directory(&media_file, base_path)
            .unwrap();
        assert_eq!(result, PathBuf::from("/storage/2024/01"));
    }

    #[test]
    fn test_monthly_organizer_no_datetime() {
        let organizer = MonthlyOrganizer::new();
        let path = PathBuf::from("/test/image.jpg");
        let metadata = MediaMetadata::new("image/jpeg".to_string());
        let media_file = MediaFile::new(&path).with_metadata(metadata);
        let base_path = Path::new("/media");

        let result = organizer.determine_target_directory(&media_file, base_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CleanboxError::Exif(_)
        ));
    }

    #[test]
    fn test_monthly_organizer_invalid_datetime_format() {
        let organizer = MonthlyOrganizer::new();

        // Test with empty datetime (no parts at all)
        let media_file = create_test_media_file_with_datetime("", "image/jpeg");
        let base_path = Path::new("/media");
        let result = organizer.determine_target_directory(&media_file, base_path);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::CleanboxError::InvalidDateTime(_) => {} // Expected
            other => panic!("Expected InvalidDateTime, got: {:?}", other),
        }

        // Test with only one part (missing month)
        let media_file = create_test_media_file_with_datetime("2023_14-30-00", "image/jpeg");
        let result = organizer.determine_target_directory(&media_file, base_path);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::CleanboxError::InvalidDateTime(_) => {} // Expected
            other => panic!("Expected InvalidDateTime, got: {:?}", other),
        }
    }

    #[test]
    fn test_flat_organizer() {
        let organizer = FlatOrganizer::new();
        let media_file = create_test_media_file_with_datetime("2023-12-01_14-30-00", "image/jpeg");
        let base_path = Path::new("/media");

        let result = organizer
            .determine_target_directory(&media_file, base_path)
            .unwrap();
        assert_eq!(result, PathBuf::from("/media"));
    }

    #[test]
    fn test_yearly_organizer() {
        let organizer = YearlyOrganizer::new();
        let media_file = create_test_media_file_with_datetime("2023-12-01_14-30-00", "image/jpeg");
        let base_path = Path::new("/media");

        let result = organizer
            .determine_target_directory(&media_file, base_path)
            .unwrap();
        assert_eq!(result, PathBuf::from("/media/2023"));
    }

    #[test]
    fn test_custom_organizer_datetime_replacement() {
        let organizer = CustomOrganizer::new("{year}/{month}/{day}".to_string());
        let media_file = create_test_media_file_with_datetime("2023-12-01_14-30-00", "image/jpeg");
        let base_path = Path::new("/media");

        let result = organizer
            .determine_target_directory(&media_file, base_path)
            .unwrap();
        assert_eq!(result, PathBuf::from("/media/2023/12/01"));
    }

    #[test]
    fn test_custom_organizer_media_type() {
        let organizer = CustomOrganizer::new("{media_type}/{year}".to_string());
        let media_file = create_test_media_file_with_datetime("2023-12-01_14-30-00", "image/jpeg");
        let base_path = Path::new("/media");

        let result = organizer
            .determine_target_directory(&media_file, base_path)
            .unwrap();
        assert_eq!(result, PathBuf::from("/media/image/2023"));
    }

    #[test]
    fn test_custom_organizer_video_type() {
        let organizer = CustomOrganizer::new("{media_type}/{year}".to_string());
        let media_file = create_test_media_file_with_datetime("2023-12-01_14-30-00", "video/mp4");
        let base_path = Path::new("/media");

        let result = organizer
            .determine_target_directory(&media_file, base_path)
            .unwrap();
        assert_eq!(result, PathBuf::from("/media/video/2023"));
    }

    #[test]
    fn test_parse_datetime_parts() {
        let result = MonthlyOrganizer::parse_datetime_parts("2023-12-01_14-30-00").unwrap();
        assert_eq!(result, ("2023".to_string(), "12".to_string()));
    }

    #[test]
    fn test_parse_datetime_parts_invalid_format() {
        let result = MonthlyOrganizer::parse_datetime_parts("invalid");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CleanboxError::InvalidDateTime(_)
        ));
    }

    #[test]
    fn test_parse_datetime_parts_missing_month() {
        let result = MonthlyOrganizer::parse_datetime_parts("2023_14-30-00");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CleanboxError::InvalidDateTime(_)
        ));
    }
}
