use crate::core::types::SymlinkChain;
use crate::error::{Result, SymseekError};
use std::path::{Path, PathBuf};

pub fn resolve(path: &Path) -> Result<SymlinkChain> {
    if !path.is_absolute() {
        return Err(SymseekError::InvalidInput {
            message: "Path must be absolute".to_string(),
        });
    }

    let mut chain = SymlinkChain::new(path.to_path_buf());
    let mut current = path.to_path_buf();

    loop {
        match current.read_link() {
            Ok(target) => {
                let resolved = resolve_target(&current, &target)?;
                current = resolved.clone();
                chain.add_link(resolved, false);
            }
            Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
                if let Some(last) = chain.links.last_mut() {
                    last.is_final = true;
                }
                break;
            }
            Err(e) => {
                return Err(SymseekError::SymlinkResolution {
                    path: current.clone(),
                    reason: e.to_string(),
                })
            }
        }
    }

    Ok(chain)
}

fn resolve_target(current: &Path, target: &Path) -> Result<PathBuf> {
    if target.is_absolute() {
        Ok(target.to_path_buf())
    } else {
        let parent = current.parent().unwrap_or_else(|| Path::new("/"));
        Ok(path_clean::clean(parent.join(target)))
    }
}
