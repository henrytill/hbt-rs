use time::macros::datetime;

use super::*;

fn create_test_collection() -> Collection {
    let json = std::fs::read_to_string(format!(
        "{}/src/collection/fixtures/test_collection.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|_| panic!("Failed to load test collection fixture"));

    serde_json::from_str(&json).unwrap()
}

#[test]
fn test_incompatible_version() {
    let collection = create_test_collection();
    let mut json = serde_json::to_string(&collection).unwrap();

    // Replace version with an incompatible one
    json = json.replace("\"0.1.0\"", "\"0.2.0\"");

    // Attempt to deserialize should fail
    let result: Result<Collection, _> = serde_json::from_str(&json);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("incompatible version"));
}

#[test]
fn test_empty_collection() {
    let collection = Collection::new();
    let json = serde_json::to_string(&collection).unwrap();
    let deserialized: Collection = serde_json::from_str(&json).unwrap();

    assert!(collection.is_empty());
    assert!(deserialized.is_empty());
}

#[test]
fn test_entity_updates() {
    let mut collection = Collection::new();

    // Create initial entity
    let url = Url::parse("https://example.com/page").unwrap();
    let entity = Entity::new(
        url.clone(),
        datetime!(2024-01-15 0:00 UTC).into(),
        Some(Name::from("Original")),
        vec![Label::from("tag1")].into_iter().collect(),
    );
    let id = collection.insert(entity);

    // Serialize and deserialize
    let json = serde_json::to_string(&collection).unwrap();
    let mut deserialized: Collection = serde_json::from_str(&json).unwrap();

    // Update the deserialized entity
    let updated_entity = deserialized.entity_mut(id);
    updated_entity.update(
        datetime!(2024-01-16 0:00 UTC).into(),
        vec![Name::from("Updated")].into_iter().collect(),
        vec![Label::from("tag2")].into_iter().collect(),
    );

    // Serialize and deserialize again
    let json = serde_json::to_string(&deserialized).unwrap();
    let final_collection: Collection = serde_json::from_str(&json).unwrap();

    // Verify updates persisted
    let final_entity = final_collection.entity(id);
    assert!(final_entity.names().contains(&Name::from("Original")));
    assert!(final_entity.names().contains(&Name::from("Updated")));
    assert!(final_entity.labels().contains(&Label::from("tag1")));
    assert!(final_entity.labels().contains(&Label::from("tag2")));
    assert_eq!(final_entity.updated_at().len(), 1);
}

#[test]
fn snapshot_collection_serialization() {
    let collection = {
        let mut labels: BTreeSet<Label> = vec![Label::from("foo")].into_iter().collect();
        let foo = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            datetime!(2023-11-15 0:00 UTC).into(),
            Some(Name::from("Foo")),
            labels.clone(),
        );
        labels.insert(Label::from("bar"));
        let bar = Entity::new(
            Url::parse("https://bar.com").unwrap(),
            datetime!(2023-11-15 0:00 UTC).into(),
            Some(Name::from("Bar")),
            labels.clone(),
        );
        labels.insert(Label::from("baz"));
        let baz = Entity::new(
            Url::parse("https://baz.com").unwrap(),
            datetime!(2023-11-15 0:00 UTC).into(),
            Some(Name::from("Baz")),
            labels,
        );
        let mut tmp = Collection::new();
        let id_foo = tmp.upsert(foo);
        let id_bar = tmp.upsert(bar);
        let id_baz = tmp.upsert(baz);
        tmp.add_edges(id_foo, id_bar);
        tmp.add_edges(id_foo, id_baz);
        tmp
    };
    insta::assert_yaml_snapshot!(collection)
}

#[test]
fn test_update_labels() {
    let mut collection = create_test_collection();

    // Test basic label update
    let update = vec![
        ("tag1".to_string(), "tag1-updated".to_string()),
        ("tag2".to_string(), "tag2-updated".to_string()),
    ];
    collection.update_labels(update).unwrap();

    // Verify updates on first entity
    let id1 = Id::new(0);
    let labels1 = collection.entity(id1).labels();
    assert!(labels1.contains(&Label::from("tag1-updated")));
    assert!(labels1.contains(&Label::from("tag2-updated")));
    assert!(!labels1.contains(&Label::from("tag1")));
    assert!(!labels1.contains(&Label::from("tag2")));

    // Verify updates on second entity
    let id2 = Id::new(1);
    let labels2 = collection.entity(id2).labels();
    assert!(labels2.contains(&Label::from("tag2-updated")));
    assert!(labels2.contains(&Label::from("tag3")));
    assert!(!labels2.contains(&Label::from("tag2")));
}

#[test]
fn test_update_labels_empty_mapping() {
    let mut collection = create_test_collection();

    // Empty update should not modify labels
    let update: Vec<(String, String)> = vec![];
    collection.update_labels(update).unwrap();

    let id1 = Id::new(0);
    assert_eq!(
        collection.entity(id1).labels(),
        &vec![Label::from("tag1"), Label::from("tag2")].into_iter().collect()
    );
}

#[test]
fn test_update_labels_non_string_values() {
    let mut collection = create_test_collection();

    // Test that only valid string pairs are processed
    let update = vec![("tag2".to_string(), "valid-update".to_string())];
    collection.update_labels(update).unwrap();

    let id1 = Id::new(0);
    let labels = collection.entity(id1).labels();
    assert!(labels.contains(&Label::from("tag1"))); // unchanged
    assert!(labels.contains(&Label::from("valid-update")));
    assert!(!labels.contains(&Label::from("tag2")));
}

mod html_tests {
    use super::*;

    fn load_fixture(filename: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/src/collection/fixtures/{}",
            env!("CARGO_MANIFEST_DIR"),
            filename
        ))
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", filename))
    }

    macro_rules! snapshot_html_test {
        ($test_name:ident, $fixture_name:expr) => {
            #[test]
            fn $test_name() {
                let html = load_fixture($fixture_name);
                let collection = Collection::from_html_str(&html).unwrap();
                insta::assert_yaml_snapshot!(collection);
            }
        };
    }

    snapshot_html_test!(snapshot_html_simple, "bookmarks_simple.html");
    snapshot_html_test!(snapshot_html_folders, "bookmarks_folders.html");
    snapshot_html_test!(snapshot_html_privacy, "bookmarks_privacy.html");
    snapshot_html_test!(snapshot_html_feeds, "bookmarks_feeds.html");
    snapshot_html_test!(snapshot_html_pinboard, "bookmarks_pinboard.html");

    macro_rules! snapshot_to_html_test {
        ($test_name:ident, $fixture_name:expr) => {
            #[test]
            fn $test_name() {
                let html = load_fixture($fixture_name);
                let collection = Collection::from_html_str(&html).unwrap();
                let generated_html = collection.to_html().unwrap();
                insta::assert_snapshot!(generated_html);
            }
        };
    }

    snapshot_to_html_test!(snapshot_to_html_simple, "bookmarks_simple.html");
    snapshot_to_html_test!(snapshot_to_html_folders, "bookmarks_folders.html");
    snapshot_to_html_test!(snapshot_to_html_privacy, "bookmarks_privacy.html");
    snapshot_to_html_test!(snapshot_to_html_feeds, "bookmarks_feeds.html");
    snapshot_to_html_test!(snapshot_to_html_pinboard, "bookmarks_pinboard.html");

    #[test]
    fn test_html_roundtrip_consistency() {
        // Test that parsing and regenerating preserves data structure
        let html = load_fixture("bookmarks_simple.html");
        let collection = Collection::from_html_str(&html).unwrap();
        let generated_html = collection.to_html().unwrap();
        let roundtrip_collection = Collection::from_html_str(&generated_html).unwrap();

        // Verify the collections are equivalent
        assert_eq!(collection.len(), roundtrip_collection.len());

        for i in 0..collection.len() {
            let id = Id::new(i);
            let original = collection.entity(id);
            let roundtrip = roundtrip_collection.entity(id);

            assert_eq!(original.url(), roundtrip.url());
            assert_eq!(original.names(), roundtrip.names());
            assert_eq!(original.labels(), roundtrip.labels());
            assert_eq!(original.shared(), roundtrip.shared());
            assert_eq!(original.toread(), roundtrip.toread());
            assert_eq!(original.is_feed(), roundtrip.is_feed());
        }
    }
}

mod to_html_tests {
    use super::*;

    #[test]
    fn test_to_html_basic() {
        // Create a simple collection for testing
        let mut collection = Collection::new();

        let url = Url::parse("https://example.com/").unwrap();
        let entity = Entity::new(
            url,
            datetime!(2021-01-01 0:00 UTC).into(),
            Some(Name::from("Example Website")),
            vec![Label::from("test"), Label::from("example")].into_iter().collect(),
        );

        collection.upsert(entity);

        let html = collection.to_html().unwrap();

        // Basic structure checks
        assert!(html.contains("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
        assert!(html.contains("<TITLE>Bookmarks</TITLE>"));
        assert!(html.contains("<H1>Bookmarks</H1>"));
        assert!(html.contains("<DL>"));
        assert!(html.contains("</DL>"));

        // Should contain the bookmark
        assert!(html.contains(r#"HREF="https://example.com/""#));
        assert!(html.contains("ADD_DATE="));
        assert!(html.contains("Example Website"));
        // Tags might be in different order due to BTreeSet
        assert!(html.contains("TAGS=") && html.contains("test") && html.contains("example"));
    }

    #[test]
    fn test_to_html_empty_collection() {
        let collection = Collection::new();
        let html = collection.to_html().unwrap();

        // Should generate valid HTML structure even for empty collection
        assert!(html.contains("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
        assert!(html.contains("<TITLE>Bookmarks</TITLE>"));
        assert!(html.contains("<DL><p>"));
        assert!(html.contains("</DL><p>"));

        // Should not contain any bookmarks
        assert!(!html.contains("<DT><A HREF="));
    }

    #[test]
    fn test_to_html_fallback_to_url() {
        // Test that when no name is provided, URL is used as title
        let mut collection = Collection::new();

        let url = Url::parse("https://github.com/").unwrap();
        let entity = Entity::new(
            url,
            datetime!(2021-01-01 0:00 UTC).into(),
            None, // No name provided
            BTreeSet::new(),
        );

        collection.upsert(entity);

        let html = collection.to_html().unwrap();

        // Should use URL as the title when no name is available
        assert!(html.contains(">https://github.com/</A>"));
    }
}
