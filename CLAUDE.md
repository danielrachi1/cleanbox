# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cleanbox is a Rust CLI tool that intelligently organizes files by processing both media and documents from an inbox directory. Media files (images/videos) are automatically processed using EXIF metadata for timestamp-based naming, while documents are processed interactively with user-provided semantic information. The codebase features a unified, modular architecture that handles different file types through intelligent routing.

**Note**: All core functionality is fully implemented and tested with 121 comprehensive tests covering all modules and integration scenarios.

## Development Guidelines

- Always resolve all lints suggested by clippy.

## Architecture

[... rest of the existing content remains unchanged ...]