use std::path::Path;

/// Declares `test-data` as an input to compiling this crate.
///
/// The `test_parser!` and `test_formatter!` macros discover fixtures by walking that directory
/// while they expand, which happens at compile time. Cargo cannot see through a proc macro to
/// learn what it read, so without this, adding or removing a fixture changes nothing it tracks:
/// the previously expanded tests are reused and a newly added fixture silently generates no test,
/// leaving the suite green without covering it. (Editing an existing fixture is caught either way,
/// since the expected file is read when the test runs.)
fn main() {
    let test_data = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("test crate must have a parent directory")
        .join("test-data");

    // Cargo scans a directory recursively, so this covers every fixture under it.
    println!("cargo:rerun-if-changed={}", test_data.display());
}
