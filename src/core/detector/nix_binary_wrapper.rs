use crate::core::detector::{MAX_FILE_SIZE, WrapperDetector, extract_strings_from_binary};
use crate::error::{Result, SymseekError};
use log::debug;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

const DETECTOR_NAME: &str = "NixBinaryWrapperDetector";

static MAKE_C_WRAPPER_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"makeCWrapper\s+'([^']+)'").unwrap());

pub struct NixBinaryWrapperDetector;

impl WrapperDetector for NixBinaryWrapperDetector {
    fn detect(&self, path: &Path) -> Result<Option<String>> {
        debug!("{DETECTOR_NAME}: checking {}", path.display());

        let metadata = fs::metadata(path).map_err(|e| SymseekError::Io {
            context: format!("Failed to read metadata for {}", path.display()),
            source: e,
        })?;

        debug!("{DETECTOR_NAME}: file size = {} bytes", metadata.len());

        if metadata.len() > MAX_FILE_SIZE {
            debug!("{DETECTOR_NAME}: file too large");
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

        debug!(
            "{DETECTOR_NAME}: content length = {} chars",
            content_str.len()
        );

        if !content_str.contains("makeCWrapper") {
            debug!("{DETECTOR_NAME}: no makeCWrapper in content");
            return Ok(None);
        }

        // Normalize content by removing backslash-newline continuations
        let normalized = content_str.replace("\\\n", "");

        // Try to match makeCWrapper 'path' pattern
        if let Some(caps) = MAKE_C_WRAPPER_PATH_REGEX.captures(&normalized)
            && let Some(matched) = caps.get(1)
        {
            let candidate_str = matched.as_str();
            debug!("{DETECTOR_NAME}: found makeCWrapper path: {candidate_str}");

            // Only accept nix store paths as targets
            if !candidate_str.contains("/nix/store/") {
                debug!("{DETECTOR_NAME}: target is not a nix store path");
                return Ok(None);
            }

            let candidate_path = Path::new(candidate_str);
            if candidate_path != path {
                debug!("{DETECTOR_NAME}: found target: {candidate_str}");
                return Ok(Some(candidate_str.to_string()));
            }
        }

        debug!("{DETECTOR_NAME}: no target path");
        Ok(None)
    }
}
