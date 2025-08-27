pub mod utils {
    use std::{env, io, path};

    pub enum FileLocation {
        Cwd(path::PathBuf),
        Path(Vec<path::PathBuf>),
    }

    pub fn search_file(name: &str) -> Result<FileLocation, io::Error> {
        let cwd = env::current_dir().expect("Failed to get current directory");
        let cwd_target = cwd.join(name);

        match cwd_target.try_exists() {
            Ok(true) => return Ok(FileLocation::Cwd(cwd_target)),
            Ok(false) => {}
            Err(e) => return Err(e),
        }

        // We only continue to search in PATH if the name only contains binary name.
        if name.contains(path::MAIN_SEPARATOR) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("File name '{}' not found in current directory", name),
            ));
        }

        if let Ok(paths) = env::var("PATH") {
            let mut found_paths = Vec::new();

            for path in env::split_paths(&paths) {
                let full_path = path.join(name);
                match full_path.try_exists() {
                    Ok(true) => found_paths.push(full_path),
                    Ok(false) => {}
                    Err(e) => return Err(e),
                }
            }

            if !found_paths.is_empty() {
                return Ok(FileLocation::Path(found_paths));
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File '{}' not found in current directory and PATH", name),
        ))
    }

    pub fn resolve_symlink(path: &path::Path) -> Result<Vec<path::PathBuf>, io::Error> {
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Path must be absolute",
            ));
        }

        let mut resolved_paths = Vec::new();
        let mut current_path = path.to_path_buf();

        loop {
            match current_path.read_link() {
                Ok(target) => {
                    if target.is_absolute() {
                        resolved_paths.push(target.clone());
                        current_path = target;
                    } else {
                        let joined_path = current_path
                            .parent()
                            .unwrap_or_else(|| path::Path::new("/"))
                            .join(&target);
                        let cleaned_path = path_clean::clean(joined_path);

                        resolved_paths.push(cleaned_path.clone());
                        current_path = cleaned_path;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::InvalidInput => {
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(resolved_paths)
    }
}
