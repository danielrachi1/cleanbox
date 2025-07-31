# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cleanbox is a Rust CLI tool that intelligently organizes files by processing both media and documents from an inbox directory. Media files (images/videos) are automatically processed using EXIF metadata for timestamp-based naming, while documents are processed interactively with user-provided semantic information. The codebase features a unified, modular architecture that handles different file types through intelligent routing.

## Architecture

The codebase follows a clean, modular architecture with clear separation of concerns:

### Core Modules

- `src/main.rs`: CLI entry point with unified processing workflow
- `src/cli.rs`: Command-line argument parsing using clap
- `src/lib.rs`: Public API exports and convenience functions
- `src/error.rs`: Centralized error handling with `CleanboxError` enum
- `src/media.rs`: Domain types (`File`, `FileType`, `FileMetadata`)
- `src/metadata.rs`: Metadata parsing abstraction with `MetadataParser` trait  
- `src/filesystem.rs`: File operations abstraction with `FileManager` trait
- `src/naming.rs`: File naming strategies (`TimestampNamingStrategy`, `DocumentNamingStrategy`)
- `src/organization.rs`: Directory organization strategies (`MonthlyOrganizer`, `DocumentOrganizer`)
- `src/processor.rs`: Processing orchestration (`FileProcessor`, `UnifiedProcessor`)
- `src/config.rs`: Configuration types (`ProcessingConfig`, `LifeConfig`)
- `src/document.rs`: Document-specific types and validation (`DocumentInput`)
- `src/tags.rs`: Tag management and fuzzy matching (`TagDictionary`, `TagResolutionFlow`)
- `src/interactive.rs`: Interactive user prompts and document input collection
- `src/paths.rs`: Path resolution for different file types (`BasePathResolver`)

### Key Traits and Extension Points

- **MetadataParser**: Swap metadata parsing implementations (default: `RexifParser`)
- **FileManager**: Mock filesystem operations for testing (default: `StdFileManager`)
- **NamingStrategy**: Customize file naming (`TimestampNamingStrategy`, `DocumentNamingStrategy`)
- **OrganizationStrategy**: Customize directory structure (`MonthlyOrganizer`, `DocumentOrganizer`)
- **BasePathResolver**: Route files to appropriate directories (default: `LifeDirectoryResolver`)
- **UserPrompt**: Interactive user input handling (default: `ConsolePrompt`)

### Unified Processing Pipeline

1. **Scan**: Read and categorize files from inbox directory (media/documents/unknown)
2. **Route**: Intelligently route files based on type:
   - **Media files**: Automatic processing with EXIF metadata extraction
   - **Documents**: Interactive processing with user input collection
   - **Unknown**: Skip with user notification
3. **Process**: Apply type-specific processing:
   - **Media**: Extract metadata → generate timestamp-based filename → organize by date
   - **Documents**: Collect user input → generate semantic filename → organize by date
4. **Move**: Handle conflicts with hash suffixes and move to final location

### Data Flow

1. CLI parses `--life-path` argument
2. Creates `LifeConfig` with single source of truth for all paths
3. `UnifiedProcessor` orchestrates the complete workflow:
   - Scans `{life_path}/inbox/` and categorizes files by type
   - **Media files**: Extracts EXIF data → renames using `YYYY-MM-DD_HH-MM-SS.ext` → moves to `{life_path}/media/YYYY/MM/`
   - **Documents**: Interactive prompts → validates input → renames using `YYYY-MM-DD_description@@tag1,tag2.ext` → moves to `{life_path}/documents/YYYY/MM/`
   - **Unknown files**: Reports and leaves in inbox
   - Handles duplicates by appending SHA1 hash suffix

## Usage Examples

### Unified Processing (Recommended)
```rust
use cleanbox::process_life_directory_unified;

let result = process_life_directory_unified("/path/to/life")?;
println!("Media processed: {}, Documents processed: {}", 
         result.media_processed, result.documents_processed);
```

### Legacy Media-Only Processing
```rust
use cleanbox::process_media_directory;

let result = process_media_directory("/path/to/inbox", "/path/to/media")?;
println!("Processed {} files", result.processed_files);
```

### Custom Unified Configuration
```rust
use cleanbox::{UnifiedProcessor, LifeConfig, RexifParser, StdFileManager, 
               interactive::ConsolePrompt, DuplicateHandling};

let life_config = LifeConfig::new("/path/to/life".into())
    .with_hash_length(8)
    .with_duplicate_handling(DuplicateHandling::Skip);

let processor = UnifiedProcessor::new(
    RexifParser::new(),
    StdFileManager::new(),
    ConsolePrompt::new(),
    life_config,
);

let result = processor.process_life_directory()?;
```

### Document Processing Components
```rust
use cleanbox::{DocumentNamingStrategy, DocumentOrganizer, DocumentInput};

let naming = DocumentNamingStrategy::new();
let organizer = DocumentOrganizer::new();

let document_input = DocumentInput::new(
    "2025-07-31".to_string(),
    "quarterly-report".to_string(), 
    vec!["finance".to_string(), "reports".to_string()]
);

let filename = naming.generate_name_from_input(&document_input, "pdf")?;
// Result: "2025-07-31_quarterly-report@@finance,reports.pdf"
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
    fn generate_name(&self, file: &File) -> Result<String> {
        // Custom naming logic
    }
}
```

### New Organization Strategy
Implement `OrganizationStrategy` trait:
```rust
pub struct MyOrganizer;
impl OrganizationStrategy for MyOrganizer {
    fn determine_target_directory(&self, file: &File, base_path: &Path) -> Result<PathBuf> {
        // Custom directory structure
    }
}
```

### New Interactive Prompts
Implement `UserPrompt` trait:
```rust
pub struct MyPrompt;
impl UserPrompt for MyPrompt {
    fn prompt_string(&self, message: &str, default: Option<&str>) -> Result<String> {
        // Custom prompt logic
    }
    // ... other methods
}
```

## Testing

The codebase includes comprehensive unit tests with 121 test cases covering all modules:

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
- **File types**: File validation, metadata handling, path operations, processing behavior methods
- **Naming strategies**: Timestamp formatting, document formatting, custom patterns, placeholder replacement
- **Organization**: Monthly/yearly/flat/custom/document directory structures  
- **File operations**: Mock filesystem for testing, hash generation
- **Configuration**: Builder patterns, option chaining, LifeConfig integration
- **Processing pipeline**: Error handling, duplicate resolution, orchestration, unified workflow
- **Document processing**: Interactive prompts, tag validation, fuzzy matching
- **Integration**: End-to-end API usage, unified processing, type compatibility

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
- `UnsupportedFileType(String)`: Non-supported files
- `UserCancelled`: Interactive processing cancelled by user
- `InvalidUserInput(String)`: User input validation failures
- `TagDictionaryCorrupted(String)`: Malformed tags.txt file

## Configuration Options

### LifeConfig Builder (Recommended)
```rust
use cleanbox::{LifeConfig, DuplicateHandling};

let config = LifeConfig::new("/path/to/life".into())
    .with_hash_length(8)                                    // Hash suffix length (default: 6)
    .with_duplicate_handling(DuplicateHandling::Skip)       // How to handle duplicates
    .with_backup(true)                                      // Create backups (default: false)
    .skip_unsupported(false);                              // Skip unsupported files (default: true)

// Derived paths
let inbox = config.inbox_path();                            // /path/to/life/inbox
let media = config.media_root();                           // /path/to/life/media
let documents = config.documents_root();                   // /path/to/life/documents
let tags_file = config.tags_file();                        // /path/to/life/documents/tags.txt
```

### Legacy ProcessingConfig Builder
```rust
use cleanbox::{ProcessingConfig, DuplicateHandling};

let config = ProcessingConfig::new(inbox_path, media_root)
    .with_hash_length(8)
    .with_duplicate_handling(DuplicateHandling::Skip)
    .with_backup(true)
    .skip_unsupported(false);
```

### Duplicate Handling Options
- `AppendHash`: Add hash suffix to filename (default)
- `Skip`: Skip duplicate files
- `Overwrite`: Replace existing files
- `Error`: Fail on duplicates

### Available Organizers
- `MonthlyOrganizer`: `base/YYYY/MM/` (default for both media and documents)
- `DocumentOrganizer`: `documents/YYYY/MM/` (specialized for documents)
- `YearlyOrganizer`: `base/YYYY/`
- `FlatOrganizer`: `base/` (no subdirectories)
- `CustomOrganizer`: Custom pattern with placeholders

### Available Naming Strategies
- `TimestampNamingStrategy`: `YYYY-MM-DD_HH-MM-SS.ext` (default for media)
- `DocumentNamingStrategy`: `YYYY-MM-DD_description@@tag1,tag2.ext` (for documents)
- `CustomNamingStrategy`: Custom pattern with placeholders like `{year}`, `{month}`, `{hash6}`, etc.

## Dependencies

- `rexif`: EXIF metadata extraction from images/videos
- `clap`: Command-line argument parsing with derive feature
- `sha1`: Hash generation for duplicate handling
- `strsim`: Fuzzy string matching for tag suggestions
- `infer`: MIME type detection for file categorization