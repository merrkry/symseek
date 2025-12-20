use crate::core::types::FileLocation;
use crate::error::{Result, SymseekError};
use log::{debug, trace};
use std::{env, path};

/// Find a file by name in the current directory or PATH.
///
/// If the name contains a path separator, it's treated as a path and searched
/// in the current directory. Otherwise, it's treated as a binary name and
/// searched in the PATH environment variable.
///
/// # Errors
///
/// Returns an error if:
/// - The file is not found in the searched locations
/// - The current directory cannot be determined
/// - The PATH environment variable is not set
/// - File existence cannot be checked
pub fn find_file(name: &str) -> Result<FileLocation> {
    debug!("find_file called with: {name}");

    // If input contains path separators, handle as a path
    if name.contains(path::MAIN_SEPARATOR) {
        debug!("Input contains path separator, treating as path");
        if let Some(path) = search_in_cwd(name)? {
            debug!("Found path in current directory: {}", path.display());
            return Ok(FileLocation::CurrentDirectory(path));
        }

        debug!("Path not found in current directory");
        return Err(SymseekError::NotFound {
            name: name.to_string(),
            searched_locations: vec!["current directory".to_string()],
        });
    }

    // If input is just a binary name, search only in PATH
    debug!("Input is a binary name, searching in PATH");
    let paths = search_in_path(name)?;
    if !paths.is_empty() {
        debug!("Found {} matches in PATH", paths.len());
        return Ok(FileLocation::PathEnvironment(paths));
    }

    debug!("No matches found in PATH");
    Err(SymseekError::NotFound {
        name: name.to_string(),
        searched_locations: vec!["PATH".to_string()],
    })
}

fn search_in_cwd(name: &str) -> Result<Option<path::PathBuf>> {
    let cwd = env::current_dir().map_err(|e| SymseekError::Io {
        context: "Failed to get current directory".to_string(),
        source: e,
    })?;

    let target = cwd.join(name);
    trace!("Checking if exists in cwd: {}", target.display());

    match target.try_exists() {
        Ok(true) => {
            trace!("File exists: {}", target.display());
            Ok(Some(target))
        }
        Ok(false) => {
            trace!("File does not exist: {}", target.display());
            Ok(None)
        }
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

    debug!("Searching PATH for: {name}");
    let mut found_paths = Vec::new();

    for path in env::split_paths(&paths) {
        let full_path = path.join(name);
        trace!("Checking PATH entry: {}", full_path.display());

        match full_path.try_exists() {
            Ok(true) => {
                trace!("Found in PATH: {}", full_path.display());
                found_paths.push(full_path);
            }
            Ok(false) => {}
            Err(e) => {
                return Err(SymseekError::Io {
                    context: format!("Failed to check if {} exists", full_path.display()),
                    source: e,
                });
            }
        }
    }

    Ok(found_paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::FileLocation;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use std::os::unix::fs::PermissionsExt;

    fn create_executable(path: &std::path::PathBuf) {
        std::fs::File::create(path).unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn test_find_file_in_path() {
        let temp = TempDir::new().unwrap();

        // Create mock PATH directories
        let bin1 = temp.child("bin1");
        bin1.create_dir_all().unwrap();
        let bin2 = temp.child("bin2");
        bin2.create_dir_all().unwrap();

        // Add executable to bin2
        let myexe = bin2.child("myexe");
        create_executable(&myexe.to_path_buf());

        // Set PATH
        let path_value = format!("{}:{}", bin1.path().display(), bin2.path().display());
        let original_path = env::var("PATH").ok();
        unsafe {
            env::set_var("PATH", &path_value);
        }

        let result = find_file("myexe");

        // Restore PATH
        if let Some(original) = original_path {
            unsafe {
                env::set_var("PATH", original);
            }
        }

        assert!(result.is_ok());
        match result.unwrap() {
            FileLocation::PathEnvironment(paths) => {
                assert_eq!(paths.len(), 1);
                assert!(paths[0].ends_with("bin2/myexe"));
            }
            FileLocation::CurrentDirectory(_) => panic!("Expected PathEnvironment"),
        }
    }

    #[test]
    fn test_find_file_multiple_in_path() {
        let temp = TempDir::new().unwrap();

        let bin1 = temp.child("bin1");
        bin1.create_dir_all().unwrap();
        let bin2 = temp.child("bin2");
        bin2.create_dir_all().unwrap();

        // Add same executable to both
        let cmd1 = bin1.child("cmd");
        let cmd2 = bin2.child("cmd");
        create_executable(&cmd1.to_path_buf());
        create_executable(&cmd2.to_path_buf());

        let path_value = format!("{}:{}", bin1.path().display(), bin2.path().display());
        let original_path = env::var("PATH").ok();
        unsafe {
            env::set_var("PATH", &path_value);
        }

        let result = find_file("cmd");

        if let Some(original) = original_path {
            unsafe {
                env::set_var("PATH", original);
            }
        }

        assert!(result.is_ok());
        match result.unwrap() {
            FileLocation::PathEnvironment(paths) => {
                assert_eq!(paths.len(), 2);
            }
            FileLocation::CurrentDirectory(_) => panic!("Expected PathEnvironment"),
        }
    }

    #[test]
    fn test_find_file_not_in_path() {
        let temp = TempDir::new().unwrap();
        let bin = temp.child("bin");
        bin.create_dir_all().unwrap();

        let original_path = env::var("PATH").ok();
        unsafe {
            env::set_var("PATH", bin.path().to_str().unwrap());
        }

        let result = find_file("nonexistent");

        if let Some(original) = original_path {
            unsafe {
                env::set_var("PATH", original);
            }
        }

        assert!(result.is_err());
    }

    #[test]
    fn test_find_file_binary_name_only() {
        let temp = TempDir::new().unwrap();
        let bin = temp.child("bin");
        bin.create_dir_all().unwrap();

        let testcmd = bin.child("testcmd");
        create_executable(&testcmd.to_path_buf());

        let original_path = env::var("PATH").ok();
        unsafe {
            env::set_var("PATH", bin.path().to_str().unwrap());
        }

        let result = find_file("testcmd");

        if let Some(original) = original_path {
            unsafe {
                env::set_var("PATH", original);
            }
        }

        assert!(result.is_ok());
        match result.unwrap() {
            FileLocation::PathEnvironment(paths) => {
                assert!(!paths.is_empty());
                assert!(paths[0].ends_with("testcmd"));
            }
            FileLocation::CurrentDirectory(_) => panic!("Expected PathEnvironment for binary name"),
        }
    }
}
