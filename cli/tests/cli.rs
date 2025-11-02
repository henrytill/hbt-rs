use std::path::Path;

use snapbox::cmd::Command;
use snapbox::{cargo_bin, file};

const TEST_FILE: &str = "test-data/markdown/basic.input.md";

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

#[test]
fn schema_output() {
    Command::new(cargo_bin!("hbt"))
        .current_dir(workspace_root())
        .args(["--schema"])
        .assert()
        .success()
        .stdout_eq(file!["../../test-data/collection.schema.json"]);
}

#[test]
fn info_flag() {
    Command::new(cargo_bin!("hbt"))
        .current_dir(workspace_root())
        .args(["--info", TEST_FILE])
        .assert()
        .success();
}

#[test]
fn list_tags_flag() {
    Command::new(cargo_bin!("hbt"))
        .current_dir(workspace_root())
        .args(["--list-tags", TEST_FILE])
        .assert()
        .success();
}

#[test]
fn yaml_output() {
    Command::new(cargo_bin!("hbt"))
        .current_dir(workspace_root())
        .args(["-t", "yaml", TEST_FILE])
        .assert()
        .success();
}

#[test]
fn html_output() {
    Command::new(cargo_bin!("hbt"))
        .current_dir(workspace_root())
        .args(["-t", "html", TEST_FILE])
        .assert()
        .success();
}
