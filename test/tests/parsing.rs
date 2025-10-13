use hbt_test_macros::test_parser;

mod html {
    super::test_parser!("test-data/html", "html");
}

mod markdown {
    super::test_parser!("test-data/markdown", "md");
}

mod pinboard_json {
    super::test_parser!("test-data/pinboard/json", "json");
}

mod pinboard_xml {
    super::test_parser!("test-data/pinboard/xml", "xml");
}
