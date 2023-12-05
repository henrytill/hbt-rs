use std::vec;

use time::macros::date;

use super::*;

fn convert_edges(edges: &[Id]) -> Vec<usize> {
    edges.iter().cloned().map(Into::into).collect()
}

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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[0]);
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[1]);
    }
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

    let expected = Entity::new(
        Vec::new(),
        Url::parse("https://foo.com").unwrap(),
        date!(2023 - 11 - 15),
        Vec::new(),
    );

    assert_eq!(&expected, &collection[0]);
}

const TEST_INDENTED: &str = "\
# November 15, 2023

  - [Foo](https://foo.com)
";

#[test]
fn test_indented() {
    let collection = parse(TEST_INDENTED).unwrap();

    assert_eq!(collection.len(), 1);

    let expected = Entity::new(
        vec!["Foo".to_string()],
        Url::parse("https://foo.com").unwrap(),
        date!(2023 - 11 - 15),
        Vec::new(),
    );

    assert_eq!(&expected, &collection[0]);
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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![1]);
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![0]);
    }
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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![1]);
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 2);
        assert_eq!(edges, vec![0, 2]);
    }

    {
        let expected = Entity::new(
            vec!["Baz".to_string()],
            Url::parse("https://baz.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[2]);

        let edges = collection.edges(2usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![1]);
    }
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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![1]);
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 2);
        assert_eq!(edges, vec![0, 2]);
    }

    {
        let expected = Entity::new(
            vec!["Baz".to_string()],
            Url::parse("https://baz.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[2]);

        let edges = collection.edges(2usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![1]);
    }
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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 3);
        assert_eq!(edges, vec![1, 2, 3]);
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![0]);
    }

    {
        let expected = Entity::new(
            vec!["Baz".to_string()],
            Url::parse("https://baz.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[2]);

        let edges = collection.edges(2usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![0]);
    }

    {
        let expected = Entity::new(
            vec!["Quux".to_string()],
            Url::parse("https://quux.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[3]);

        let edges = collection.edges(3usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges, vec![0]);
    }
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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        let edges = convert_edges(edges);
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        let edges = convert_edges(edges);
        assert!(edges.is_empty());
    }
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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        let edges = convert_edges(edges);
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        let edges = convert_edges(edges);
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Baz".to_string()],
            Url::parse("https://baz.com").unwrap(),
            expected_date,
            Vec::new(),
        );

        assert_eq!(&expected, &collection[2]);

        let edges = collection.edges(2usize).unwrap();
        let edges = convert_edges(edges);
        assert!(edges.is_empty());
    }
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
    let expected_labels = vec![Label::from("Foo")];

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        assert!(edges.is_empty());
    }
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
    let expected_labels = vec![Label::from("Foo")];

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        assert!(edges.is_empty());
    }

    let expected_labels = vec![Label::from("Baz")];

    {
        let expected = Entity::new(
            vec!["Baz".to_string()],
            Url::parse("https://baz.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[2]);

        let edges = collection.edges(2usize).unwrap();
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Quux".to_string()],
            Url::parse("https://quux.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[3]);

        let edges = collection.edges(3usize).unwrap();
        assert!(edges.is_empty());
    }
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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            vec![Label::from("Foo")],
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Bar".to_string()],
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            vec![Label::from("Foo"), Label::from("Bar")],
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Baz".to_string()],
            Url::parse("https://baz.com").unwrap(),
            expected_date,
            vec![Label::from("Foo"), Label::from("Bar"), Label::from("Baz")],
        );

        assert_eq!(&expected, &collection[2]);

        let edges = collection.edges(2usize).unwrap();
        assert!(edges.is_empty());
    }
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

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            vec![Label::from("Foo")],
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        let edges = convert_edges(edges);
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            Vec::new(),
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            vec![Label::from("Foo"), Label::from("Bar")],
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        let edges = convert_edges(edges);
        assert!(edges.is_empty());
    }

    {
        let expected = Entity::new(
            vec!["Hello, world!".to_string()],
            Url::parse("https://example.com").unwrap(),
            expected_date,
            vec![Label::from("Misc")],
        );

        assert_eq!(&expected, &collection[2]);

        let edges = collection.edges(2usize).unwrap();
        let edges = convert_edges(edges);
        assert!(edges.is_empty());
    }
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
    let expected_labels = vec![Label::from("Foo")];

    {
        let expected = Entity::new(
            vec!["Foo".to_string()],
            Url::parse("https://foo.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[0]);

        let edges = collection.edges(0usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(vec![1, 2, 4], edges);
    }

    {
        let expected = Entity::new(
            Vec::new(),
            Url::parse("https://bar.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[1]);

        let edges = collection.edges(1usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(vec![0], edges);
    }

    {
        let expected = Entity::new(
            vec!["Hello, world!".to_string()],
            Url::parse("https://example.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[2]);

        let edges = collection.edges(2usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(vec![0, 3], edges);
    }

    {
        let expected = Entity::new(
            vec!["Quux".to_string()],
            Url::parse("https://quux.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[3]);

        let edges = collection.edges(3usize).unwrap();
        let actual: Vec<usize> = convert_edges(edges);
        let expected: Vec<usize> = vec![2];
        assert_eq!(expected, actual);
    }

    {
        let expected = Entity::new(
            Vec::new(),
            Url::parse("https://baz.com").unwrap(),
            expected_date,
            expected_labels.to_owned(),
        );

        assert_eq!(&expected, &collection[4]);

        let edges = collection.edges(4usize).unwrap();
        let edges = convert_edges(edges);
        assert_eq!(vec![0], edges);
    }
}
