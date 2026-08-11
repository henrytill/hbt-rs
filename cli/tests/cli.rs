use std::fs;
use std::path::{Path, PathBuf};

use snapbox::cmd::Command;
use snapbox::{cargo_bin, file};

const TEST_FILE: &str = "test-data/markdown/basic.input.md";

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn hbt() -> Command {
    Command::new(cargo_bin!("hbt")).current_dir(workspace_root())
}

/// A path under the target tmpdir, unique per test so tests can run in parallel.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn schema_output() {
    hbt()
        .args(["--schema"])
        .assert()
        .success()
        .stdout_eq(file!["../../test-data/collection.schema.json"]);
}

#[test]
fn info_flag_counts_entities() {
    hbt()
        .args(["--info", TEST_FILE])
        .assert()
        .success()
        .stdout_eq("test-data/markdown/basic.input.md: 3 entities\n");
}

/// Tags come out of a BTreeSet, so the order is sorted rather than whatever the input used.
#[test]
fn list_tags_flag_is_sorted() {
    hbt()
        .args(["--list-tags", TEST_FILE])
        .assert()
        .success()
        .stdout_eq("Bar\nFoo\nMisc\n");
}

#[test]
fn yaml_output_matches_the_fixture() {
    hbt()
        .args(["-t", "yaml", TEST_FILE])
        .assert()
        .success()
        .stdout_eq(file!["../../test-data/markdown/basic.expected.yaml"]);
}

#[test]
fn html_output_is_a_bookmark_file() {
    let out = hbt().args(["-t", "html", TEST_FILE]).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.starts_with("<!DOCTYPE NETSCAPE-Bookmark-file-1>"),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"<A HREF="https://example.com/""#),
        "{stdout}"
    );
}

/// Writing to a file must produce exactly what stdout would have, and print nothing.
#[test]
fn output_file_matches_stdout() {
    let expected = hbt().args(["-t", "yaml", TEST_FILE]).assert().success();
    let expected = expected.get_output().stdout.clone();

    let out = scratch("output_file").join("out.yaml");
    hbt()
        .args(["-t", "yaml", "-o"])
        .arg(&out)
        .arg(TEST_FILE)
        .assert()
        .success()
        .stdout_eq("");

    assert_eq!(fs::read(&out).unwrap(), expected);
}

/// With no -t, the output format comes from the -o filename.
#[test]
fn output_format_detected_from_output_filename() {
    let out = scratch("detect_output").join("out.html");
    hbt()
        .args(["-o"])
        .arg(&out)
        .arg(TEST_FILE)
        .assert()
        .success();

    let written = fs::read_to_string(&out).unwrap();
    assert!(
        written.starts_with("<!DOCTYPE NETSCAPE-Bookmark-file-1>"),
        "{written}"
    );
}

/// -f overrides extension-based detection.
#[test]
fn input_format_can_be_forced() {
    let dir = scratch("forced_input");
    let input = dir.join("bookmarks.txt");
    fs::copy(workspace_root().join(TEST_FILE), &input).unwrap();

    hbt()
        .args(["-f", "md", "--info"])
        .arg(&input)
        .assert()
        .success()
        .stdout_eq(format!("{}: 3 entities\n", input.display()));
}

#[test]
fn mappings_rewrite_labels() {
    let dir = scratch("mappings");
    let mappings = dir.join("mappings.yaml");
    fs::write(&mappings, "Foo: Renamed\n").unwrap();

    hbt()
        .args(["--list-tags", "--mappings"])
        .arg(&mappings)
        .arg(TEST_FILE)
        .assert()
        .success()
        .stdout_eq("Bar\nMisc\nRenamed\n");
}

#[test]
fn missing_input_file_is_an_error() {
    hbt()
        .args(["-t", "yaml", "test-data/does-not-exist.md"])
        .assert()
        .failure();
}

#[test]
fn missing_input_argument_is_an_error() {
    hbt()
        .assert()
        .failure()
        .stderr_eq("Error: Input file required\n");
}

#[test]
fn missing_output_format_is_an_error() {
    hbt().args([TEST_FILE]).assert().failure().stderr_eq(
        "Error: Must specify an output format (-t) or analysis flag (--info, --list-tags)\n",
    );
}

#[test]
fn undetectable_input_format_is_an_error() {
    hbt()
        .args(["-t", "yaml", "Cargo.lock"])
        .assert()
        .failure()
        .stderr_eq("Error: No parser for file: Cargo.lock\n");
}

#[test]
fn missing_mappings_file_is_an_error() {
    hbt()
        .args([
            "--info",
            "--mappings",
            "test-data/no-such-mappings.yaml",
            TEST_FILE,
        ])
        .assert()
        .failure();
}

#[test]
fn mappings_file_of_the_wrong_shape_is_an_error() {
    let dir = scratch("bad_mappings");
    let mappings = dir.join("mappings.yaml");
    fs::write(&mappings, "- just\n- a\n- list\n").unwrap();

    hbt()
        .args(["--info", "--mappings"])
        .arg(&mappings)
        .arg(TEST_FILE)
        .assert()
        .failure()
        .stderr_eq("Error: Mapping file must contain a YAML mapping\n");
}

/// A link before any level-1 date heading has no creation time to take.
#[test]
fn markdown_link_without_a_date_is_an_error() {
    let dir = scratch("no_date");
    let input = dir.join("no-date.md");
    fs::write(&input, "- [Foo](https://foo.test/)\n").unwrap();

    hbt()
        .args(["-t", "yaml"])
        .arg(&input)
        .assert()
        .failure()
        .stderr_eq("Error: missing date\n");
}
