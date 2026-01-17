use crate::core::detector::{
    NIX_STORE_PATH_REGEX, WrapperDetector, extract_strings_from_binary, programs_match,
};
use crate::error::{Result, SymseekError};
use log::{debug, trace};
use std::fs;
use std::path::Path;

const DETECTOR_NAME: &str = "NixProgramNameDetector";

const MAX_FILE_SIZE: u64 = 1_048_576;

pub struct NixProgramNameDetector;

impl WrapperDetector for NixProgramNameDetector {
    fn detect(&self, path: &Path) -> Result<Option<String>> {
        let path_str = path.to_string_lossy();
        trace!("{DETECTOR_NAME}: checking {path_str}");

        if !path_str.contains("nix") {
            trace!("{DETECTOR_NAME}: not a nix path, skipping");
            return Ok(None);
        }

        let metadata = fs::metadata(path).map_err(|e| SymseekError::Io {
            context: format!("Failed to read metadata for {}", path.display()),
            source: e,
        })?;

        if metadata.len() > MAX_FILE_SIZE {
            trace!("{DETECTOR_NAME}: file too large");
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
                while candidate_str.ends_with('"')
                    || candidate_str.ends_with('\'')
                    || candidate_str.ends_with('$')
                {
                    candidate_str = &candidate_str[..candidate_str.len() - 1];
                }

                let candidate_path = Path::new(candidate_str);
                trace!("{DETECTOR_NAME}: found path in content: {candidate_str}");

                let names_match = programs_match(path, candidate_path);
                let is_file = candidate_path.is_file();
                let not_same = candidate_path != path;

                trace!("  names_match={names_match}, is_file={is_file}, not_same={not_same}");

                if names_match && is_file && not_same {
                    debug!("{DETECTOR_NAME}: found matching path: {candidate_str}");
                    return Ok(Some(candidate_str.to_string()));
                }
            }
        }

        trace!("{DETECTOR_NAME}: no target path");
        Ok(None)
    }
}
