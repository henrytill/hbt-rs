use time::macros::date;

use super::*;

const TEST_BASIC: &str = "\
# November 16, 2023

## Foo

- [Foo](https://foo.com)

### Bar

- <https://bar.com>

## Misc

- [Hello, world!](https://example.com/)
";

#[test]
fn test_basic() {
    let collection = parse(TEST_BASIC).unwrap();

    let expected_date = date!(2023 - 11 - 16);

    assert_eq!(collection.len(), 3);
    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let entity = &collection[1];
    assert_eq!(entity.name(), "https://bar.com");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::from("Foo"), Label::from("Bar")]);

    let entity = &collection[2];
    assert_eq!(entity.name(), "Hello, world!");
    assert_eq!(entity.url().as_str(), "https://example.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::from("Misc")]);
}

const TEST_NESTED: &str = "\
# November 17, 2023

## Foo

- [Foo](https://foo.com)
  - <https://bar.com>
  - [Hello, world!](https://example.com/)
    - [Quux](https://quux.com)
  - <https://baz.com>
";

#[test]
fn test_nested() {
    let collection = parse(TEST_NESTED).unwrap();
    assert_eq!(collection.len(), 5);

    let expected_date = date!(2023 - 11 - 17);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(0).unwrap();
    let expected: Vec<usize> = vec![1, 2, 4];
    let actual: Vec<usize> = edges.iter().cloned().map(Into::into).collect();
    assert_eq!(expected, actual);

    let entity = &collection[1];
    assert_eq!(entity.name(), "https://bar.com");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(1).unwrap();
    let expected: Vec<usize> = vec![0];
    let actual: Vec<usize> = edges.iter().cloned().map(Into::into).collect();
    assert_eq!(expected, actual);

    let entity = &collection[2];
    assert_eq!(entity.name(), "Hello, world!");
    assert_eq!(entity.url().as_str(), "https://example.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(2).unwrap();
    let expected: Vec<usize> = vec![0, 3];
    let actual: Vec<usize> = edges.iter().cloned().map(Into::into).collect();
    assert_eq!(expected, actual);

    let entity = &collection[3];
    assert_eq!(entity.name(), "Quux");
    assert_eq!(entity.url().as_str(), "https://quux.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(3).unwrap();
    let expected: Vec<usize> = vec![2];
    let actual: Vec<usize> = edges.iter().cloned().map(Into::into).collect();
    assert_eq!(expected, actual);

    let entity = &collection[4];
    assert_eq!(entity.name(), "https://baz.com");
    assert_eq!(entity.url().as_str(), "https://baz.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(4).unwrap();
    let expected: Vec<usize> = vec![0];
    let actual: Vec<usize> = edges.iter().cloned().map(Into::into).collect();
    assert_eq!(expected, actual);
}
