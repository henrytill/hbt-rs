use snapbox::{
    cmd::{cargo_bin, Command},
    file,
};

const BIN: &str = env!("CARGO_PKG_NAME");

#[test]
fn test_help() {
    Command::new(cargo_bin(BIN))
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/help.stdout"));
}

#[test]
fn test_version() {
    Command::new(cargo_bin(BIN))
        .arg("--version")
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/version.stdout"));
}

#[test]
fn test_missing_file() {
    Command::new(cargo_bin(BIN))
        .arg("nonexistent.md")
        .assert()
        .failure()
        .stderr_eq(file!("cli/snapshots/missing.stderr"));
}

#[test]
fn test_basic_markdown() {
    Command::new(cargo_bin(BIN))
        .arg("tests/cli/fixtures/basic.md")
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.stdout"));
}

#[test]
fn test_dump_markdown() {
    Command::new(cargo_bin(BIN))
        .args(["--dump", "tests/cli/fixtures/basic.md"])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.dump.stdout"));
}

#[cfg(feature = "pinboard")]
#[test]
fn test_basic_html() {
    Command::new(cargo_bin(BIN))
        .arg("tests/cli/fixtures/basic.html")
        .assert()
        .success()
        .stdout_eq("tests/cli/fixtures/basic.html: 3 entities\n");
}

#[cfg(feature = "pinboard")]
#[test]
fn test_dump_html() {
    Command::new(cargo_bin(BIN))
        .args(["--dump", "tests/cli/fixtures/basic.html"])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.dump.stdout"));
}

#[test]
fn test_mappings() {
    Command::new(cargo_bin(BIN))
        .args([
            "--dump",
            "--mappings",
            "tests/cli/fixtures/mappings.json",
            "tests/cli/fixtures/basic.md",
        ])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.mapped.stdout"));
}
