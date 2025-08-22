use super::*;

fn load_fixture(filename: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/src/markdown/fixtures/{}",
        env!("CARGO_MANIFEST_DIR"),
        filename
    ))
    .unwrap_or_else(|_| panic!("Failed to load fixture: {}", filename))
}

macro_rules! snapshot_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let fixture_name = stringify!($name).strip_prefix("test_").unwrap_or(stringify!($name));
            let markdown = load_fixture(&format!("{}.md", fixture_name));
            let collection = parse(&markdown).unwrap();
            insta::assert_yaml_snapshot!(fixture_name, collection);
        }
    };
}

macro_rules! error_test {
    ($name:ident, $expected_error:expr) => {
        #[test]
        fn $name() {
            let fixture_name = stringify!($name).strip_prefix("test_").unwrap_or(stringify!($name));
            let markdown = load_fixture(&format!("{}.md", fixture_name));
            let result = parse(&markdown);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().to_string(), $expected_error);
        }
    };
}

snapshot_test!(test_empty);
error_test!(test_no_date, "missing date");
snapshot_test!(test_only_date);
snapshot_test!(test_no_labels);
snapshot_test!(test_no_url);
snapshot_test!(test_no_title);
snapshot_test!(test_indented);
snapshot_test!(test_indented_double);
snapshot_test!(test_parent);
snapshot_test!(test_parents);
snapshot_test!(test_parents_indented);
snapshot_test!(test_single_parent);
snapshot_test!(test_no_parent);
snapshot_test!(test_inverted_parents);
snapshot_test!(test_label);
snapshot_test!(test_labels);
snapshot_test!(test_multiple_labels);
snapshot_test!(test_update);
snapshot_test!(test_descending_dates);
snapshot_test!(test_mixed_dates);
snapshot_test!(test_basic);
snapshot_test!(test_nested);
