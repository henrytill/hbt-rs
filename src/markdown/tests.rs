use time::macros::date;

use super::*;

const TEST_EMPTY: &str = "";

#[test]
fn test_empty() {
    let collection = parse(TEST_EMPTY).unwrap();
    assert!(collection.is_empty());
}

const TEST_NO_DATE: &str = "\
- [Foo](https://foo.com)
";

#[test]
fn test_no_date() {
    let expected = Error::new(ErrorImpl::MissingDate);
    let actual = parse(TEST_NO_DATE).expect_err("Expected error");
    assert_eq!(expected, actual);
}

const TEST_ONLY_DATE: &str = "\
# November 15, 2023
";

#[test]
fn test_only_date() {
    let collection = parse(TEST_ONLY_DATE).unwrap();
    assert!(collection.is_empty());
}

const TEST_NO_LABELS: &str = "\
# November 15, 2023

- [Foo](https://foo.com)
- [Bar](https://bar.com)
";

#[test]
fn test_no_labels() {
    let collection = parse(TEST_NO_LABELS).unwrap();
    assert_eq!(collection.len(), 2);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());
}

const TEST_NO_URL: &str = "\
# November 15, 2023

- Foo
";

#[test]
fn test_no_url() {
    let collection = parse(TEST_NO_URL).unwrap();
    assert!(collection.is_empty());
}

const TEST_NO_TITLE: &str = "\
# November 15, 2023

- <https://foo.com>
";

#[test]
fn test_no_title() {
    let collection = parse(TEST_NO_TITLE).unwrap();
    assert_eq!(collection.len(), 1);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "https://foo.com");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());
}

const TEST_INDENTED: &str = "\
# November 15, 2023

  - [Foo](https://foo.com)
";

#[test]
fn test_indented() {
    let collection = parse(TEST_INDENTED).unwrap();
    assert_eq!(collection.len(), 1);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());
}

const TEST_INDENTED_DOUBLE: &str = "\
# November 15, 2023

    - [Foo](https://foo.com)
";

#[test]
fn test_indented_double() {
    let collection = parse(TEST_INDENTED_DOUBLE).unwrap();
    assert!(collection.is_empty());
}

const TEST_PARENT: &str = "\
# November 15, 2023

- [Foo](https://foo.com)
  - [Bar](https://bar.com)
";

#[test]
fn test_parent() {
    let collection = parse(TEST_PARENT).unwrap();
    assert_eq!(collection.len(), 2);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(0).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 1);

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(1).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 0);
}

const TEST_PARENTS: &str = "\
# November 15, 2023

- [Foo](https://foo.com)
  - [Bar](https://bar.com)
    - [Baz](https://baz.com)
";

#[test]
fn test_parents() {
    let collection = parse(TEST_PARENTS).unwrap();
    assert_eq!(collection.len(), 3);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(0).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 1);

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(1).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0], 0);
    assert_eq!(edges[1], 2);

    let entity = &collection[2];
    assert_eq!(entity.name(), "Baz");
    assert_eq!(entity.url().as_str(), "https://baz.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(2).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 1);
}

const TEST_PARENTS_INDENTED: &str = "\
# November 15, 2023

  - [Foo](https://foo.com)
    - [Bar](https://bar.com)
      - [Baz](https://baz.com)
";

#[test]
fn test_parents_indented() {
    let collection = parse(TEST_PARENTS_INDENTED).unwrap();
    assert_eq!(collection.len(), 3);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(0).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 1);

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(1).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0], 0);
    assert_eq!(edges[1], 2);

    let entity = &collection[2];
    assert_eq!(entity.name(), "Baz");
    assert_eq!(entity.url().as_str(), "https://baz.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(2).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 1);
}

const TEST_SINGLE_PARENT: &str = "\
# November 15, 2023

- [Foo](https://foo.com)
  - [Bar](https://bar.com)
  - [Baz](https://baz.com)
  - [Quux](https://quux.com)
";

#[test]
fn test_single_parent() {
    let collection = parse(TEST_SINGLE_PARENT).unwrap();
    assert_eq!(collection.len(), 4);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(0).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 3);
    assert_eq!(edges[0], 1);
    assert_eq!(edges[1], 2);
    assert_eq!(edges[2], 3);

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(1).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 0);

    let entity = &collection[2];
    assert_eq!(entity.name(), "Baz");
    assert_eq!(entity.url().as_str(), "https://baz.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(2).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 0);

    let entity = &collection[3];
    assert_eq!(entity.name(), "Quux");
    assert_eq!(entity.url().as_str(), "https://quux.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(3).unwrap();
    let edges = convert_edges(edges);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], 0);
}

const TEST_INVERTED_PARENT: &str = "\
# November 15, 2023

  - [Foo](https://foo.com)
- [Bar](https://bar.com)
";

#[test]
fn test_no_parent() {
    let collection = parse(TEST_INVERTED_PARENT).unwrap();
    assert_eq!(collection.len(), 2);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(0).unwrap();
    assert!(edges.is_empty());

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(1).unwrap();
    assert!(edges.is_empty());
}

const TEST_INVERTED_SINGLE_PARENT: &str = "\
# November 15, 2023

  - [Foo](https://foo.com)
  - [Bar](https://bar.com)
- [Baz](https://baz.com)
";

#[test]
fn test_inverted_parents() {
    let collection = parse(TEST_INVERTED_SINGLE_PARENT).unwrap();
    assert_eq!(collection.len(), 3);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(0).unwrap();
    assert!(edges.is_empty());

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(1).unwrap();
    assert!(edges.is_empty());

    let entity = &collection[2];
    assert_eq!(entity.name(), "Baz");
    assert_eq!(entity.url().as_str(), "https://baz.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert!(entity.labels().is_empty());

    let edges = collection.edges(2).unwrap();
    assert!(edges.is_empty());
}

const TEST_LABEL: &str = "\
# November 15, 2023

## Foo

- [Foo](https://foo.com)
- [Bar](https://bar.com)
";

#[test]
fn test_label() {
    let collection = parse(TEST_LABEL).unwrap();
    assert_eq!(collection.len(), 2);

    let expected_date = date!(2023 - 11 - 15);
    let expected_labels = &[Label::from("Foo")];

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.labels(), expected_labels);

    let edges = collection.edges(0).unwrap();
    assert!(edges.is_empty());

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.labels(), expected_labels);

    let edges = collection.edges(1).unwrap();
    assert!(edges.is_empty());
}

const TEST_LABELS: &str = "\
# November 15, 2023

## Foo

- [Foo](https://foo.com)
- [Bar](https://bar.com)

## Baz

- [Baz](https://baz.com)
- [Quux](https://quux.com)
";

#[test]
fn test_labels() {
    let collection = parse(TEST_LABELS).unwrap();
    assert_eq!(collection.len(), 4);

    let expected_date = date!(2023 - 11 - 15);
    let expected_labels = &[Label::from("Foo")];

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.labels(), expected_labels);

    let edges = collection.edges(0).unwrap();
    assert!(edges.is_empty());

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.labels(), expected_labels);

    let edges = collection.edges(1).unwrap();
    assert!(edges.is_empty());

    let expected_labels = &[Label::from("Baz")];

    let entity = &collection[2];
    assert_eq!(entity.name(), "Baz");
    assert_eq!(entity.url().as_str(), "https://baz.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.labels(), expected_labels);

    let edges = collection.edges(2).unwrap();
    assert!(edges.is_empty());

    let entity = &collection[3];
    assert_eq!(entity.name(), "Quux");
    assert_eq!(entity.url().as_str(), "https://quux.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.labels(), expected_labels);

    let edges = collection.edges(3).unwrap();
    assert!(edges.is_empty());
}

const TEST_MULTIPLE_LABELS: &str = "\
# November 15, 2023

## Foo

- [Foo](https://foo.com)

### Bar

- [Bar](https://bar.com)

#### Baz

- [Baz](https://baz.com)
";

#[test]
fn test_multiple_labels() {
    let collection = parse(TEST_MULTIPLE_LABELS).unwrap();
    assert_eq!(collection.len(), 3);

    let expected_date = date!(2023 - 11 - 15);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(0).unwrap();
    assert!(edges.is_empty());

    let entity = &collection[1];
    assert_eq!(entity.name(), "Bar");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(entity.labels(), &[Label::from("Foo"), Label::from("Bar")]);

    let edges = collection.edges(1).unwrap();
    assert!(edges.is_empty());

    let entity = &collection[2];
    assert_eq!(entity.name(), "Baz");
    assert_eq!(entity.url().as_str(), "https://baz.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert_eq!(
        entity.labels(),
        &[Label::from("Foo"), Label::from("Bar"), Label::from("Baz")]
    );

    let edges = collection.edges(2).unwrap();
    assert!(edges.is_empty());
}

// Original tests below

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
    assert_eq!(collection.len(), 3);

    let expected_date = date!(2023 - 11 - 16);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let entity = &collection[1];
    assert_eq!(entity.name(), "https://bar.com");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert_eq!(entity.labels(), &[Label::from("Foo"), Label::from("Bar")]);

    let entity = &collection[2];
    assert_eq!(entity.name(), "Hello, world!");
    assert_eq!(entity.url().as_str(), "https://example.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
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

fn convert_edges(edges: &[Id]) -> Vec<usize> {
    edges.iter().cloned().map(Into::into).collect()
}

#[test]
fn test_nested() {
    let collection = parse(TEST_NESTED).unwrap();
    assert_eq!(collection.len(), 5);

    let expected_date = date!(2023 - 11 - 17);

    let entity = &collection[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(0).unwrap();
    let expected: Vec<usize> = vec![1, 2, 4];
    let actual: Vec<usize> = convert_edges(edges);
    assert_eq!(expected, actual);

    let entity = &collection[1];
    assert_eq!(entity.name(), "https://bar.com");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(1).unwrap();
    let expected: Vec<usize> = vec![0];
    let actual: Vec<usize> = convert_edges(edges);
    assert_eq!(expected, actual);

    let entity = &collection[2];
    assert_eq!(entity.name(), "Hello, world!");
    assert_eq!(entity.url().as_str(), "https://example.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(2).unwrap();
    let expected: Vec<usize> = vec![0, 3];
    let actual: Vec<usize> = convert_edges(edges);
    assert_eq!(expected, actual);

    let entity = &collection[3];
    assert_eq!(entity.name(), "Quux");
    assert_eq!(entity.url().as_str(), "https://quux.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(3).unwrap();
    let expected: Vec<usize> = vec![2];
    let actual: Vec<usize> = convert_edges(edges);
    assert_eq!(expected, actual);

    let entity = &collection[4];
    assert_eq!(entity.name(), "https://baz.com");
    assert_eq!(entity.url().as_str(), "https://baz.com/");
    assert_eq!(entity.created_at(), &expected_date);
    assert!(entity.updated_at().is_empty());
    assert_eq!(entity.labels(), &[Label::from("Foo")]);

    let edges = collection.edges(4).unwrap();
    let expected: Vec<usize> = vec![0];
    let actual: Vec<usize> = convert_edges(edges);
    assert_eq!(expected, actual);
}
