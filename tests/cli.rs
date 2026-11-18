use std::{
    path::Path,
    process::{Command, Output},
};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oculi"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap()
}

#[test]
fn summarizes_the_example_in_declaration_order() {
    let output = cli(&["examples/counter.vcd"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Timescale: 1 ns\nSignal\tBits\tTransitions\ntb.clk\t1\t5\ntb.count[3:0]\t4\t3\ntb.dut.clk\t1\t5\n"
    );
}

#[test]
fn help_succeeds_and_bad_arguments_fail_without_a_summary() {
    let output = cli(&["--help"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: oculi"));
    for args in [&[][..], &["a.vcd", "b.vcd"][..]] {
        let output = cli(args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Usage:"));
    }
}

#[test]
fn file_errors_and_malformed_input_fail_without_a_summary() {
    for path in ["examples/no-such-trace.vcd", "Cargo.toml"] {
        let output = cli(&[path]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(path), "{error}");
    }
}

// Each test owns a unique temporary directory; Drop cleans it even on failure.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "oculi-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
#[test]
fn opens_non_utf8_paths() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
    let directory = TempDir::new();
    let path = directory.0.join(OsStr::from_bytes(b"trace-\xff.vcd"));
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/counter.vcd"),
        &path,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_oculi"))
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("tb.count[3:0]\t4\t3"));
}
