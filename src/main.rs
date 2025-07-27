use std::fs;
use std::path::Path;

fn extract_datetime_original<P: AsRef<std::path::Path>>(file_path: P) -> Result<String, String> {
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
                return Ok(format!("{}_{}", date, time));
            }
            return Err("Malformed DateTimeOriginal value".to_string());
        }
    }
    Err("DateTimeOriginal tag not found".to_string())
}

fn rename_file_with_datetime<P: AsRef<Path>>(file_path: P, datetime: &str) -> Result<String, String> {
    let path = file_path.as_ref();
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .ok_or("File has no extension")?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let new_name = format!("{}.{}", datetime, ext);
    let new_path = parent.join(&new_name);
    fs::rename(path, &new_path).map_err(|e| format!("Failed to rename file: {e}"))?;
    Ok(new_path.to_string_lossy().to_string())
}

fn rename_all_media_in_dir<P: AsRef<Path>>(dir_path: P) {
    let dir = match fs::read_dir(&dir_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read directory: {}", e);
            return;
        }
    };
    for entry in dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to read entry: {}", e);
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
                Ok(datetime) => {
                    match rename_file_with_datetime(file_str, &datetime) {
                        Ok(new_path) => println!("Renamed {} to {}", file_str, new_path),
                        Err(e) => eprintln!("Failed to rename {}: {}", file_str, e),
                    }
                }
                Err(e) => eprintln!("Failed to extract DateTimeOriginal from {}: {}", file_str, e),
            }
        }
    }
}

fn main() {
    let dir_path = "/home/daniel/life-inbox-copy"; // Change this to your target directory
    rename_all_media_in_dir(dir_path);
}
