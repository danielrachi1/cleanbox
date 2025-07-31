use std::fmt;

#[derive(Debug)]
pub enum CleanboxError {
    Io(std::io::Error),
    Exif(String),
    InvalidPath(String),
    InvalidDateTime(String),
    InvalidFileExtension(String),
    InvalidFileStem(String),
    FileAlreadyExists(String),
    UnsupportedFileType(String),
    UserCancelled,
    InvalidUserInput(String),
    TagDictionaryCorrupted(String),
}

impl fmt::Display for CleanboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CleanboxError::Io(err) => write!(f, "IO error: {err}"),
            CleanboxError::Exif(msg) => write!(f, "EXIF error: {msg}"),
            CleanboxError::InvalidPath(path) => write!(f, "Invalid path: {path}"),
            CleanboxError::InvalidDateTime(dt) => write!(f, "Invalid datetime format: {dt}"),
            CleanboxError::InvalidFileExtension(path) => {
                write!(f, "File has no extension: {path}")
            }
            CleanboxError::InvalidFileStem(path) => write!(f, "Invalid file stem: {path}"),
            CleanboxError::FileAlreadyExists(path) => write!(f, "File already exists: {path}"),
            CleanboxError::UnsupportedFileType(mime) => {
                write!(f, "Unsupported file type: {mime}")
            }
            CleanboxError::UserCancelled => write!(f, "Operation cancelled by user"),
            CleanboxError::InvalidUserInput(msg) => write!(f, "Invalid user input: {msg}"),
            CleanboxError::TagDictionaryCorrupted(msg) => {
                write!(f, "Tag dictionary corrupted: {msg}")
            }
        }
    }
}

impl std::error::Error for CleanboxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CleanboxError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CleanboxError {
    fn from(err: std::io::Error) -> Self {
        CleanboxError::Io(err)
    }
}

impl From<rexif::ExifError> for CleanboxError {
    fn from(err: rexif::ExifError) -> Self {
        CleanboxError::Exif(format!("Failed to parse EXIF: {err}"))
    }
}

pub type Result<T> = std::result::Result<T, CleanboxError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::io;

    #[test]
    fn test_display_formatting() {
        let io_err = CleanboxError::Io(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        assert!(format!("{io_err}").contains("IO error:"));

        let exif_err = CleanboxError::Exif("parsing failed".to_string());
        assert_eq!(format!("{exif_err}"), "EXIF error: parsing failed");

        let path_err = CleanboxError::InvalidPath("/invalid/path".to_string());
        assert_eq!(format!("{path_err}"), "Invalid path: /invalid/path");

        let datetime_err = CleanboxError::InvalidDateTime("bad format".to_string());
        assert_eq!(
            format!("{datetime_err}"),
            "Invalid datetime format: bad format"
        );

        let ext_err = CleanboxError::InvalidFileExtension("file.txt".to_string());
        assert_eq!(format!("{ext_err}"), "File has no extension: file.txt");

        let stem_err = CleanboxError::InvalidFileStem("file.txt".to_string());
        assert_eq!(format!("{stem_err}"), "Invalid file stem: file.txt");

        let exists_err = CleanboxError::FileAlreadyExists("/path/file.txt".to_string());
        assert_eq!(
            format!("{exists_err}"),
            "File already exists: /path/file.txt"
        );

        let file_err = CleanboxError::UnsupportedFileType("text/plain".to_string());
        assert_eq!(format!("{file_err}"), "Unsupported file type: text/plain");

        let cancel_err = CleanboxError::UserCancelled;
        assert_eq!(format!("{cancel_err}"), "Operation cancelled by user");

        let input_err = CleanboxError::InvalidUserInput("bad date format".to_string());
        assert_eq!(
            format!("{input_err}"),
            "Invalid user input: bad date format"
        );

        let tag_err = CleanboxError::TagDictionaryCorrupted("malformed tags.txt".to_string());
        assert_eq!(
            format!("{tag_err}"),
            "Tag dictionary corrupted: malformed tags.txt"
        );
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let cleanbox_err: CleanboxError = io_err.into();
        match cleanbox_err {
            CleanboxError::Io(_) => {}
            _ => panic!("Expected IO error variant"),
        }
    }

    #[test]
    fn test_error_source() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test error");
        let cleanbox_err = CleanboxError::Io(io_err);
        assert!(cleanbox_err.source().is_some());

        let exif_err = CleanboxError::Exif("test".to_string());
        assert!(exif_err.source().is_none());
    }
}
