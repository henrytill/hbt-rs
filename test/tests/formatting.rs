mod html {
    hbt_test_macros::test_formatter!("test-data/html", "html");
}

// The YAML output had no fixture coverage at all, so divergences from the shared wire format -
// which every implementation reads and writes - went unnoticed. Every category's expected YAML is
// also a formatter fixture, so compare against all of them.
mod yaml {
    mod html {
        hbt_test_macros::test_formatter!("test-data/html", "yaml");
    }

    mod markdown {
        hbt_test_macros::test_formatter!("test-data/markdown", "yaml");
    }

    mod pinboard {
        mod json {
            hbt_test_macros::test_formatter!("test-data/pinboard/json", "yaml");
        }

        mod xml {
            hbt_test_macros::test_formatter!("test-data/pinboard/xml", "yaml");
        }
    }
}
