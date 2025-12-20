use crate::core::detector::{self, FileType, NixStorePathDetector, WrapperDetector};
use crate::core::types::{FileKind, LinkType, ScriptType, SymlinkChain, WrapperKind};
use crate::error::{Result, SymseekError};
use log::{debug, trace};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Resolve a path by following symlinks and detecting wrappers.
///
/// Starting from the given path, follows all symlinks and detects wrapper
/// scripts/binaries, building a chain of all links found.
///
/// # Errors
///
/// Returns an error if:
/// - The path is not absolute
/// - A symlink cannot be read
/// - A cycle is detected in symlinks
/// - File metadata or content cannot be read
pub fn resolve(path: &Path) -> Result<SymlinkChain> {
    debug!("resolve called for: {}", path.display());

    if !path.is_absolute() {
        return Err(SymseekError::InvalidInput {
            message: "Path must be absolute".to_string(),
        });
    }

    let mut chain = SymlinkChain::new(path.to_path_buf());
    let mut current = path.to_path_buf();
    let mut visited = HashSet::new();
    let mut iteration = 0;

    loop {
        iteration += 1;
        trace!("Iteration {iteration}: processing {}", current.display());

        // Cycle detection
        if visited.contains(&current) {
            debug!("Cycle detected at: {}", current.display());
            return Err(SymseekError::CycleDetected { path: current });
        }
        visited.insert(current.clone());

        // Try symlink first
        let is_symlink = match current.read_link() {
            Ok(target) => {
                debug!(
                    "Found symlink: {} -> {}",
                    current.display(),
                    target.display()
                );
                let resolved = resolve_target(&current, &target);
                current.clone_from(&resolved);
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
                // Not a symlink - continue to wrapper detection
                trace!("Not a symlink: {}", current.display());
                false
            }
            Err(e) => {
                debug!("Error reading symlink {}: {}", current.display(), e);
                return Err(SymseekError::SymlinkResolution {
                    path: current.clone(),
                    reason: e.to_string(),
                });
            }
        };

        // Detect file type and extract wrapper
        trace!("Detecting file type for: {}", current.display());
        let file_type = detector::detect_file_type(&current)?;
        debug!("File type detected: {file_type:?}");

        // Use NixStorePathDetector for shell scripts and binaries
        let wrapper_result = match file_type {
            FileType::ShellScript => {
                let detector = NixStorePathDetector;
                detector.detect(&current)?.map(|target| {
                    (
                        target,
                        LinkType::Wrapper(WrapperKind::Text(ScriptType::Shell)),
                    )
                })
            }
            FileType::ElfBinary => {
                let detector = NixStorePathDetector;
                detector
                    .detect(&current)?
                    .map(|target| (target, LinkType::Wrapper(WrapperKind::Binary)))
            }
            // Python, Perl, and other script types: future work
            // For now, treat them as terminal nodes
            _ => None,
        };

        if let Some((target, link_type)) = wrapper_result {
            // Found a wrapper, add current path with wrapper type
            debug!("Found wrapper, following to: {target}");
            chain.add_link(current.clone(), false, link_type);
            // Add the wrapper target and continue
            current = PathBuf::from(target);
            continue;
        }

        // No wrapper found - add with appropriate type based on what we found earlier
        if is_symlink {
            chain.add_link(current.clone(), false, LinkType::Symlink);
            continue;
        }

        // Terminal node - determine what type of file it is
        trace!("Reached terminal node: {}", current.display());
        let file_type = detector::detect_file_type(&current)?;

        // Convert file type to the appropriate link type for the terminal node
        let terminal_link_type = match file_type {
            FileType::ElfBinary | FileType::OtherBinary => LinkType::Terminal(FileKind::Binary),
            _ => LinkType::Terminal(FileKind::Text), // Symlinks, scripts, and other files are "text" from user perspective
        };

        chain.add_link(current.clone(), true, terminal_link_type);
        break;
    }

    debug!(
        "Resolution complete: {} link(s) in chain",
        chain.links.len()
    );
    Ok(chain)
}

fn resolve_target(current: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        let parent = current.parent().unwrap_or_else(|| Path::new("/"));
        path_clean::clean(parent.join(target))
    }
}
