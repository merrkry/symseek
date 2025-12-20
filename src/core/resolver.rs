use crate::core::detector::{self, FileType};
use crate::core::types::{LinkType, ScriptType, SymlinkChain, WrapperKind};
use crate::error::{Result, SymseekError};
use log::{debug, trace};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
        trace!("Iteration {}: processing {}", iteration, current.display());

        // Cycle detection
        if visited.contains(&current) {
            debug!("Cycle detected at: {}", current.display());
            return Err(SymseekError::CycleDetected { path: current });
        }
        visited.insert(current.clone());

        // Try symlink first
        match current.read_link() {
            Ok(target) => {
                debug!("Found symlink: {} -> {:?}", current.display(), target);
                let resolved = resolve_target(&current, &target);
                current.clone_from(&resolved);
                chain.add_link(resolved, false, LinkType::Symlink);
                continue;
            }
            Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
                // Not a symlink - continue to wrapper detection
                trace!("Not a symlink: {}", current.display());
            }
            Err(e) => {
                debug!("Error reading symlink {}: {}", current.display(), e);
                return Err(SymseekError::SymlinkResolution {
                    path: current.clone(),
                    reason: e.to_string(),
                })
            }
        }

        // Detect file type and extract wrapper
        trace!("Detecting file type for: {}", current.display());
        let file_type = detector::detect_file_type(&current)?;
        debug!("File type detected: {:?}", file_type);

        // Try to extract wrapper based on file type
        let wrapper_result = match file_type {
            FileType::ShellScript => {
                trace!("Attempting shell wrapper extraction");
                detector::extract_shell_wrapper_target(&current)?
                    .map(|target| (target, LinkType::Wrapper(WrapperKind::Text(ScriptType::Shell))))
            }
            FileType::ElfBinary => {
                trace!("Attempting binary wrapper extraction");
                detector::extract_binary_wrapper_target(&current)?
                    .map(|target| (target, LinkType::Wrapper(WrapperKind::Binary)))
            }
            // Python, Perl, and other script types: future work
            // For now, treat them as terminal nodes
            _ => None,
        };

        if let Some((target, link_type)) = wrapper_result {
            // Found a wrapper, continue following the chain
            debug!("Found wrapper, following to: {}", target);
            current = PathBuf::from(target.clone());
            chain.add_link(PathBuf::from(target), false, link_type);
            continue;
        }

        // Terminal node - no wrapper found
        trace!("Reached terminal node: {}", current.display());
        if let Some(last) = chain.links.last_mut() {
            last.is_final = true;
        }
        break;
    }

    debug!("Resolution complete: {} link(s) in chain", chain.links.len());
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
