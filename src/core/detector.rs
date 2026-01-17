pub mod nix_binary_wrapper;
pub mod nix_program_name;

use crate::error::{Result, SymseekError};
use log::{debug, trace};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

const MAX_FILE_SIZE: u64 = 1_048_576;
const BUFFER_SIZE: usize = 512;
const ELF_MAGIC: &[u8] = &[0x7f, b'E', b'L', b'F'];
const SHEBANG_PREFIX: &[u8] = b"#!";
const PRINTABLE_ASCII_MIN: u8 = 32;
const PRINTABLE_ASCII_MAX: u8 = 126;
const WRAPPED_SUFFIX: &str = "-wrapped";
const UNWRAPPED_SUFFIX: &str = "-unwrapped";

pub static NIX_STORE_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/nix/store/[a-z0-9]+-[^/\s]+(?:/[^/\s]+)*").unwrap());

pub static MAKE_C_WRAPPER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"makeCWrapper\s+'([^']+)'").unwrap());

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
    let bytes_read = {
        use std::io::Read;
        fs::File::open(path)
            .and_then(|mut f| f.read(&mut buffer))
            .map_err(|e| SymseekError::Io {
                context: format!("Failed to read {}", path.display()),
                source: e,
            })?
    };

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

pub trait WrapperDetector {
    /// Detect if the given path is a wrapper for another executable.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or analyzed.
    fn detect(&self, path: &Path) -> Result<Option<String>>;
}

#[must_use]
pub fn normalize_program_name(name: &str) -> &str {
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

pub fn programs_match(current: &Path, candidate: &Path) -> bool {
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

#[must_use]
pub fn extract_strings_from_binary(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut current = Vec::new();

    for &byte in bytes {
        if byte == 0 || byte == b'\n' {
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
    }

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
            let binary_data = &[0x89, 0x50, 0x4E, 0x47];
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

        #[test]
        fn test_nix_binary_wrapper_detector() {
            use super::super::nix_binary_wrapper::NixBinaryWrapperDetector;

            let temp = TempDir::new().unwrap();

            let bin_dir = temp.child("bin");
            bin_dir.create_dir_all().unwrap();

            let script_content = "#!/bin/bash\n# Generated by makeCWrapper\nmakeCWrapper '/nix/store/abc123-quickshell-0.2.1/bin/qs'\n";
            let wrapper = bin_dir.child("noctalia-shell-wrapped");
            wrapper.write_str(script_content).unwrap();

            let detector = NixBinaryWrapperDetector;
            let result = detector.detect(wrapper.path()).unwrap();
            assert!(result.is_some());
            assert_eq!(result.unwrap(), "/nix/store/abc123-quickshell-0.2.1/bin/qs");
        }

        #[test]
        fn test_nix_binary_wrapper_detector_not_nix_path() {
            use super::super::nix_binary_wrapper::NixBinaryWrapperDetector;

            let temp = TempDir::new().unwrap();

            let bin_dir = temp.child("bin");
            bin_dir.create_dir_all().unwrap();

            let script_content = "#!/bin/bash\nmakeCWrapper '/usr/local/bin/qs'\n";
            let wrapper = bin_dir.child("wrapper");
            wrapper.write_str(script_content).unwrap();

            let detector = NixBinaryWrapperDetector;
            let result = detector.detect(wrapper.path()).unwrap();
            assert!(result.is_none());
        }
    }
}
