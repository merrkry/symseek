use assert_fs::TempDir;
use assert_fs::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn verbose_json_keeps_logs_out_of_stdout() {
    let temp = TempDir::new().unwrap();
    let target = temp.child("target");
    target
        .write_binary(&[0x7f, b'E', b'L', b'F', 0x02, 0x01, 0x01, 0x00])
        .unwrap();

    let mut permissions = target.metadata().unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(target.path(), permissions).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_symseek"))
        .args(["--verbose", "--json"])
        .arg(target.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("DEBUG"));
}
