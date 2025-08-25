use snapbox::{cmd::Command, file};
use snapbox_macros::cargo_bin;

macro_rules! cli_to_yaml_test {
    ($test_name:ident, $input:literal, $output:literal) => {
        #[test]
        fn $test_name() {
            Command::new(cargo_bin!("hbt"))
                .args(["-t", "yaml", concat!("tests/", $input)])
                .assert()
                .success()
                .stdout_eq(file!($output));
        }
    };
}

// Include auto-generated tests
include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
