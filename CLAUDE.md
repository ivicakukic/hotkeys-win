# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

HotKeys is a Windows application that provides global keyboard shortcuts mapped to numeric keypad keys (1-9). It displays a 3x3 board when triggered by `Ctrl Alt NumPad_0` and allows users to execute predefined actions for specific applications.

## Architecture

The project follows a layered Rust architecture with clear separation of concerns:

### Core Layers

- `src/main.rs` - Application entry point
  - Command-line argument parsing (--config_dir, --board, dynamic params)
  - Resource path resolution (dev vs production)
  - Icon cache initialization
  - Logging setup and application lifecycle

- `src/framework/` - Application framework
  - `app_registry.rs` - Global application handler registration
  - `router.rs` - Window procedure message routing
  - `traits.rs` - Core traits (AppHandler, Window)

- `src/app/` - Application core
  - `app.rs` - Main Application struct, message loop, window procedure handling
  - `hook.rs` - Windows low-level keyboard hook implementation
  - `message.rs` - Inter-thread messaging structures
  - `board_manager.rs` - Board lifecycle and display management
  - `action_factory.rs` - Action creation with factory pattern and registry
  - `board_factory.rs` - Board creation with factory pattern and registry
  - `windows/` - Window implementations
    - `main.rs` - Main application window
    - `board.rs` - 3x3 board window display
    - `tray.rs` - System tray icon

### Domain & Data Layers

- `src/core/` - Core domain models and abstractions
  - `data.rs` - Domain data structures (Board, PadSet, Pad, ColorScheme, TextStyle, Detection)
  - `integration.rs` - Integration types (ActionType, BoardType, Param, ActionParams, BoardParams)
  - `repository.rs` - Repository trait definitions (SettingsRepository, SettingsRepositoryMut)
  - `resources.rs` - Resource path management and icon detection

- `src/settings/` - Configuration management
  - `settings.rs` - Settings implementation with repository pattern
  - `persistence.rs` - JSON file storage (loads/saves from resources/settings.json)
  - `validation.rs` - Settings validation

- `src/model/` - UI-specific models
  - `data.rs` - UI model types (PadId, Pad with UI extensions, Tag, Anchor, Color helpers)
  - `handle.rs` - Handle types for UI resources
  - `traits.rs` - Model trait definitions (Board trait)

### UI & Components

- `src/components/` - Board component implementations
  - `traits.rs` - Component traits (BoardComponent, UiEventHandler, UiEvent, UiEventResult)
  - `boards.rs` - Board component helpers
  - `state_machine.rs` - State machine wrapper for board navigation
  - `main_board.rs` - Static board implementation
  - `home_board.rs` - Home/detection board with dynamic icon loading
  - `settings_board.rs` - Settings editor board
  - `colors_board.rs` - Color scheme picker
  - `fonts_board.rs` - Font/text style picker
  - `controls.rs` - UI control abstractions
  - `result_helpers.rs` - Result handling utilities

- `src/ui/` - UI infrastructure
  - `components/` - Rendering components
    - `painter.rs` - GDI drawing utilities
    - `assets.rs` - UI asset management (fonts, colors)
    - `svg.rs` - SVG icon rendering with caching
    - `png.rs` - PNG image loading with caching
  - `dialogs/` - Modal dialog windows
    - `pad_editor.rs` - Pad action/board editor
    - `color_picker.rs` - Color selection dialog
    - `font_selector.rs` - Font selection dialog
    - `capture_dialog.rs` - Keyboard shortcut capture
  - `shared/` - Shared UI utilities
    - `layout.rs` - Layout calculation helpers
    - `utils.rs` - Window messaging and utility functions

### Input Layer

- `src/input/` - Input handling and script execution
  - `api.rs` - Windows SendInput API wrapper
  - `script.rs` - Input script builder and executor
  - `steps.rs` - Input step definitions
  - `capture.rs` - Keyboard input capture utilities
  - `keys/` - Key mapping
    - `vkey.rs` - Virtual key code definitions and mappings

## Common Commands

```bash
# Build the application
cargo build

# Build for release
cargo build --release

# Run in development
cargo run

# Check for compilation errors
cargo check

# Format code
cargo fmt

# Run clippy for linting
cargo clippy
```

## Development Notes

- Application runs in Windows subsystem mode (no console in release builds)
- Uses unsafe code for global app instance management
- Logging configured via `resources/log.toml`
- Windows resources embedded via `build.rs` and `resources.rc`
- Heavy use of Windows API through the `windows` crate
- Application profiles support hierarchical boards via Board actions