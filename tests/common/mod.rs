//! Shared test utilities and fixtures for symseek tests.

use assert_fs::TempDir;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Create an executable file with the given content and permissions.
pub fn create_executable(dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
    let file = dir.child(name);
    file.write_binary(content).unwrap();
    let mut perms = fs::metadata(file.path()).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(file.path(), perms).unwrap();
    file.to_path_buf()
}

/// Create a shell script wrapper that calls another program.
pub fn create_shell_wrapper(dir: &TempDir, name: &str, target_path: &str) -> PathBuf {
    let script = format!("#!/bin/bash\nexec {} \"$@\"\n", target_path);
    create_executable(dir, name, script.as_bytes())
}

/// Create an ELF binary mock with ELF magic bytes.
pub fn create_elf_binary(dir: &TempDir, name: &str) -> PathBuf {
    create_executable(dir, name, ELF_MAGIC)
}

/// Create a binary wrapper with ELF magic bytes and embedded path.
pub fn create_binary_wrapper(dir: &TempDir, name: &str, target_path: &str) -> PathBuf {
    let mut content = ELF_MAGIC.to_vec();
    content.extend_from_slice(b"\0\0\0\0");
    content.extend_from_slice(target_path.as_bytes());
    content.push(0);
    create_executable(dir, name, &content)
}

/// Create a symlink chain by chaining multiple symlinks.
pub fn create_symlink_chain(
    dir: &TempDir,
    names: &[&str],
    final_target: &PathBuf,
) -> PathBuf {
    let mut previous = final_target.clone();

    for name in names.iter().rev() {
        let link = dir.child(name);
        link.symlink_to_file(&previous).unwrap();
        previous = link.to_path_buf();
    }

    previous
}

// Test data constants

/// ELF binary magic bytes
pub const ELF_MAGIC: &[u8] = &[0x7f, b'E', b'L', b'F', 0x02, 0x01, 0x01, 0x00];

/// Bash shebang
pub const BASH_SHEBANG: &str = "#!/bin/bash\n";

/// Python 3 shebang
pub const PYTHON_SHEBANG: &str = "#!/usr/bin/python3\n";

/// Perl shebang
pub const PERL_SHEBANG: &str = "#!/usr/bin/perl\n";
