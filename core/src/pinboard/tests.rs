use super::*;

fn load_fixture(filename: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/src/pinboard/fixtures/{}",
        env!("CARGO_MANIFEST_DIR"),
        filename
    ))
    .unwrap_or_else(|_| panic!("Failed to load fixture: {}", filename))
}

macro_rules! snapshot_xml_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let fixture_name = stringify!($name).strip_prefix("test_").unwrap_or(stringify!($name));
            let xml = load_fixture(&format!("{}.xml", fixture_name));
            let posts = Post::from_xml(&xml).unwrap();
            insta::assert_yaml_snapshot!(fixture_name, posts);
        }
    };
}

macro_rules! snapshot_json_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let fixture_name = stringify!($name).strip_prefix("test_").unwrap_or(stringify!($name));
            let json = load_fixture(&format!("{}.json", fixture_name));
            let posts = Post::from_json(&json).unwrap();
            insta::assert_yaml_snapshot!(fixture_name, posts);
        }
    };
}

snapshot_xml_test!(test_empty);
snapshot_xml_test!(test_xml_sample);
snapshot_json_test!(test_json_sample);
