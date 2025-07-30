# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cleanbox is a Rust CLI tool that organizes media files by processing images and videos from an inbox directory, extracting EXIF metadata to rename files with their creation timestamps, and moving them to organized directories. The codebase has been refactored into a modular, extensible architecture.

## Architecture

The codebase follows a clean, modular architecture with clear separation of concerns:

### Core Modules

- `src/main.rs`: CLI entry point that uses the library API
- `src/cli.rs`: Command-line argument parsing using clap
- `src/lib.rs`: Public API exports and convenience functions
- `src/error.rs`: Centralized error handling with `CleanboxError` enum
- `src/media.rs`: Domain types (`MediaFile`, `MediaType`, `MediaMetadata`)
- `src/exif.rs`: EXIF parsing abstraction with `ExifParser` trait
- `src/filesystem.rs`: File operations abstraction with `FileManager` trait
- `src/naming.rs`: File naming strategies with `NamingStrategy` trait
- `src/organization.rs`: Directory organization strategies with `OrganizationStrategy` trait
- `src/processor.rs`: Main orchestration logic with `MediaProcessor`
- `src/config.rs`: Configuration types and builder patterns

### Key Traits and Extension Points

- **ExifParser**: Swap EXIF parsing implementations (default: `RexifParser`)
- **FileManager**: Mock filesystem operations for testing (default: `StdFileManager`)
- **NamingStrategy**: Customize file naming (default: `TimestampNamingStrategy`)
- **OrganizationStrategy**: Customize directory structure (default: `MonthlyOrganizer`)

### Processing Pipeline

1. **Scan**: Read files from inbox directory
2. **Parse**: Extract EXIF metadata and determine media type
3. **Name**: Generate new filename using naming strategy
4. **Organize**: Determine target directory using organization strategy
5. **Move**: Handle conflicts and move files to final location

### Data Flow

1. CLI parses `--life-path` argument
2. Creates `ProcessingConfig` with inbox and media root paths
3. `MediaProcessor` orchestrates the pipeline:
   - Scans `{life_path}/inbox/` for files
   - Parses EXIF data to extract DateTimeOriginal
   - Renames using timestamp format (YYYY-MM-DD_HH-MM-SS)
   - Organizes into `{life_path}/media/YYYY/MM/` structure
   - Handles duplicates by appending SHA1 hash

## Usage Examples

### Simple Usage
```rust
use cleanbox::process_media_directory;

let result = process_media_directory("/path/to/inbox", "/path/to/media")?;
println!("Processed {} files", result.processed_files);
```

### Custom Configuration
```rust
use cleanbox::{MediaProcessor, ProcessingConfig, RexifParser, StdFileManager, 
               TimestampNamingStrategy, YearlyOrganizer, DuplicateHandling};

let config = ProcessingConfig::new(inbox_path, media_root)
    .with_hash_length(8)
    .with_duplicate_handling(DuplicateHandling::Skip);

let processor = MediaProcessor::new(
    RexifParser::new(),
    StdFileManager::new(),
    TimestampNamingStrategy::new(),
    YearlyOrganizer::new(), // Organize by year only
    config,
);

let result = processor.process_directory()?;
```

### Custom Naming Pattern
```rust
use cleanbox::{CustomNamingStrategy, CustomOrganizer};

let naming = CustomNamingStrategy::new("{year}-{month}-{day}_{hour}-{minute}-{second}_{hash6}.{ext}".to_string());
let organization = CustomOrganizer::new("{media_type}/{year}".to_string());
```

## Development Commands

```bash
# Build the project
cargo build

# Run with arguments
cargo run -- --life-path /path/to/life/directory

# Run tests
cargo test

# Check for warnings and formatting
cargo fmt
cargo clippy

# Build release version
cargo build --release
```

## Adding New Features

### New Naming Strategy
Implement `NamingStrategy` trait:
```rust
pub struct MyNamingStrategy;
impl NamingStrategy for MyNamingStrategy {
    fn generate_name(&self, media_file: &MediaFile) -> Result<String> {
        // Custom naming logic
    }
}
```

### New Organization Strategy
Implement `OrganizationStrategy` trait:
```rust
pub struct MyOrganizer;
impl OrganizationStrategy for MyOrganizer {
    fn determine_target_directory(&self, media_file: &MediaFile, base_path: &Path) -> Result<PathBuf> {
        // Custom directory structure
    }
}
```

## Testing

The codebase includes comprehensive unit tests with 63 test cases covering all modules:

### Running Tests
```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test media::tests

# Run tests with output
cargo test -- --nocapture

# Run tests with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out html
```

### Test Coverage
- **Error handling**: Display formatting, error conversion, source chaining
- **Media types**: File validation, metadata handling, path operations
- **Naming strategies**: Timestamp formatting, custom patterns, placeholder replacement
- **Organization**: Monthly/yearly/flat/custom directory structures
- **File operations**: Mock filesystem for testing, hash generation
- **Configuration**: Builder patterns, option chaining
- **Processing pipeline**: Error handling, duplicate resolution, orchestration
- **Integration**: End-to-end API usage, type compatibility

### Mock Objects for Testing
```rust
use cleanbox::filesystem::MockFileManager;
use cleanbox::processor::tests::{MockExifParser, MockNamingStrategy, MockOrganizationStrategy};

let mut file_manager = MockFileManager::new();
file_manager.add_file(PathBuf::from("/test/image.jpg"), b"test content".to_vec());
```

## Error Handling

The codebase uses a centralized error system with proper error propagation:

```rust
use cleanbox::{CleanboxError, Result};

// All functions return Result<T> for consistent error handling
fn process_file() -> Result<String> {
    // Automatic conversion from std::io::Error
    let content = std::fs::read_to_string("file.txt")?;
    
    // Custom errors
    if content.is_empty() {
        return Err(CleanboxError::InvalidPath("Empty file".to_string()));
    }
    
    Ok(content)
}
```

### Error Types
- `Io(std::io::Error)`: File system operations
- `Exif(String)`: EXIF parsing failures
- `InvalidPath(String)`: Invalid file paths
- `InvalidDateTime(String)`: Date parsing errors
- `InvalidFileExtension(String)`: Missing file extensions
- `InvalidFileStem(String)`: Invalid file stems
- `FileAlreadyExists(String)`: Duplicate file conflicts
- `UnsupportedMediaType(String)`: Non-media files

## Configuration Options

### ProcessingConfig Builder
```rust
use cleanbox::{ProcessingConfig, DuplicateHandling};

let config = ProcessingConfig::new(inbox_path, media_root)
    .with_hash_length(8)                                    // Hash suffix length (default: 6)
    .with_duplicate_handling(DuplicateHandling::Skip)       // How to handle duplicates
    .with_backup(true)                                      // Create backups (default: false)
    .skip_unsupported(false);                              // Skip unsupported files (default: true)
```

### Duplicate Handling Options
- `AppendHash`: Add hash suffix to filename (default)
- `Skip`: Skip duplicate files
- `Overwrite`: Replace existing files
- `Error`: Fail on duplicates

### Available Organizers
- `MonthlyOrganizer`: `media/YYYY/MM/` (default)
- `YearlyOrganizer`: `media/YYYY/`
- `FlatOrganizer`: `media/` (no subdirectories)
- `CustomOrganizer`: Custom pattern with placeholders

### Available Naming Strategies
- `TimestampNamingStrategy`: `YYYY-MM-DD_HH-MM-SS.ext` (default)
- `CustomNamingStrategy`: Custom pattern with placeholders like `{year}`, `{month}`, `{hash6}`, etc.

## Dependencies

- `rexif`: EXIF metadata extraction from images/videos
- `clap`: Command-line argument parsing with derive feature
- `sha1`: Hash generation for duplicate handling