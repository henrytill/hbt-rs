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
