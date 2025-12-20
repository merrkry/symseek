use crate::error::{Result, SymseekError};
use log::{debug, trace};
use regex::Regex;
use std::fs;
use std::path::Path;

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

pub fn detect_file_type(path: &Path) -> Result<FileType> {
    trace!("detect_file_type called for: {}", path.display());

    // Check if symlink via metadata (don't follow links)
    let metadata = fs::symlink_metadata(path).map_err(|e| SymseekError::Io {
        context: format!("Failed to read metadata for {}", path.display()),
        source: e,
    })?;

    if metadata.is_symlink() {
        trace!("Detected as symlink: {}", path.display());
        return Ok(FileType::Symlink);
    }

    // Read first 512 bytes to detect file type
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

    // Check for ELF magic number (0x7F 'E' 'L' 'F')
    if buffer.len() >= 4 && &buffer[0..4] == b"\x7FELF" {
        trace!("Detected as ELF binary: {}", path.display());
        return Ok(FileType::ElfBinary);
    }

    // Check for shebang and parse interpreter
    if buffer.starts_with(b"#!") {
        trace!("Shebang detected in: {}", path.display());

        // Find the end of the shebang line
        let newline_pos = buffer.iter().position(|&b| b == b'\n').unwrap_or(buffer.len());
        let shebang = &buffer[2..newline_pos];

        // Convert to string, ignore if not valid UTF-8
        if let Ok(shebang_str) = std::str::from_utf8(shebang) {
            let shebang_lower = shebang_str.to_lowercase();
            debug!("Shebang: {}", shebang_str.trim());

            // Detect interpreter type
            if shebang_lower.contains("bash") || shebang_lower.contains("/sh") {
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

    // Try to decode as UTF-8 to distinguish text vs binary
    match std::str::from_utf8(&buffer) {
        Ok(_) => {
            trace!("Detected as other text: {}", path.display());
            Ok(FileType::OtherText)
        }
        Err(_) => {
            trace!("Detected as other binary: {}", path.display());
            Ok(FileType::OtherBinary)
        }
    }
}

pub fn extract_shell_wrapper_target(path: &Path) -> Result<Option<String>> {
    trace!("extract_shell_wrapper_target called for: {}", path.display());

    let content = fs::read_to_string(path).map_err(|e| SymseekError::Io {
        context: format!("Failed to read shell script {}", path.display()),
        source: e,
    })?;

    // Pattern matches: exec "/path/to/binary" or exec '/path/to/binary'
    // Also handles: exec -a name "/path" or exec -- "/path"
    // The pattern is: exec (with optional flags) followed by a quoted path
    let re = Regex::new(r#"(?m)^\s*exec\s+(?:(?:-[a-z]\s+\S+|\-\-)\s+)*["']([^"']+)["']"#).unwrap();

    if let Some(caps) = re.captures(&content)
        && let Some(matched) = caps.get(1)
    {
        let target_path = matched.as_str();
        debug!("Found exec target in shell script: {}", target_path);

        // If path is relative, resolve it relative to script's parent directory
        let resolved_path = if Path::new(target_path).is_absolute() {
            target_path.to_string()
        } else {
            let parent = path.parent().unwrap_or_else(|| Path::new("/"));
            let joined = parent.join(target_path);
            joined.to_string_lossy().to_string()
        };

        trace!("Resolved wrapper target: {}", resolved_path);
        return Ok(Some(resolved_path));
    }

    trace!("No exec pattern found in shell script: {}", path.display());
    Ok(None)
}

pub fn extract_binary_wrapper_target(path: &Path) -> Result<Option<String>> {
    trace!("extract_binary_wrapper_target called for: {}", path.display());

    let path_str = path.to_string_lossy();

    // Only process if path contains /nix/store (optimization)
    if !path_str.contains("/nix/store") {
        trace!("Not a /nix/store binary, skipping wrapper extraction");
        return Ok(None);
    }

    trace!("Reading binary to extract embedded strings: {}", path.display());

    let content = fs::read(path).map_err(|e| SymseekError::Io {
        context: format!("Failed to read binary {}", path.display()),
        source: e,
    })?;

    // Extract null-terminated strings from binary
    let mut strings = Vec::new();
    let mut current = Vec::new();

    for &byte in &content {
        if byte == 0 {
            if !current.is_empty() {
                if let Ok(s) = String::from_utf8(current.clone()) {
                    strings.push(s);
                }
                current.clear();
            }
        } else if (32..=126).contains(&byte) {
            // Printable ASCII
            current.push(byte);
        } else {
            current.clear();
        }
    }

    trace!("Extracted {} strings from binary", strings.len());

    // Look for pattern: /nix/store/...-wrapped or /nix/store/.../bin/...
    let wrapped_re = Regex::new(r"^/nix/store/[^/]+/bin/\.[^/]+-wrapped$").unwrap();
    let bin_re = Regex::new(r"^/nix/store/[^/]+/bin/[^/]+$").unwrap();

    for s in strings {
        if (wrapped_re.is_match(&s) || bin_re.is_match(&s)) && s != path_str {
            let candidate = Path::new(&s);
            // Verify it exists
            if candidate.exists() {
                debug!("Found binary wrapper target: {}", s);
                return Ok(Some(s));
            }
            trace!("Candidate path does not exist: {}", s);
        }
    }

    trace!("No wrapper target found in binary");
    Ok(None)
}

pub fn extract_script_wrapper_target(
    _path: &Path,
    _script_type: crate::core::types::ScriptType,
) -> Result<Option<String>> {
    // For now, only shell scripts are implemented
    // Python, Perl, and other script types are reserved for future work
    // Return None for non-shell scripts

    // This is a stub that will be expanded in future versions
    // to support Python virtual environments, Perl wrappers, etc.
    Ok(None)
}
