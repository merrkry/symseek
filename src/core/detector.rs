use crate::error::{Result, SymseekError};
use log::{debug, trace};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

const MAX_FILE_SIZE: u64 = 1_048_576; // 1 MiB

static NIX_STORE_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/nix/store/[a-z0-9]+-[^/\s]+(?:/[^/\s]+)*").unwrap());

#[derive(Debug, Clone)]
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

    let mut buffer = vec![0u8; 512];
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

    if buffer.len() >= 4 && buffer[0..4] == [0x7f, b'E', b'L', b'F'] {
        trace!("Detected as ELF binary: {}", path.display());
        return Ok(FileType::ElfBinary);
    }

    if buffer.starts_with(b"#!") {
        trace!("Shebang detected in: {}", path.display());

        let newline_pos = buffer
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(buffer.len());
        let shebang = &buffer[2..newline_pos];

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
/// used by NixOS wrappers.
fn normalize_program_name(name: &str) -> &str {
    let mut result = name;

    if let Some(stripped) = result.strip_prefix('.') {
        result = stripped;
    }

    if result.ends_with("unwrapped") {
        result = &result[..result.len() - 10];
    } else if result.ends_with("wrapped") {
        result = &result[..result.len() - 8];
    }

    result
}

/// Check if two paths have the same normalized program name.
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
        } else if (32..=126).contains(&byte) {
            current.push(byte);
        } else {
            current.clear();
        }
    }

    result
}
