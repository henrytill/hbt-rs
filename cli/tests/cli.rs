use snapbox::{cmd::Command, file};
use snapbox_macros::cargo_bin;

#[test]
fn test_help() {
    Command::new(cargo_bin!("hbt"))
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/help.stdout"));
}

#[test]
fn test_version() {
    Command::new(cargo_bin!("hbt"))
        .arg("--version")
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/version.stdout"));
}

#[test]
fn test_missing_file() {
    Command::new(cargo_bin!("hbt"))
        .arg("nonexistent.md")
        .assert()
        .failure()
        .stderr_eq(file!("cli/snapshots/missing.stderr"));
}

#[test]
fn test_basic_markdown() {
    Command::new(cargo_bin!("hbt"))
        .arg("tests/cli/fixtures/basic.md")
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.md.stdout"));
}

#[test]
fn test_dump_markdown() {
    Command::new(cargo_bin!("hbt"))
        .args(["--dump", "tests/cli/fixtures/basic.md"])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.dump.stdout"));
}

#[cfg(feature = "pinboard")]
#[test]
fn test_basic_html() {
    Command::new(cargo_bin!("hbt"))
        .arg("tests/cli/fixtures/basic.html")
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.html.stdout"));
}

#[cfg(feature = "pinboard")]
#[test]
fn test_dump_html() {
    Command::new(cargo_bin!("hbt"))
        .args(["--dump", "tests/cli/fixtures/basic.html"])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.dump.stdout"));
}

#[test]
fn test_mappings() {
    Command::new(cargo_bin!("hbt"))
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

#[test]
fn test_empty_mappings() {
    Command::new(cargo_bin!("hbt"))
        .args([
            "--dump",
            "--mappings",
            "tests/cli/fixtures/mappings-empty.json",
            "tests/cli/fixtures/basic.md",
        ])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.dump.stdout")); // Should match original dump
}

#[test]
fn test_invalid_mappings() {
    Command::new(cargo_bin!("hbt"))
        .args([
            "--dump",
            "--mappings",
            "tests/cli/fixtures/mappings-invalid.json",
            "tests/cli/fixtures/basic.md",
        ])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.dump.stdout")); // Should match original dump since invalid mappings are ignored
}

#[test]
fn test_tags() {
    Command::new(cargo_bin!("hbt"))
        .args(["--tags", "tests/cli/fixtures/basic.md"])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.tags.stdout"));
}

#[test]
fn test_tags_with_mappings() {
    Command::new(cargo_bin!("hbt"))
        .args([
            "--tags",
            "--mappings",
            "tests/cli/fixtures/mappings.json",
            "tests/cli/fixtures/basic.md",
        ])
        .assert()
        .success()
        .stdout_eq(file!("cli/snapshots/basic.mapped.tags.stdout"));
}
