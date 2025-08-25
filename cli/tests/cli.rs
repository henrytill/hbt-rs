use snapbox::{cmd::Command, file};
use snapbox_macros::cargo_bin;

macro_rules! cli_to_yaml_test {
    ($test_name:ident, $input:literal, $output:literal) => {
        #[test]
        fn $test_name() {
            Command::new(cargo_bin!("hbt"))
                .args(["-t", "yaml", $input])
                .assert()
                .success()
                .stdout_eq(file!($output));
        }
    };
}

macro_rules! cli_to_html_test {
    ($test_name:ident, $input:literal, $output:literal) => {
        #[test]
        fn $test_name() {
            Command::new(cargo_bin!("hbt"))
                .args(["-t", "html", $input])
                .assert()
                .success()
                .stdout_eq(file!($output));
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
