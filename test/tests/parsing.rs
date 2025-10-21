mod html {
    hbt_test_macros::test_parser!("test-data/html", "html");
}

mod markdown {
    hbt_test_macros::test_parser!("test-data/markdown", "md");
}

mod pinboard {
    mod json {
        hbt_test_macros::test_parser!("test-data/pinboard/json", "json");
    }

    mod xml {
        hbt_test_macros::test_parser!("test-data/pinboard/xml", "xml");
    }
}
