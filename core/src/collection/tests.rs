use serde_json::json;
use time::macros::datetime;

use super::*;

fn create_test_collection() -> Collection {
    let mut collection = Collection::new();

    // Create first entity
    let url1 = Url::parse("https://example.com/page1").unwrap();
    let entity1 = Entity::new(
        url1,
        datetime!(2024-01-15 0:00 UTC).into(),
        Some(Name::from("Page One")),
        vec![Label::from("tag1"), Label::from("tag2")].into_iter().collect(),
    );
    let id1 = collection.insert(entity1);

    // Create second entity
    let url2 = Url::parse("https://example.com/page2").unwrap();
    let entity2 = Entity::new(
        url2,
        datetime!(2024-01-16 0:00 UTC).into(),
        Some(Name::from("Page Two")),
        vec![Label::from("tag2"), Label::from("tag3")].into_iter().collect(),
    );
    let id2 = collection.insert(entity2);

    // Add bidirectional edge between entities
    collection.add_edges(id1, id2);

    collection
}

#[test]
fn test_collection_serialization() {
    let collection = create_test_collection();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&collection).unwrap();

    // Deserialize back to Collection
    let deserialized: Collection = serde_json::from_str(&json).unwrap();

    // Verify the collections are equal
    assert_eq!(collection.len(), deserialized.len());

    assert_eq!(collection, deserialized);

    // Check first entity
    let id1 = Id::new(0);
    assert_eq!(collection.entity(id1).url(), deserialized.entity(id1).url());
    assert_eq!(collection.entity(id1).names(), deserialized.entity(id1).names());
    assert_eq!(collection.entity(id1).labels(), deserialized.entity(id1).labels());

    // Check second entity
    let id2 = Id::new(1);
    assert_eq!(collection.entity(id2).url(), deserialized.entity(id2).url());
    assert_eq!(collection.entity(id2).names(), deserialized.entity(id2).names());
    assert_eq!(collection.entity(id2).labels(), deserialized.entity(id2).labels());

    // Verify edges
    assert_eq!(collection.edges(id1), deserialized.edges(id1));
    assert_eq!(collection.edges(id2), deserialized.edges(id2));
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
    insta::assert_json_snapshot!(collection)
}

#[test]
fn test_update_labels() {
    let mut collection = create_test_collection();

    // Test basic label update
    let update = json!({
        "tag1": "tag1-updated",
        "tag2": "tag2-updated"
    });
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
    let update = json!({});
    collection.update_labels(update).unwrap();

    let id1 = Id::new(0);
    assert_eq!(
        collection.entity(id1).labels(),
        &vec![Label::from("tag1"), Label::from("tag2")].into_iter().collect()
    );
}

#[test]
fn test_update_labels_invalid_json() {
    let mut collection = create_test_collection();

    // Non-object JSON should return error
    let update = json!(["not", "an", "object"]);
    assert!(collection.update_labels(update).is_err());

    // Non-string values should be ignored
    let update = json!({
        "tag1": 42,
        "tag2": "valid-update",
        "tag3": null
    });
    collection.update_labels(update).unwrap();

    let id1 = Id::new(0);
    let labels = collection.entity(id1).labels();
    assert!(labels.contains(&Label::from("tag1"))); // unchanged
    assert!(labels.contains(&Label::from("valid-update")));
    assert!(!labels.contains(&Label::from("tag2")));
}

#[cfg(feature = "pinboard")]
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

    #[test]
    fn snapshot_html_simple() {
        let html = load_fixture("bookmarks_simple.html");
        let collection = Collection::from_html_str(&html).unwrap();
        insta::assert_json_snapshot!(collection);
    }

    #[test]
    fn snapshot_html_folders() {
        let html = load_fixture("bookmarks_folders.html");
        let collection = Collection::from_html_str(&html).unwrap();
        insta::assert_json_snapshot!(collection);
    }

    #[test]
    fn snapshot_html_privacy() {
        let html = load_fixture("bookmarks_privacy.html");
        let collection = Collection::from_html_str(&html).unwrap();
        insta::assert_json_snapshot!(collection);
    }

    #[test]
    fn snapshot_html_feeds() {
        let html = load_fixture("bookmarks_feeds.html");
        let collection = Collection::from_html_str(&html).unwrap();
        insta::assert_json_snapshot!(collection);
    }

    #[test]
    fn snapshot_html_pinboard() {
        let html = load_fixture("bookmarks_pinboard.html");
        let collection = Collection::from_html_str(&html).unwrap();
        insta::assert_json_snapshot!(collection);
    }

    #[test]
    fn test_html_parsing_basic() {
        let html = load_fixture("bookmarks_simple.html");
        let collection = Collection::from_html_str(&html).unwrap();

        assert_eq!(collection.len(), 3);

        // Check that URLs are parsed correctly
        assert!(collection.contains(&Url::parse("https://example.com/").unwrap()));
        assert!(collection.contains(&Url::parse("https://github.com/").unwrap()));
        assert!(collection.contains(&Url::parse("https://news.ycombinator.com/").unwrap()));
    }

    #[test]
    fn test_html_parsing_folders() {
        let html = load_fixture("bookmarks_folders.html");
        let collection = Collection::from_html_str(&html).unwrap();

        assert_eq!(collection.len(), 7); // Updated count for the folders fixture

        // Find GitHub entity and check its labels include folder name
        let github_url = Url::parse("https://github.com/").unwrap();
        if let Some(id) = collection.id(&github_url) {
            let entity = collection.entity(id);
            let labels = entity.labels();
            assert!(labels.contains(&Label::from("Programming")));
            assert!(labels.contains(&Label::from("git")));
            assert!(labels.contains(&Label::from("hosting")));
        } else {
            panic!("GitHub entity not found");
        }
    }

    #[test]
    fn test_html_parsing_privacy() {
        let html = load_fixture("bookmarks_privacy.html");
        let collection = Collection::from_html_str(&html).unwrap();

        assert_eq!(collection.len(), 6);

        // Check private bookmark
        let private_url = Url::parse("https://internal.company.com/").unwrap();
        if let Some(id) = collection.id(&private_url) {
            let entity = collection.entity(id);
            assert_eq!(entity.shared, false); // PRIVATE="1" should set shared=false
        } else {
            panic!("Private entity not found");
        }

        // Check public bookmark
        let public_url = Url::parse("https://example.com/").unwrap();
        if let Some(id) = collection.id(&public_url) {
            let entity = collection.entity(id);
            assert_eq!(entity.shared, true); // PRIVATE="0" should set shared=true
        } else {
            panic!("Public entity not found");
        }

        // Check feed bookmarks
        let feed_url = Url::parse("https://public-feed.example.com/rss").unwrap();
        if let Some(id) = collection.id(&feed_url) {
            let entity = collection.entity(id);
            assert_eq!(entity.is_feed(), true); // FEED="true" should set is_feed=true
        } else {
            panic!("Feed entity not found");
        }
    }
}
