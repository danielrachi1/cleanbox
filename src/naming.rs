use crate::error::{CleanboxError, Result};
use crate::media::File;

pub trait NamingStrategy {
    fn generate_name(&self, file: &File) -> Result<String>;
}

pub struct TimestampNamingStrategy;

impl TimestampNamingStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimestampNamingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl NamingStrategy for TimestampNamingStrategy {
    fn generate_name(&self, file: &File) -> Result<String> {
        let extension = file.extension()?;

        let datetime = file
            .metadata
            .as_ref()
            .and_then(|m| m.datetime_original.as_ref())
            .ok_or_else(|| CleanboxError::Exif("No datetime available for naming".to_string()))?;

        Ok(format!("{datetime}.{extension}"))
    }
}

pub struct CustomNamingStrategy {
    pattern: String,
}

impl CustomNamingStrategy {
    pub fn new(pattern: String) -> Self {
        Self { pattern }
    }

    fn replace_placeholders(&self, file: &File) -> Result<String> {
        let mut result = self.pattern.clone();

        if let Some(metadata) = &file.metadata {
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

                    let time_parts: Vec<&str> = parts[1].split('-').collect();
                    if time_parts.len() == 3 {
                        result = result.replace("{hour}", time_parts[0]);
                        result = result.replace("{minute}", time_parts[1]);
                        result = result.replace("{second}", time_parts[2]);
                    }
                }
            }

            if let Some(hash) = &metadata.file_hash {
                result = result.replace("{hash}", hash);
                result = result.replace("{hash6}", &hash[..std::cmp::min(6, hash.len())]);
            }
        }

        let original_name = file.file_name()?;
        result = result.replace("{original}", original_name);

        let stem = file.file_stem()?;
        result = result.replace("{stem}", stem);

        let extension = file.extension()?;
        result = result.replace("{ext}", extension);

        Ok(result)
    }
}

impl NamingStrategy for CustomNamingStrategy {
    fn generate_name(&self, file: &File) -> Result<String> {
        self.replace_placeholders(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{File, FileMetadata};
    use std::path::PathBuf;

    fn create_test_file_with_datetime(datetime: &str) -> File {
        let path = PathBuf::from("/test/image.jpg");
        let metadata =
            FileMetadata::new("image/jpeg".to_string()).with_datetime(datetime.to_string());
        File::new(&path).with_metadata(metadata)
    }

    #[test]
    fn test_timestamp_naming_strategy() {
        let strategy = TimestampNamingStrategy::new();
        let file = create_test_file_with_datetime("2023-12-01_14-30-00");

        let result = strategy.generate_name(&file).unwrap();
        assert_eq!(result, "2023-12-01_14-30-00.jpg");
    }

    #[test]
    fn test_timestamp_naming_strategy_no_datetime() {
        let strategy = TimestampNamingStrategy::new();
        let path = PathBuf::from("/test/image.jpg");
        let metadata = FileMetadata::new("image/jpeg".to_string());
        let file = File::new(&path).with_metadata(metadata);

        let result = strategy.generate_name(&file);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CleanboxError::Exif(_)
        ));
    }

    #[test]
    fn test_custom_naming_strategy_datetime_replacement() {
        let strategy = CustomNamingStrategy::new("{datetime}.{ext}".to_string());
        let file = create_test_file_with_datetime("2023-12-01_14-30-00");

        let result = strategy.generate_name(&file).unwrap();
        assert_eq!(result, "2023-12-01_14-30-00.jpg");
    }

    #[test]
    fn test_custom_naming_strategy_date_parts() {
        let strategy = CustomNamingStrategy::new(
            "{year}-{month}-{day}_{hour}{minute}{second}.{ext}".to_string(),
        );
        let file = create_test_file_with_datetime("2023-12-01_14-30-00");

        let result = strategy.generate_name(&file).unwrap();
        assert_eq!(result, "2023-12-01_143000.jpg");
    }

    #[test]
    fn test_custom_naming_strategy_with_hash() {
        let strategy = CustomNamingStrategy::new("{datetime}_{hash6}.{ext}".to_string());
        let path = PathBuf::from("/test/image.jpg");
        let metadata = FileMetadata::new("image/jpeg".to_string())
            .with_datetime("2023-12-01_14-30-00".to_string())
            .with_hash("abcdef123456".to_string());
        let file = File::new(&path).with_metadata(metadata);

        let result = strategy.generate_name(&file).unwrap();
        assert_eq!(result, "2023-12-01_14-30-00_abcdef.jpg");
    }

    #[test]
    fn test_custom_naming_strategy_original_and_stem() {
        let strategy = CustomNamingStrategy::new("{stem}_processed.{ext}".to_string());
        let path = PathBuf::from("/test/my_image.jpg");
        let metadata = FileMetadata::new("image/jpeg".to_string());
        let file = File::new(&path).with_metadata(metadata);

        let result = strategy.generate_name(&file).unwrap();
        assert_eq!(result, "my_image_processed.jpg");
    }

    #[test]
    fn test_custom_naming_strategy_short_hash() {
        let strategy = CustomNamingStrategy::new("{hash6}.{ext}".to_string());
        let path = PathBuf::from("/test/image.jpg");
        let metadata = FileMetadata::new("image/jpeg".to_string()).with_hash("abc".to_string()); // Hash shorter than 6 chars
        let file = File::new(&path).with_metadata(metadata);

        let result = strategy.generate_name(&file).unwrap();
        assert_eq!(result, "abc.jpg");
    }

    #[test]
    fn test_custom_naming_strategy_no_datetime_parts() {
        let strategy = CustomNamingStrategy::new("{year}.{ext}".to_string());
        let path = PathBuf::from("/test/image.jpg");
        let metadata = FileMetadata::new("image/jpeg".to_string());
        let file = File::new(&path).with_metadata(metadata);

        let result = strategy.generate_name(&file).unwrap();
        assert_eq!(result, "{year}.jpg"); // Should leave placeholder as-is
    }
}
