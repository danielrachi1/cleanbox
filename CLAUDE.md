# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cleanbox is a Rust CLI tool for organizing and processing media files and documents from an "inbox" directory into structured directories. It processes media files (images, videos, audio) using EXIF metadata and handles documents through interactive prompts for tagging and organization.

## Commands

### Build and Development
- `cargo build` - Build the project
- `cargo run -- --life-path /path/to/life` - Run the application
- `cargo test` - Run all tests
- `cargo check` - Check code without building

### Testing
- `cargo test` - Run all unit tests
- `cargo test test_name` - Run specific test
- `cargo test --lib` - Run library tests only

## Architecture

### Core Processing Flow
The application operates on a "life directory" structure:
```
life/
├── inbox/          # Input files to be processed
├── media/          # Organized media files (by year/month)
└── documents/      # Organized documents with tags
    └── tags.txt    # Tag dictionary for consistent tagging
```

### Key Components

**UnifiedProcessor** (`src/processor.rs`): Main orchestrator that processes both media and documents
- Handles media files automatically using EXIF metadata
- Processes documents interactively with user prompts for date, description, and tags
- Uses a tag dictionary system for consistent tagging

**Configuration System** (`src/config.rs`):
- `LifeConfig`: High-level configuration for life directory structure
- `ProcessingConfig`: Lower-level processing options (duplicate handling, hash length)
- `DuplicateHandling`: Skip, AppendHash, Overwrite, or Error strategies

**Generic Processing Pipeline**:
- `FileProcessor<E, F, N, O, R>`: Generic processor with pluggable components
- `E: MetadataParser` - EXIF/metadata extraction (RexifParser)
- `F: FileManager` - File operations (StdFileManager)
- `N: NamingStrategy` - File naming (TimestampNamingStrategy, CustomNamingStrategy)
- `O: OrganizationStrategy` - Directory structure (MonthlyOrganizer, YearlyOrganizer, etc.)
- `R: BasePathResolver` - Path resolution (LifeDirectoryResolver)

**Interactive Components** (`src/interactive.rs`):
- Document processing requires user input for date, description, and tags
- Tag system with similarity matching and validation
- Progress indicators and user prompts

**File Type Handling**:
- Media files: Automatic processing using EXIF metadata for timestamps
- Documents: Interactive processing with manual date/description/tag input
- Uses `infer` crate for MIME type detection

### Module Structure
- `cli.rs`: Command-line argument parsing using clap
- `config.rs`: Configuration structs and builders
- `document.rs`: Document input validation and processing
- `error.rs`: Custom error types and Result alias
- `filesystem.rs`: File operations and hashing
- `interactive.rs`: User interaction components
- `media.rs`: Media file metadata and type detection
- `metadata.rs`: EXIF parsing using rexif
- `naming.rs`: File naming strategies
- `organization.rs`: Directory organization strategies
- `paths.rs`: Path resolution strategies
- `processor.rs`: Main processing logic
- `tags.rs`: Tag dictionary and validation system

## Entry Points

- `process_life_directory_unified()`: Main entry point for processing a life directory
- `create_default_processor()`: Creates a processor with default configuration
- CLI accepts `--life-path` argument pointing to the root life directory

## Development Workflow

### Issue Resolution Process
- Agents resolving issues must follow this process: 
  1. create a branch based off main 
  2. implement a solution in that branch 
  3. fetch and rebase over main 
  4. fix conflicts, if any 
  5. open a pull request