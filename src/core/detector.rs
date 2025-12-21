use crate::error::{Result, SymseekError};
use log::{debug, trace};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

// File type detection constants
const MAX_FILE_SIZE: u64 = 1_048_576; // 1 MiB
const BUFFER_SIZE: usize = 512;
const ELF_MAGIC: &[u8] = &[0x7f, b'E', b'L', b'F'];
const SHEBANG_PREFIX: &[u8] = b"#!";
const PRINTABLE_ASCII_MIN: u8 = 32;
const PRINTABLE_ASCII_MAX: u8 = 126;
const WRAPPED_SUFFIX: &str = "-wrapped";
const UNWRAPPED_SUFFIX: &str = "-unwrapped";

// Nix store path detection regex
static NIX_STORE_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/nix/store/[a-z0-9]+-[^/\s]+(?:/[^/\s]+)*").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    Symlink,
    ShellScript,
    PythonScript,
    PerlScript,
    OtherScript,
    ElfBinary,
    OtherBinary,
    OtherText,
}

/// Detect the type of a file by examining its content.
///
/// Checks file metadata and content to determine if it's a symlink, shell script,
/// Python script, Perl script, ELF binary, or other text/binary file.
///
/// Uses the ELF magic number for binary detection and shebangs for script detection.
///
/// # Errors
///
/// Returns an error if file metadata or content cannot be read.
pub fn detect_file_type(path: &Path) -> Result<FileType> {
    trace!("detect_file_type called for: {}", path.display());

    let metadata = fs::symlink_metadata(path).map_err(|e| SymseekError::Io {
        context: format!("Failed to read metadata for {}", path.display()),
        source: e,
    })?;

    if metadata.is_symlink() {
        trace!("Detected as symlink: {}", path.display());
        return Ok(FileType::Symlink);
    }

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let bytes_read = fs::File::open(path)
        .and_then(|mut f| {
            use std::io::Read;
            f.read(&mut buffer)
        })
        .map_err(|e| SymseekError::Io {
            context: format!("Failed to read {}", path.display()),
            source: e,
        })?;

    buffer.truncate(bytes_read);
    trace!("Read {} bytes from {}", bytes_read, path.display());

    if buffer.len() >= ELF_MAGIC.len() && buffer[0..ELF_MAGIC.len()] == *ELF_MAGIC {
        trace!("Detected as ELF binary: {}", path.display());
        return Ok(FileType::ElfBinary);
    }

    if buffer.starts_with(SHEBANG_PREFIX) {
        trace!("Shebang detected in: {}", path.display());

        let newline_pos = buffer
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(buffer.len());
        let shebang = &buffer[SHEBANG_PREFIX.len()..newline_pos];

        if let Ok(shebang_str) = std::str::from_utf8(shebang) {
            let shebang_lower = shebang_str.to_lowercase();
            debug!("Shebang: {}", shebang_str.trim());

            if shebang_lower.contains("bash") || shebang_lower.contains("sh") {
                trace!("Detected as shell script: {}", path.display());
                return Ok(FileType::ShellScript);
            } else if shebang_lower.contains("python") {
                trace!("Detected as Python script: {}", path.display());
                return Ok(FileType::PythonScript);
            } else if shebang_lower.contains("perl") {
                trace!("Detected as Perl script: {}", path.display());
                return Ok(FileType::PerlScript);
            }
            trace!("Detected as other script: {}", path.display());
            return Ok(FileType::OtherScript);
        }
    }

    if std::str::from_utf8(&buffer).is_ok() {
        trace!("Detected as other text: {}", path.display());
        Ok(FileType::OtherText)
    } else {
        trace!("Detected as other binary: {}", path.display());
        Ok(FileType::OtherBinary)
    }
}

/// Trait for wrapper detection strategies.
///
/// Implementations can detect different types of wrappers by examining file
/// content and determining if a file wraps another executable.
pub trait WrapperDetector {
    /// Detect if the given path is a wrapper for another executable.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or analyzed.
    fn detect(&self, path: &Path) -> Result<Option<String>>;

    /// Return the name of this detector for logging purposes.
    fn name(&self) -> &'static str;
}

/// Normalize a program name by stripping common prefixes and suffixes.
///
/// Removes leading dots (`.`) and trailing suffixes (`-wrapped`, `-unwrapped`)
/// used by NixOS wrappers. For example:
/// - `.nvim-wrapped` → `nvim`
/// - `python-unwrapped` → `python`
/// - `gcc` → `gcc`
fn normalize_program_name(name: &str) -> &str {
    let mut result = name;

    if let Some(stripped) = result.strip_prefix('.') {
        result = stripped;
    }

    if result.ends_with(UNWRAPPED_SUFFIX) {
        result = &result[..result.len() - UNWRAPPED_SUFFIX.len()];
    } else if result.ends_with(WRAPPED_SUFFIX) {
        result = &result[..result.len() - WRAPPED_SUFFIX.len()];
    }

    result
}

/// Check if two paths have the same normalized program name.
///
/// Returns `true` if both paths refer to the same program after normalizing
/// wrapped/unwrapped variants and dot prefixes. For example:
/// - `/usr/bin/nvim` and `/nix/store/xxx/bin/nvim-wrapped` match
/// - `/usr/bin/nvim` and `/usr/bin/python` do not match
fn programs_match(current: &Path, candidate: &Path) -> bool {
    let current_name = current
        .file_name()
        .and_then(|n| n.to_str())
        .map_or("", normalize_program_name);

    let candidate_name = candidate
        .file_name()
        .and_then(|n| n.to_str())
        .map_or("", normalize_program_name);

    !current_name.is_empty() && current_name == candidate_name
}

pub struct NixStorePathDetector;

impl WrapperDetector for NixStorePathDetector {
    fn detect(&self, path: &Path) -> Result<Option<String>> {
        let path_str = path.to_string_lossy();
        trace!("NixStorePathDetector: checking {path_str}");

        if !path_str.contains("nix") {
            trace!("NixStorePathDetector: not a nix path, skipping");
            return Ok(None);
        }

        let metadata = fs::metadata(path).map_err(|e| SymseekError::Io {
            context: format!("Failed to read metadata for {}", path.display()),
            source: e,
        })?;

        if metadata.len() > MAX_FILE_SIZE {
            trace!("NixStorePathDetector: file too large");
            return Ok(None);
        }

        let content_str = if let Ok(text) = fs::read_to_string(path) {
            text
        } else {
            let bytes = fs::read(path).map_err(|e| SymseekError::Io {
                context: format!("Failed to read file {}", path.display()),
                source: e,
            })?;

            extract_strings_from_binary(&bytes)
        };

        for caps in NIX_STORE_PATH_REGEX.captures_iter(&content_str) {
            if let Some(matched) = caps.get(0) {
                let mut candidate_str = matched.as_str();
                // Remove trailing quotes and special characters
                while candidate_str.ends_with('"')
                    || candidate_str.ends_with('\'')
                    || candidate_str.ends_with('$')
                {
                    candidate_str = &candidate_str[..candidate_str.len() - 1];
                }

                let candidate_path = Path::new(candidate_str);
                trace!("NixStorePathDetector: found path in content: {candidate_str}");

                let names_match = programs_match(path, candidate_path);
                let exists = candidate_path.exists();
                let not_same = candidate_path != path;

                trace!("  names_match={names_match}, exists={exists}, not_same={not_same}");

                if names_match && exists && not_same {
                    debug!("NixStorePathDetector: found matching path: {candidate_str}");
                    return Ok(Some(candidate_str.to_string()));
                }
            }
        }

        trace!("NixStorePathDetector: no target path");
        Ok(None)
    }

    fn name(&self) -> &'static str {
        "NixStorePathDetector"
    }
}

/// Extract null-terminated strings from binary data.
///
/// Scans through binary data and extracts sequences of printable ASCII characters
/// (32-126) separated by null bytes or non-printable bytes. Useful for finding
/// embedded file paths and strings in binary files.
///
/// # Example
/// ```ignore
/// let binary = b"path\0data\0";
/// let result = extract_strings_from_binary(binary);
/// // result contains "path\ndata\n"
/// ```
fn extract_strings_from_binary(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut current = Vec::new();

    for &byte in bytes {
        if byte == 0 {
            if !current.is_empty() {
                if let Ok(s) = String::from_utf8(current.clone()) {
                    result.push_str(&s);
                    result.push('\n');
                }
                current.clear();
            }
        } else if (PRINTABLE_ASCII_MIN..=PRINTABLE_ASCII_MAX).contains(&byte) {
            current.push(byte);
        } else {
            current.clear();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // normalize_program_name tests
    #[test]
    fn test_normalize_program_name_basic() {
        assert_eq!(normalize_program_name("nvim"), "nvim");
        assert_eq!(normalize_program_name("python3"), "python3");
        assert_eq!(normalize_program_name("gcc"), "gcc");
    }

    #[test]
    fn test_normalize_program_name_wrapped() {
        assert_eq!(normalize_program_name("nvim-wrapped"), "nvim");
        assert_eq!(normalize_program_name("gcc-wrapped"), "gcc");
        assert_eq!(normalize_program_name("bash-wrapped"), "bash");
    }

    #[test]
    fn test_normalize_program_name_unwrapped() {
        assert_eq!(normalize_program_name("nvim-unwrapped"), "nvim");
        assert_eq!(normalize_program_name("python-unwrapped"), "python");
        assert_eq!(normalize_program_name("gcc-unwrapped"), "gcc");
    }

    #[test]
    fn test_normalize_program_name_dot_prefix() {
        assert_eq!(normalize_program_name(".nvim-wrapped"), "nvim");
        assert_eq!(normalize_program_name(".hidden"), "hidden");
        assert_eq!(normalize_program_name(".python-unwrapped"), "python");
    }

    #[test]
    fn test_normalize_program_name_edge_cases() {
        assert_eq!(normalize_program_name(""), "");
        // Note: normalize removes the suffix from the entire name
        // so "wrapped" only (7 chars) becomes "" after trying to remove 8 chars
        // The actual behavior here depends on how the code handles string slicing
    }

    // programs_match tests
    #[test]
    fn test_programs_match_exact() {
        let path1 = PathBuf::from("/usr/bin/nvim");
        let path2 = PathBuf::from("/nix/store/xxx/bin/nvim");
        assert!(programs_match(&path1, &path2));
    }

    #[test]
    fn test_programs_match_wrapped_variants() {
        let original = PathBuf::from("/usr/bin/nvim");
        let wrapped = PathBuf::from("/usr/bin/nvim-wrapped");
        let unwrapped = PathBuf::from("/nix/store/xxx/bin/nvim-unwrapped");

        assert!(programs_match(&original, &wrapped));
        assert!(programs_match(&original, &unwrapped));
        assert!(programs_match(&wrapped, &unwrapped));
    }

    #[test]
    fn test_programs_match_different_programs() {
        let nvim = PathBuf::from("/usr/bin/nvim");
        let vim = PathBuf::from("/usr/bin/vim");
        assert!(!programs_match(&nvim, &vim));
    }

    #[test]
    fn test_programs_match_dot_prefix() {
        let normal = PathBuf::from("/usr/bin/nvim");
        let dotted = PathBuf::from("/usr/bin/.nvim-wrapped");
        assert!(programs_match(&normal, &dotted));
    }

    #[test]
    fn test_programs_match_different_suffixes() {
        let path1 = PathBuf::from("/usr/bin/vim");
        let path2 = PathBuf::from("/usr/local/bin/nano");
        // Different program names should not match
        assert!(!programs_match(&path1, &path2));
    }

    // extract_strings_from_binary tests
    #[test]
    fn test_extract_strings_simple() {
        let binary = b"Hello\0World\0";
        let result = extract_strings_from_binary(binary);
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
    }

    #[test]
    fn test_extract_strings_with_nix_paths() {
        let binary = b"\x00\x01\x02/nix/store/abc123-pkg/bin/exe\0more data\0";
        let result = extract_strings_from_binary(binary);
        assert!(result.contains("/nix/store/abc123-pkg/bin/exe"));
        assert!(result.contains("more data"));
    }

    #[test]
    fn test_extract_strings_filters_non_printable() {
        // Test that non-printable bytes clear the buffer
        // "Valid\0" gets extracted, but "Other" (without null terminator after)
        // doesn't get extracted unless followed by null
        let binary = b"Valid\0\x01\x02\x03Other\0Next\0";
        let result = extract_strings_from_binary(binary);
        assert!(result.contains("Valid"));
        assert!(result.contains("Other"));
        assert!(result.contains("Next"));
    }

    #[test]
    fn test_extract_strings_empty() {
        let binary = b"";
        let result = extract_strings_from_binary(binary);
        assert_eq!(result, "");
    }

    #[test]
    fn test_extract_strings_only_binary() {
        let binary = &[0x01, 0x02, 0x03, 0x04, 0xff, 0xfe];
        let result = extract_strings_from_binary(binary);
        assert_eq!(result, "");
    }

    #[test]
    fn test_extract_strings_multiple_sequences() {
        let binary = b"first\0second\0third\0";
        let result = extract_strings_from_binary(binary);
        assert!(result.contains("first"));
        assert!(result.contains("second"));
        assert!(result.contains("third"));
    }

    // Filesystem-dependent tests (require tempfile)
    #[cfg(test)]
    mod fs_tests {
        use super::*;
        use assert_fs::TempDir;
        use assert_fs::prelude::*;
        use std::os::unix::fs::PermissionsExt;

        fn create_executable_script(
            dir: &TempDir,
            name: &str,
            content: &str,
        ) -> std::path::PathBuf {
            let path = dir.child(name);
            path.write_str(content).unwrap();
            let mut perms = fs::metadata(path.path()).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path.path(), perms).unwrap();
            path.to_path_buf()
        }

        #[test]
        fn test_detect_elf_binary() {
            let temp = TempDir::new().unwrap();
            let elf_magic = [0x7f, b'E', b'L', b'F', 0x02, 0x01, 0x01, 0x00];
            let file = temp.child("binary");
            file.write_binary(&elf_magic).unwrap();

            let file_type = detect_file_type(file.path()).unwrap();
            assert!(matches!(file_type, FileType::ElfBinary));
        }

        #[test]
        fn test_detect_shell_script() {
            let temp = TempDir::new().unwrap();
            let script = "#!/bin/bash\necho 'hello'\n";
            let path = create_executable_script(&temp, "script.sh", script);

            let file_type = detect_file_type(&path).unwrap();
            assert!(matches!(file_type, FileType::ShellScript));
        }

        #[test]
        fn test_detect_shell_script_variants() {
            let temp = TempDir::new().unwrap();
            let variants = vec![
                "#!/bin/sh",
                "#!/usr/bin/bash",
                "#!/usr/bin/env bash",
                "#!/bin/zsh",
            ];

            for shebang in variants {
                let content = format!("{shebang}\necho test");
                let path = create_executable_script(&temp, "test", &content);
                let detected = detect_file_type(&path).unwrap();
                assert!(matches!(detected, FileType::ShellScript));
            }
        }

        #[test]
        fn test_detect_python_script() {
            let temp = TempDir::new().unwrap();
            let script = "#!/usr/bin/python3\nprint('hello')\n";
            let path = create_executable_script(&temp, "script.py", script);

            let file_type = detect_file_type(&path).unwrap();
            assert!(matches!(file_type, FileType::PythonScript));
        }

        #[test]
        fn test_detect_perl_script() {
            let temp = TempDir::new().unwrap();
            let script = "#!/usr/bin/perl\nprint \"hello\\n\";\n";
            let path = create_executable_script(&temp, "script.pl", script);

            let file_type = detect_file_type(&path).unwrap();
            assert!(matches!(file_type, FileType::PerlScript));
        }

        #[test]
        fn test_detect_other_script() {
            let temp = TempDir::new().unwrap();
            let script = "#!/usr/bin/ruby\nputs 'hello'\n";
            let path = create_executable_script(&temp, "script.rb", script);

            let file_type = detect_file_type(&path).unwrap();
            assert!(matches!(file_type, FileType::OtherScript));
        }

        #[test]
        fn test_detect_plain_text() {
            let temp = TempDir::new().unwrap();
            let content = "This is plain text\nwith multiple lines\n";
            let file = temp.child("readme.txt");
            file.write_str(content).unwrap();

            let file_type = detect_file_type(file.path()).unwrap();
            assert!(matches!(file_type, FileType::OtherText));
        }

        #[test]
        fn test_detect_other_binary() {
            let temp = TempDir::new().unwrap();
            let binary_data = &[0x89, 0x50, 0x4E, 0x47]; // PNG magic
            let file = temp.child("image.png");
            file.write_binary(binary_data).unwrap();

            let file_type = detect_file_type(file.path()).unwrap();
            assert!(matches!(file_type, FileType::OtherBinary));
        }

        #[test]
        fn test_detect_symlink() {
            let temp = TempDir::new().unwrap();
            let target = temp.child("target");
            target.write_str("content").unwrap();
            let link = temp.child("link");
            link.symlink_to_file(target.path()).unwrap();

            let file_type = detect_file_type(link.path()).unwrap();
            assert!(matches!(file_type, FileType::Symlink));
        }

        #[test]
        fn test_detect_nonexistent_file() {
            let path = std::path::PathBuf::from("/nonexistent/file");
            let result = detect_file_type(&path);
            assert!(result.is_err());
        }
    }
}
