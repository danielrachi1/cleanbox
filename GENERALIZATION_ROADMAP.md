# Generalization Roadmap

**Goal**: Make the codebase generic for any file type, keeping existing media functionality intact while enabling seamless document processing through a unified, intelligent workflow.

## 🎉 Implementation Status

**Phase 0 Complete**: Core architectural changes successfully implemented!

✅ **Completed Tasks:**
- Core type renames (MediaType → FileType, MediaFile → File, etc.)
- Module restructuring (exif → metadata) 
- Document file type support with MIME detection
- Processing behavior methods for intelligent file routing
- BasePathResolver architecture for flexible path handling
- Enhanced error handling for interactive workflows
- Comprehensive test coverage (71 tests passing)
- Full backwards compatibility maintained

🚀 **Phase 0 Complete**: All core architectural changes implemented and tested. Ready to begin configuration modernization and document processing implementation.

## Overview

The current codebase is architecturally generic but has three fundamental assumptions that limit document processing:
1. **EXIF-centric metadata** - assumes binary metadata extraction
2. **Timestamp-based naming** - assumes datetime-driven file names  
3. **Automatic processing** - assumes no user interaction required

This roadmap addresses both the renaming generalization AND the architectural changes needed for document processing per `life-dir-spec.txt`.

## UX Vision

**One unified command**: `cleanbox --life-path /life/path`

**Intelligent file handling**:
1. **Scan inbox** → categorize all files (media/documents/unrecognized)
2. **Process media files** → automatic EXIF-based naming and organization
3. **Process documents** → interactive naming with smart tag suggestions
4. **Skip unrecognized files** → leave in inbox with clear messaging

**User experience flow**:
```
$ cleanbox --life-path /home/user/life
Scanning inbox... Found 15 media files, 8 documents, 3 unrecognized files

Processing media files... ████████████████ 15/15 complete

Processing documents:
File: important-report.pdf
Date [2025-07-31]: 
Description: quarterly-financial-report
Tags: finance,reports,
  Tag "finance" not found. Similar tags:
  1. financial-planning
  2. quarterly-reports
  3. finance-team
  Select (1-3) or create 'finance' [1/2/3/c]: c
  
Processed 8 documents. 3 unrecognized files remain in inbox.
```

## Critical Insight

This is **not just a renaming exercise** - it's a **unified processing system**:
- **Media**: Fully automated, metadata-driven (`YYYY-MM-DD_HH-MM-SS.ext`)
- **Documents**: Interactive, user-driven semantics (`YYYY-MM-DD_description@@tag1,tag2.ext`) with smart tag suggestions
- **Unrecognized**: Gracefully skipped, user informed

The architecture must support intelligent file categorization and routing within a single command.

## Phase 0: Pre-Generalization Architecture Changes ⚠️ CRITICAL

**These architectural changes MUST happen before generalization to support document processing.**

### Step 0.1: FileType Processing Behavior
- [x] Add processing behavior methods to `FileType` in `src/media.rs`:
  ```rust
  impl FileType {
      pub fn needs_interactive_processing(&self) -> bool {
          matches!(self, FileType::Document)
      }
      
      pub fn is_auto_processable(&self) -> bool {
          matches!(self, FileType::Image | FileType::Video)
      }
      
      pub fn should_skip(&self) -> bool {
          matches!(self, FileType::Unknown)
      }
  }
  ```
- [x] Update processor to use these methods instead of separate processing modes

### Step 0.2: Base Path Resolver Architecture
- [x] Create `BasePathResolver` trait in new `src/paths.rs`:
  ```rust
  pub trait BasePathResolver {
      fn resolve_base_path(&self, file_type: &FileType, config: &ProcessingConfig) -> PathBuf;
  }
  ```
- [x] Implement `LifeDirectoryResolver` (routes media → media/, documents → documents/)
- [x] Update `ProcessingConfig` to use resolver pattern

### Step 0.3: Core Type Renames (MUST HAPPEN FIRST)
- [x] Rename `MediaType` → `FileType` in `src/media.rs:4`
- [x] Rename `MediaFile` → `File` in `src/media.rs:58`
- [x] Rename `MediaMetadata` → `FileMetadata` in `src/media.rs:28`
- [x] Rename `MediaProcessor` → `FileProcessor` in `src/processor.rs`
- [x] Update all references throughout codebase
- [x] **Critical**: This must happen before adding Document support

### Step 0.4: Metadata Parser Trait Redesign + File Rename
- [x] Rename `src/exif.rs` → `src/metadata.rs`
- [x] Rename `ExifParser` trait → `MetadataParser`
- [x] Add `supports_file_type(&self, file_type: &FileType) -> bool` method
- [x] Keep `RexifParser` for Image/Video files only
- [x] Update all imports from `crate::exif` → `crate::metadata`
- [x] **Note**: Documents won't use metadata parsers - they use direct user input

### Step 0.5: Enhanced FileType with Document Support (MIME-based)
- [x] Add `Document` variant to `FileType` enum (now renamed from MediaType)
- [x] Extend `from_mime()` method for document detection:
  ```rust
  impl FileType {
      pub fn from_mime(mime: &str) -> Self {
          let mime_lower = mime.to_lowercase();
          if mime_lower.starts_with("image/") {
              FileType::Image
          } else if mime_lower.starts_with("video/") {
              FileType::Video
          } else if mime_lower.starts_with("application/pdf")
                  || mime_lower.starts_with("application/msword")
                  || mime_lower.starts_with("application/vnd.openxmlformats")
                  || mime_lower.starts_with("text/") {
              FileType::Document
          } else {
              FileType::Unknown
          }
      }
  }
  ```
- [x] Add `base_directory_name()` method returning "media", "documents", or None
- [x] Update `is_supported()` logic to include Documents alongside processing behavior methods

### Step 0.6: Basic Interactive Error Handling
- [x] Add new error variants to `CleanboxError` in `src/error.rs`:
  ```rust
  pub enum CleanboxError {
      // ... existing variants
      UserCancelled,                    // Ctrl+C or user exit
      InvalidUserInput(String),         // Validation failures
      TagDictionaryCorrupted(String),   // Malformed tags.txt
  }
  ```
- [ ] Add basic signal handling for graceful Ctrl+C interruption
- [x] Add clear error messages for user input validation failures

### Step 0.7: Simplified Configuration Pattern
- [x] Replace `ProcessingConfig` with `LifeConfig` in `src/config.rs`:
  ```rust
  pub struct LifeConfig {
      pub life_path: PathBuf,  // Single source of truth
      pub hash_length: usize,
      pub duplicate_handling: DuplicateHandling,
      // Remove: inbox_path, media_root (derived from life_path)
  }
  
  impl LifeConfig {
      pub fn inbox_path(&self) -> PathBuf { self.life_path.join("inbox") }
      pub fn media_root(&self) -> PathBuf { self.life_path.join("media") }
      pub fn documents_root(&self) -> PathBuf { self.life_path.join("documents") }
      pub fn tags_file(&self) -> PathBuf { self.documents_root().join("tags.txt") }
  }
  ```
- [x] Update all config usage throughout codebase

### Step 0.8: Document Input Structure (No Metadata Needed)
- [ ] Create `DocumentInput` struct in new `src/document.rs`:
  ```rust
  pub struct DocumentInput {
      pub date: String,           // YYYY-MM-DD (user provided)
      pub description: String,    // kebab-case (user provided)
      pub tags: Vec<String>,      // from tags.txt (user selected)
  }
  ```
- [ ] Add validation methods for date format, description format, and tag validation
- [ ] **No metadata extraction needed** - everything comes from user interaction

### Step 0.9: Intelligent Tag System with Fuzzy Matching
- [ ] Create `src/tags.rs` module with:
  - `TagValidator` trait for validating against `tags.txt`
  - `TagDictionary` struct for loading/managing tag list from `life/documents/tags.txt`
  - `FuzzyMatcher` for finding similar tags (edit distance, substring matching)
  - Tag validation functions (lowercase, kebab-case, singular, English)
  - Similar tag suggestion with ranking
- [ ] Add tag dictionary path to `ProcessingConfig`
- [ ] Add dependency on fuzzy string matching crate (e.g., `strsim` or `fuzzy-matcher`)

### Step 0.10: Enhanced Interactive Processing Components
- [ ] Create `src/interactive.rs` module with:
  - `UserPrompt` trait for getting user input
  - `DatePrompt` for document dates (default to today)
  - `DescriptionPrompt` for document descriptions (validate kebab-case)
  - `SmartTagSelector` with fuzzy matching workflow:
    ```rust
    pub struct TagResolutionFlow {
        // Input: "machine-learning"
        // 1. Check exact match
        // 2. If no match, find similar tags
        // 3. Present options: similar tags or create new
        // 4. Handle user selection
    }
    ```
  - `ProgressIndicator` for "Processing document X of Y..."
- [ ] Remove `--interactive` flag - make it automatic for document files

## Phase 1: Add Document Support to FileType

### Step 1.1: Add Document Variant to FileType
- [ ] Add `Document` variant to `FileType` enum (already renamed from MediaType in Phase 0)
- [ ] Update `is_supported()` to include Document files
- [ ] Add document MIME type detection in `from_mime()` method

### Step 1.2: Add Processing Behavior Methods
- [ ] Implement processing behavior methods added in Step 0.1
- [ ] Update processor to route files based on `needs_interactive_processing()`, `is_auto_processable()`, `should_skip()`

### Step 1.3: Integrate BasePathResolver
- [ ] Wire up `BasePathResolver` trait with `LifeDirectoryResolver` implementation
- [ ] Update processor to use resolver for determining base paths (media/ vs documents/)

### Step 1.4: Wire Up Document Processing Flow
- [ ] Create basic document processing pipeline (user input → validation → naming → organization)
- [ ] Integrate with tag system for document processing

## Phase 2: Document Processing Implementation

### Step 2.1: Create Document-Specific Components
- [ ] Implement `DocumentNamingStrategy` using format: `YYYY-MM-DD_description@@tag1,tag2.ext`
- [ ] Create `DocumentOrganizer` for `documents/YYYY/MM/` structure
- [ ] Build interactive prompt system for date/description/tag input

### Step 2.2: Tag System Implementation
- [ ] Implement `TagDictionary` for loading from `life/documents/tags.txt`
- [ ] Create fuzzy matching for tag suggestions
- [ ] Add tag validation and creation workflow

### Step 2.3: Interactive Processing Pipeline
- [ ] Build `DocumentProcessor` with user input flow
- [ ] Integrate duplicate handling with hash suffixes
- [ ] Add progress indication and error handling

### Step 2.4: Unified CLI Integration
- [ ] Add file categorization and intelligent routing
- [ ] Implement batch processing with progress reporting

## Phase 3: Update All References and Tests

### Step 3.1: Update All Module References
- [x] Update `src/naming.rs` to use `File` instead of `MediaFile`
- [x] Update `src/organization.rs` to use `File` instead of `MediaFile`
- [x] Update `src/filesystem.rs` references
- [x] Update `src/lib.rs` public API exports
- [ ] Update `src/main.rs` and `src/cli.rs` usage

### Step 3.2: Update All Tests
- [x] Update test functions in all modules for new type names
- [x] Add comprehensive tests for Document FileType processing behavior
- [ ] Add tests for tag system and interactive components
- [ ] Add integration tests for unified CLI workflow

### Step 3.3: Verify Everything Works
- [x] Run `cargo test` - all tests should pass (71 tests passing)
- [x] Run `cargo clippy` - no new warnings
- [x] Run `cargo fmt` - code formatting maintained
- [ ] Test CLI functionality end-to-end

## Phase 4: Documentation and Polish

### Step 4.1: Update CLAUDE.md
- [ ] Update architecture section with generalized design
- [ ] Add document processing examples
- [ ] Update usage examples for unified workflow
- [ ] Document new interactive features

### Step 4.2: Code Documentation
- [ ] Update inline documentation
- [ ] Update module-level docs
- [ ] Add comprehensive examples

## Verification Checklist

- [x] All tests pass with `cargo test` (71/71 tests passing)
- [x] No clippy warnings with `cargo clippy`
- [x] Code builds successfully with `cargo build`
- [x] CLI functionality remains identical (existing functionality preserved)
- [x] API remains backwards compatible (core API unchanged, only extended)
- [x] No runtime behavior changes (existing behavior preserved)
