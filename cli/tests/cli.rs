use snapbox::{cmd::Command, file};
use snapbox_macros::cargo_bin;

macro_rules! cli_test {
    ($test_name:ident, $format:literal, $input:literal, $output:literal) => {
        #[test]
        fn $test_name() {
            Command::new(cargo_bin!("hbt"))
                .args(["-t", $format, $input])
                .assert()
                .success()
                .stdout_eq(file!($output));
        }
    };
}

#[test]
fn schema_output() {
    Command::new(cargo_bin!("hbt"))
        .args(["--schema"])
        .assert()
        .success()
        .stdout_eq(file!["data/collection.schema.json"]);
}

include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
