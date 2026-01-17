# AGENTS.md

## Project Goal

Trace symlinks and wrapper scripts (e.g., NixOS wrappers) recursively to show the complete chain from an original binary to its final target.

## Tech Stack

- **Language**: Rust 2024
- **CLI**: clap 4.x
- **Error Handling**: thiserror
- **Logging**: log + env_logger
- **Serialization**: serde + serde_json
- **Utilities**: path-clean (path normalization), regex
- **Testing**: assert_fs, tempfile, assert_cmd, pretty_assertions

## Architecture

```
symseek/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Module manifest
│   ├── cli/args.rs       # CLI argument parsing
│   ├── core/
│   │   ├── types.rs      # Data types (SymlinkChain, LinkType, FileLocation)
│   │   ├── resolver.rs   # Symlink chain resolution
│   │   ├── detector.rs   # File type detection (ELF, scripts, symlinks)
│   │   └── search.rs     # PATH/cwd file search
│   ├── error.rs          # SymseekError enum
│   └── output/
│       ├── formatter.rs  # Tree output
│       ├── json.rs       # JSON output
│       └── styles.rs     # Tree characters
├── tests/common/mod.rs   # Shared test fixtures
└── Cargo.toml
```

## Detection Algorithms

### File Type Detection (`detector.rs`)

1. Check if symlink via `symlink_metadata`
2. Read first 512 bytes
3. Match ELF magic bytes (`0x7f ELF`) → ELF binary
4. Match shebang (`#!`) → Shell/Python/Perl script
5. Check UTF-8 validity → text or binary

### NixOS Wrapper Detection (`NixStorePathDetector`)

1. Check if path contains `/nix/store/`
2. Limit file size to 1 MiB
3. Read file content (text or extract printable strings from binary)
4. Match regex `/nix/store/[a-z0-9]+-[^/\s]+(?:/[^/\s]+)*`
5. Normalize program names (strip `.`, `-wrapped`, `-unwrapped` suffixes)
6. Verify target exists and is a different path with matching program name

### Symlink Resolution (`resolver.rs`)

1. Validate absolute path
2. Loop following links until terminal:
   - Track visited paths in `HashSet` for cycle detection
   - Read `read_link()` for symlinks
   - Detect wrappers via `WrapperDetector` trait
   - Detect file type via `detect_file_type()`
3. Build `SymlinkChain` with `SymlinkNode` entries

### File Search (`search.rs`)

- If name contains path separator: search in current directory
- If just binary name: search all `PATH` entries
- Return `FileLocation::CurrentDirectory(PathBuf)` or `FileLocation::PathEnvironment(Vec<PathBuf>)`
