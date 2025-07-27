use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn extract_datetime_original<P: AsRef<std::path::Path>>(
    file_path: P,
) -> Result<String, String> {
    let exif = rexif::parse_file(file_path.as_ref().to_str().unwrap())
        .map_err(|e| format!("Failed to parse EXIF: {e}"))?;
    for entry in &exif.entries {
        if entry.tag == rexif::ExifTag::DateTimeOriginal {
            // EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
            let raw = entry.value_more_readable.trim();
            let parts: Vec<&str> = raw.split(' ').collect();
            if parts.len() == 2 {
                let date = parts[0].replace(":", "-");
                let time = parts[1].replace(":", "-");
                return Ok(format!("{date}_{time}"));
            }
            return Err("Malformed DateTimeOriginal value".to_string());
        }
    }
    Err("DateTimeOriginal tag not found".to_string())
}

pub fn rename_file_with_datetime<P: AsRef<Path>>(
    file_path: P,
    datetime: &str,
) -> Result<String, String> {
    let path = file_path.as_ref();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or("File has no extension")?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let new_name = format!("{datetime}.{ext}");
    let new_path = parent.join(&new_name);
    fs::rename(path, &new_path).map_err(|e| format!("Failed to rename file: {e}"))?;
    Ok(new_path.to_string_lossy().to_string())
}

pub fn append_sha1_to_filename<P: AsRef<Path>>(file_path: P) -> Result<String, String> {
    use sha1::{Digest, Sha1};
    let path = file_path.as_ref();
    let mut file =
        fs::File::open(path).map_err(|e| format!("Failed to open file for hashing: {e}"))?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file for hashing: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let hash = hasher.finalize();
    let hash_str = format!("{hash:x}");
    let hash6 = &hash_str[..6];
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file stem")?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or("Invalid file extension")?;
    Ok(format!("{stem}_{hash6}.{ext}"))
}

pub fn move_media_to_monthly_dir<P: AsRef<Path>>(
    file_path: P,
    datetime: &str,
    life_media_root: &Path,
) -> Result<PathBuf, String> {
    // datetime: YYYY-MM-DD_HH-MM-SS
    let date_part = datetime
        .split('_')
        .next()
        .ok_or("Invalid datetime format")?;
    let mut date_split = date_part.split('-');
    let year = date_split.next().ok_or("Invalid date format")?;
    let month = date_split.next().ok_or("Invalid date format")?; // nth(1) skips month
    let target_dir = life_media_root.join(year).join(month);
    fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create target dir: {e}"))?;
    let ext = Path::new(file_path.as_ref())
        .extension()
        .and_then(|e| e.to_str())
        .ok_or("File has no extension")?;
    let target_name = format!("{datetime}.{ext}");
    let mut target_path = target_dir.join(&target_name);
    // If collision, append hash
    if target_path.exists() {
        let hash_name = append_sha1_to_filename(&file_path)?;
        target_path = target_dir.join(hash_name);
    }
    fs::rename(&file_path, &target_path).map_err(|e| format!("Failed to move file: {e}"))?;
    Ok(target_path)
}

pub fn rename_all_media_in_dir<P: AsRef<Path>>(dir_path: P, life_media_root: &Path) {
    let dir = match fs::read_dir(&dir_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read directory: {e}");
            return;
        }
    };
    for entry in dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to read entry: {e}");
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_str = match path.to_str() {
            Some(s) => s,
            None => {
                eprintln!("Invalid file path");
                continue;
            }
        };
        let exif = match rexif::parse_file(file_str) {
            Ok(exif) => exif,
            Err(_) => continue, // Not a media file rexif can parse
        };
        let mime = exif.mime.to_lowercase();
        if mime.starts_with("image/") || mime.starts_with("video/") {
            match extract_datetime_original(file_str) {
                Ok(datetime) => match rename_file_with_datetime(file_str, &datetime) {
                    Ok(renamed_path) => {
                        match move_media_to_monthly_dir(&renamed_path, &datetime, life_media_root) {
                            Ok(final_path) => println!("Moved to {}", final_path.display()),
                            Err(e) => eprintln!("Failed to move {renamed_path}: {e}"),
                        }
                    }
                    Err(e) => eprintln!("Failed to rename {file_str}: {e}"),
                },
                Err(e) => eprintln!("Failed to extract DateTimeOriginal from {file_str}: {e}"),
            }
        }
    }
}
