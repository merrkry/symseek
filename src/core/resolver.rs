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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use std::os::unix::fs::PermissionsExt;

    fn create_executable(dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
        let file = dir.child(name);
        file.write_binary(content).unwrap();
        let mut perms = std::fs::metadata(file.path()).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(file.path(), perms).unwrap();
        file.to_path_buf()
    }

    #[test]
    fn test_resolve_requires_absolute_path() {
        let relative = Path::new("relative/path");
        let result = resolve(relative);

        assert!(result.is_err());
        match result {
            Err(SymseekError::InvalidInput { message }) => {
                assert!(message.contains("absolute"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_resolve_single_symlink() {
        let temp = TempDir::new().unwrap();

        let target = create_executable(&temp, "target", b"#!/bin/bash\necho hello\n");
        let link = temp.child("link");
        link.symlink_to_file(&target).unwrap();

        let chain = resolve(link.path()).unwrap();

        assert_eq!(chain.links.len(), 2);
        assert!(matches!(chain.links[0].link_type, LinkType::Symlink));
        assert!(!chain.links[0].is_final);
        assert!(chain.links[1].is_final);
    }

    #[test]
    fn test_resolve_symlink_chain() {
        let temp = TempDir::new().unwrap();

        // Create chain: link1 -> link2 -> link3 -> target
        let target = create_executable(&temp, "target", b"#!/bin/bash\necho hello\n");

        let link3 = temp.child("link3");
        link3.symlink_to_file(&target).unwrap();

        let link2 = temp.child("link2");
        link2.symlink_to_file(link3.path()).unwrap();

        let link1 = temp.child("link1");
        link1.symlink_to_file(link2.path()).unwrap();

        let chain = resolve(link1.path()).unwrap();

        assert_eq!(chain.links.len(), 4);
        assert!(matches!(chain.links[0].link_type, LinkType::Symlink));
        assert!(matches!(chain.links[1].link_type, LinkType::Symlink));
        assert!(matches!(chain.links[2].link_type, LinkType::Symlink));
        assert!(chain.links[3].is_final);
    }

    #[test]
    fn test_resolve_relative_symlink() {
        let temp = TempDir::new().unwrap();

        let subdir = temp.child("subdir");
        subdir.create_dir_all().unwrap();

        let _target = create_executable(&temp, "target", b"#!/bin/bash\n");

        // Create symlink with relative path
        let link = subdir.child("link");
        std::os::unix::fs::symlink("../target", link.path()).unwrap();

        let chain = resolve(link.path()).unwrap();

        assert_eq!(chain.links.len(), 2);
        // Verify the target was resolved correctly
        let resolved_target = &chain.links[1].target;
        assert!(resolved_target.ends_with("target"));
        assert!(resolved_target.is_absolute());
    }

    #[test]
    fn test_resolve_cycle_detection() {
        let temp = TempDir::new().unwrap();

        let link1 = temp.child("link1");
        let link2 = temp.child("link2");

        // Create cycle: link1 -> link2 -> link1
        std::os::unix::fs::symlink(link2.path(), link1.path()).unwrap();
        std::os::unix::fs::symlink(link1.path(), link2.path()).unwrap();

        let result = resolve(link1.path());

        assert!(result.is_err());
        match result {
            Err(SymseekError::CycleDetected { .. }) => {}
            _ => panic!("Expected CycleDetected error"),
        }
    }

    #[test]
    fn test_resolve_terminal_binary() {
        let temp = TempDir::new().unwrap();

        let elf_magic = [0x7f, b'E', b'L', b'F', 0x02, 0x01, 0x01, 0x00];
        let binary = create_executable(&temp, "binary", &elf_magic);

        let chain = resolve(&binary).unwrap();

        assert_eq!(chain.links.len(), 1);
        assert!(matches!(
            chain.links[0].link_type,
            LinkType::Terminal(FileKind::Binary)
        ));
        assert!(chain.links[0].is_final);
    }

    #[test]
    fn test_resolve_terminal_text() {
        let temp = TempDir::new().unwrap();

        let script = b"#!/bin/bash\necho hello\n";
        let file = create_executable(&temp, "script", script);

        let chain = resolve(&file).unwrap();

        assert_eq!(chain.links.len(), 1);
        let last_link = &chain.links[chain.links.len() - 1];
        assert!(last_link.is_final);
        // Terminal node should be Text (not Binary)
        assert!(matches!(
            last_link.link_type,
            LinkType::Terminal(FileKind::Text)
        ));
    }

    #[test]
    fn test_resolve_target_absolute() {
        let current = Path::new("/usr/bin/link");
        let target = Path::new("/usr/local/bin/target");

        let resolved = resolve_target(current, target);
        assert_eq!(resolved, PathBuf::from("/usr/local/bin/target"));
    }

    #[test]
    fn test_resolve_target_relative() {
        let current = Path::new("/usr/bin/link");
        let target = Path::new("../lib/target");

        let resolved = resolve_target(current, target);
        // Should resolve to /usr/lib/target
        assert_eq!(resolved, PathBuf::from("/usr/lib/target"));
    }

    #[test]
    fn test_resolve_target_with_dots() {
        let current = Path::new("/usr/bin/link");
        let target = Path::new("./target");

        let resolved = resolve_target(current, target);
        assert_eq!(resolved, PathBuf::from("/usr/bin/target"));
    }
}
