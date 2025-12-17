use crate::core::types::FileLocation;
use crate::error::{Result, SymseekError};
use std::{env, path};

pub fn find_file(name: &str) -> Result<FileLocation> {
    if let Some(path) = search_in_cwd(name)? {
        return Ok(FileLocation::CurrentDirectory(path));
    }

    if name.contains(path::MAIN_SEPARATOR) {
        return Err(SymseekError::NotFound {
            name: name.to_string(),
            searched_locations: vec!["current directory".to_string()],
        });
    }

    let paths = search_in_path(name)?;
    if !paths.is_empty() {
        return Ok(FileLocation::PathEnvironment(paths));
    }

    Err(SymseekError::NotFound {
        name: name.to_string(),
        searched_locations: vec!["current directory".to_string(), "PATH".to_string()],
    })
}

fn search_in_cwd(name: &str) -> Result<Option<path::PathBuf>> {
    let cwd = env::current_dir().map_err(|e| SymseekError::Io {
        context: "Failed to get current directory".to_string(),
        source: e,
    })?;

    let target = cwd.join(name);
    match target.try_exists() {
        Ok(true) => Ok(Some(target)),
        Ok(false) => Ok(None),
        Err(e) => Err(SymseekError::Io {
            context: format!("Failed to check if {} exists", target.display()),
            source: e,
        }),
    }
}

fn search_in_path(name: &str) -> Result<Vec<path::PathBuf>> {
    let paths = env::var("PATH").map_err(|_| SymseekError::InvalidInput {
        message: "PATH environment variable not found".to_string(),
    })?;

    let mut found_paths = Vec::new();

    for path in env::split_paths(&paths) {
        let full_path = path.join(name);
        match full_path.try_exists() {
            Ok(true) => found_paths.push(full_path),
            Ok(false) => {}
            Err(e) => {
                return Err(SymseekError::Io {
                    context: format!("Failed to check if {} exists", full_path.display()),
                    source: e,
                })
            }
        }
    }

    Ok(found_paths)
}
