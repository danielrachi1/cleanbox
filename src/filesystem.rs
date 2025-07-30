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
}

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
#[derive(Default)]
pub struct MockFileManager {
    pub files: HashMap<PathBuf, Vec<u8>>,
    pub directories: Vec<PathBuf>,
}

#[cfg(test)]
impl MockFileManager {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            directories: Vec::new(),
        }
    }

    pub fn add_file(&mut self, path: PathBuf, content: Vec<u8>) {
        self.files.insert(path, content);
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

    // Integration tests with StdFileManager would require actual file system operations
    // These are typically run in a separate test environment or with temp directories
}
