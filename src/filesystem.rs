use crate::error::{CleanboxError, Result};
use sha1::{Digest, Sha1};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::collections::HashMap;

pub trait FileManager {
    fn read_directory<P: AsRef<Path>>(&self, path: P) -> Result<Vec<PathBuf>>;
    fn create_directories<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    fn rename_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> Result<()>;
    fn move_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> Result<()>;
    fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool;
    fn is_file<P: AsRef<Path>>(&self, path: P) -> bool;
    fn calculate_file_hash<P: AsRef<Path>>(&self, path: P) -> Result<String>;
    fn get_file_modified_time<P: AsRef<Path>>(&self, path: P) -> Result<std::time::SystemTime>;
}

#[derive(Clone)]
pub struct StdFileManager;

impl StdFileManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdFileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileManager for StdFileManager {
    fn read_directory<P: AsRef<Path>>(&self, path: P) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(path)?;
        let mut paths = Vec::new();

        for entry in entries {
            let entry = entry?;
            paths.push(entry.path());
        }

        Ok(paths)
    }

    fn create_directories<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        fs::create_dir_all(path)?;
        Ok(())
    }

    fn rename_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> Result<()> {
        fs::rename(from, to)?;
        Ok(())
    }

    fn move_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> Result<()> {
        if let Some(parent) = to.as_ref().parent() {
            self.create_directories(parent)?;
        }
        self.rename_file(from, to)
    }

    fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().exists()
    }

    fn is_file<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().is_file()
    }

    fn calculate_file_hash<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha1::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let hash = hasher.finalize();
        Ok(format!("{hash:x}"))
    }

    fn get_file_modified_time<P: AsRef<Path>>(&self, path: P) -> Result<std::time::SystemTime> {
        let metadata = fs::metadata(path)?;
        Ok(metadata.modified()?)
    }
}

pub struct FileHasher;

impl FileHasher {
    pub fn generate_hash_suffix(hash: &str, length: usize) -> String {
        let suffix_length = std::cmp::min(length, hash.len());
        hash[..suffix_length].to_string()
    }

    pub fn append_hash_to_filename(original_name: &str, hash_suffix: &str) -> Result<String> {
        let path = Path::new(original_name);

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| CleanboxError::InvalidFileStem(original_name.to_string()))?;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| CleanboxError::InvalidFileExtension(original_name.to_string()))?;

        Ok(format!("{stem}_{hash_suffix}.{ext}"))
    }
}

// Mock FileManager for testing - available in test builds
#[cfg(test)]
#[derive(Default, Clone)]
pub struct MockFileManager {
    pub files: HashMap<PathBuf, Vec<u8>>,
    pub directories: Vec<PathBuf>,
    pub file_modified_times: HashMap<PathBuf, std::time::SystemTime>,
}

#[cfg(test)]
impl MockFileManager {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            directories: Vec::new(),
            file_modified_times: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, path: PathBuf, content: Vec<u8>) {
        self.files.insert(path, content);
    }

    pub fn add_file_with_modified_time(&mut self, path: PathBuf, content: Vec<u8>, modified_time: std::time::SystemTime) {
        self.files.insert(path.clone(), content);
        self.file_modified_times.insert(path, modified_time);
    }
}

#[cfg(test)]
impl FileManager for MockFileManager {
    fn read_directory<P: AsRef<Path>>(&self, path: P) -> Result<Vec<PathBuf>> {
        let path_buf = path.as_ref().to_path_buf();
        let files: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|p| p.parent() == Some(&path_buf))
            .cloned()
            .collect();
        Ok(files)
    }

    fn create_directories<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        Ok(())
    }

    fn rename_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, _from: P, _to: Q) -> Result<()> {
        Ok(())
    }

    fn move_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, _from: P, _to: Q) -> Result<()> {
        Ok(())
    }

    fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.contains_key(path.as_ref())
    }

    fn is_file<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.contains_key(path.as_ref())
    }

    fn calculate_file_hash<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        if let Some(content) = self.files.get(path.as_ref()) {
            let mut hasher = sha1::Sha1::new();
            hasher.update(content);
            let hash = hasher.finalize();
            Ok(format!("{hash:x}"))
        } else {
            Err(CleanboxError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            )))
        }
    }

    fn get_file_modified_time<P: AsRef<Path>>(&self, path: P) -> Result<std::time::SystemTime> {
        if let Some(modified_time) = self.file_modified_times.get(path.as_ref()) {
            Ok(*modified_time)
        } else if self.files.contains_key(path.as_ref()) {
            // Default to current time if no specific modified time was set
            Ok(std::time::SystemTime::now())
        } else {
            Err(CleanboxError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_mock_file_manager_read_directory() {
        let mut manager = MockFileManager::new();
        manager.add_file(PathBuf::from("/test/file1.jpg"), vec![1, 2, 3]);
        manager.add_file(PathBuf::from("/test/file2.jpg"), vec![4, 5, 6]);
        manager.add_file(PathBuf::from("/other/file3.jpg"), vec![7, 8, 9]);

        let files = manager.read_directory("/test").unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&PathBuf::from("/test/file1.jpg")));
        assert!(files.contains(&PathBuf::from("/test/file2.jpg")));
    }

    #[test]
    fn test_mock_file_manager_file_exists() {
        let mut manager = MockFileManager::new();
        manager.add_file(PathBuf::from("/test/file1.jpg"), vec![1, 2, 3]);

        assert!(manager.file_exists("/test/file1.jpg"));
        assert!(!manager.file_exists("/test/nonexistent.jpg"));
    }

    #[test]
    fn test_mock_file_manager_calculate_hash() {
        let mut manager = MockFileManager::new();
        let content = b"test content".to_vec();
        manager.add_file(PathBuf::from("/test/file1.txt"), content.clone());

        let hash1 = manager.calculate_file_hash("/test/file1.txt").unwrap();
        let hash2 = manager.calculate_file_hash("/test/file1.txt").unwrap();

        // Same content should produce same hash
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_mock_file_manager_calculate_hash_nonexistent() {
        let manager = MockFileManager::new();
        let result = manager.calculate_file_hash("/test/nonexistent.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_hasher_generate_hash_suffix() {
        let hash = "abcdef123456789";
        assert_eq!(FileHasher::generate_hash_suffix(hash, 6), "abcdef");
        assert_eq!(FileHasher::generate_hash_suffix(hash, 3), "abc");
        assert_eq!(FileHasher::generate_hash_suffix(hash, 20), hash); // longer than hash
        assert_eq!(FileHasher::generate_hash_suffix(hash, 0), "");
    }

    #[test]
    fn test_file_hasher_append_hash_to_filename() {
        let result = FileHasher::append_hash_to_filename("image.jpg", "abc123").unwrap();
        assert_eq!(result, "image_abc123.jpg");

        let result = FileHasher::append_hash_to_filename("my_photo.png", "def456").unwrap();
        assert_eq!(result, "my_photo_def456.png");
    }

    #[test]
    fn test_file_hasher_append_hash_no_extension() {
        let result = FileHasher::append_hash_to_filename("filename", "abc123");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CleanboxError::InvalidFileExtension(_)
        ));
    }

    #[test]
    fn test_file_hasher_append_hash_no_stem() {
        // In Rust path handling:
        // ".gitignore" has file_stem = None and extension = None (it's treated as just a filename)
        // ".hidden.txt" has file_stem = Some(".hidden") and extension = Some("txt")

        // Test with a path that has no extension - this should fail
        let result = FileHasher::append_hash_to_filename("filename", "abc123");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CleanboxError::InvalidFileExtension(_)
        ));

        // Test with .gitignore which has no stem or extension - should fail with no extension
        let result = FileHasher::append_hash_to_filename(".gitignore", "abc123");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CleanboxError::InvalidFileExtension(_)
        ));

        // Test with a file that truly has an empty stem but valid extension
        let result = FileHasher::append_hash_to_filename(".hidden.txt", "abc123");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ".hidden_abc123.txt");
    }

    #[test]
    fn test_mock_file_manager_get_file_modified_time() {
        use std::time::{Duration, UNIX_EPOCH};
        
        let mut manager = MockFileManager::new();
        let test_time = UNIX_EPOCH + Duration::from_secs(1000000);
        
        // Test file with specific modified time
        manager.add_file_with_modified_time(
            PathBuf::from("/test/file1.txt"), 
            vec![1, 2, 3], 
            test_time
        );
        
        let result = manager.get_file_modified_time("/test/file1.txt").unwrap();
        assert_eq!(result, test_time);
        
        // Test file without specific modified time (should default to current time)
        manager.add_file(PathBuf::from("/test/file2.txt"), vec![4, 5, 6]);
        let result = manager.get_file_modified_time("/test/file2.txt");
        assert!(result.is_ok());
        
        // Test nonexistent file
        let result = manager.get_file_modified_time("/test/nonexistent.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_file_manager_add_file_with_modified_time() {
        use std::time::{Duration, UNIX_EPOCH};
        
        let mut manager = MockFileManager::new();
        let test_time = UNIX_EPOCH + Duration::from_secs(2000000);
        let content = b"test content".to_vec();
        let path = PathBuf::from("/test/file.txt");
        
        manager.add_file_with_modified_time(path.clone(), content.clone(), test_time);
        
        // Verify file exists
        assert!(manager.file_exists(&path));
        
        // Verify content is correct
        let hash = manager.calculate_file_hash(&path).unwrap();
        assert!(!hash.is_empty());
        
        // Verify modified time is correct
        let modified_time = manager.get_file_modified_time(&path).unwrap();
        assert_eq!(modified_time, test_time);
    }

    // Integration tests with StdFileManager would require actual file system operations
    // These are typically run in a separate test environment or with temp directories
}
