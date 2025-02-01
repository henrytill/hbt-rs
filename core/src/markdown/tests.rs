use std::vec;

use time::macros::datetime;

use super::*;
use crate::collection::Time;

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
    let expected = "missing date";
    let actual = parse(TEST_NO_DATE).unwrap_err().to_string();
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();

    {
        let expected = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            expected_time,
            Some(Name::from("Foo")),
            Default::default(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
    }

    {
        let expected = Entity::new(
            Url::parse("https://bar.com").unwrap(),
            expected_time,
            Some(Name::from("Bar")),
            Default::default(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
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
        Url::parse("https://foo.com").unwrap(),
        datetime!(2023-11-15 0:00 UTC).into(),
        Default::default(),
        Default::default(),
    );
    let id = collection.id(expected.url()).unwrap();
    assert_eq!(&expected, collection.entity(id));
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
        Url::parse("https://foo.com").unwrap(),
        datetime!(2023-11-15 0:00 UTC).into(),
        Some(Name::from("Foo")),
        Default::default(),
    );
    let id = collection.id(expected.url()).unwrap();
    assert_eq!(&expected, collection.entity(id));
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();

    let foo_expected = Entity::new(
        Url::parse("https://foo.com").unwrap(),
        expected_time,
        Some(Name::from("Foo")),
        Default::default(),
    );
    let foo_id = collection.id(foo_expected.url()).unwrap();
    assert_eq!(&foo_expected, collection.entity(foo_id));
    let foo_edges = collection.edges(foo_id);
    assert_eq!(foo_edges.len(), 1);

    let bar_expected = Entity::new(
        Url::parse("https://bar.com").unwrap(),
        expected_time,
        Some(Name::from("Bar")),
        Default::default(),
    );
    let bar_id = collection.id(bar_expected.url()).unwrap();
    assert_eq!(&bar_expected, collection.entity(bar_id));
    let bar_edges = collection.edges(bar_id);
    assert_eq!(bar_edges.len(), 1);

    assert_eq!(foo_edges, vec![bar_id]);
    assert_eq!(bar_edges, vec![foo_id]);
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();

    let foo_expected = Entity::new(
        Url::parse("https://foo.com").unwrap(),
        expected_time,
        Some(Name::from("Foo")),
        Default::default(),
    );
    let foo_id = collection.id(foo_expected.url()).unwrap();
    assert_eq!(&foo_expected, collection.entity(foo_id));
    let foo_edges = collection.edges(foo_id);
    assert_eq!(foo_edges.len(), 1);

    let bar_expected = Entity::new(
        Url::parse("https://bar.com").unwrap(),
        expected_time,
        Some(Name::from("Bar")),
        Default::default(),
    );
    let bar_id = collection.id(bar_expected.url()).unwrap();
    assert_eq!(&bar_expected, collection.entity(bar_id));
    let bar_edges = collection.edges(bar_id);
    assert_eq!(bar_edges.len(), 2);

    let baz_expected = Entity::new(
        Url::parse("https://baz.com").unwrap(),
        expected_time,
        Some(Name::from("Baz")),
        Default::default(),
    );
    let baz_id = collection.id(baz_expected.url()).unwrap();
    assert_eq!(&baz_expected, collection.entity(baz_id));
    let baz_edges = collection.edges(baz_id);
    assert_eq!(baz_edges.len(), 1);

    assert_eq!(foo_edges, vec![bar_id]);
    assert_eq!(bar_edges, vec![foo_id, baz_id]);
    assert_eq!(baz_edges, vec![bar_id]);
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();

    let foo_expected = Entity::new(
        Url::parse("https://foo.com").unwrap(),
        expected_time,
        Some(Name::from("Foo")),
        Default::default(),
    );
    let foo_id = collection.id(foo_expected.url()).unwrap();
    assert_eq!(&foo_expected, collection.entity(foo_id));
    let foo_edges = collection.edges(foo_id);
    assert_eq!(foo_edges.len(), 1);

    let bar_expected = Entity::new(
        Url::parse("https://bar.com").unwrap(),
        expected_time,
        Some(Name::from("Bar")),
        Default::default(),
    );
    let bar_id = collection.id(bar_expected.url()).unwrap();
    assert_eq!(&bar_expected, collection.entity(bar_id));
    let bar_edges = collection.edges(bar_id);
    assert_eq!(bar_edges.len(), 2);

    let baz_expected = Entity::new(
        Url::parse("https://baz.com").unwrap(),
        expected_time,
        Some(Name::from("Baz")),
        Default::default(),
    );
    let baz_id = collection.id(baz_expected.url()).unwrap();
    assert_eq!(&baz_expected, collection.entity(baz_id));
    let baz_edges = collection.edges(baz_id);
    assert_eq!(baz_edges.len(), 1);

    assert_eq!(foo_edges, vec![bar_id]);
    assert_eq!(bar_edges, vec![foo_id, baz_id]);
    assert_eq!(baz_edges, vec![bar_id]);
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();

    let foo_expected = Entity::new(
        Url::parse("https://foo.com").unwrap(),
        expected_time,
        Some(Name::from("Foo")),
        Default::default(),
    );
    let foo_id = collection.id(foo_expected.url()).unwrap();
    assert_eq!(&foo_expected, collection.entity(foo_id));
    let foo_edges = collection.edges(foo_id);
    assert_eq!(foo_edges.len(), 3);

    let bar_expected = Entity::new(
        Url::parse("https://bar.com").unwrap(),
        expected_time,
        Some(Name::from("Bar")),
        Default::default(),
    );

    let bar_id = collection.id(bar_expected.url()).unwrap();
    assert_eq!(&bar_expected, collection.entity(bar_id));
    let bar_edges = collection.edges(bar_id);
    assert_eq!(bar_edges.len(), 1);

    let baz_expected = Entity::new(
        Url::parse("https://baz.com").unwrap(),
        expected_time,
        Some(Name::from("Baz")),
        Default::default(),
    );
    let baz_id = collection.id(baz_expected.url()).unwrap();
    assert_eq!(&baz_expected, collection.entity(baz_id));
    let baz_edges = collection.edges(baz_id);
    assert_eq!(baz_edges.len(), 1);

    let quux_expected = Entity::new(
        Url::parse("https://quux.com").unwrap(),
        expected_time,
        Some(Name::from("Quux")),
        Default::default(),
    );
    let quux_id = collection.id(quux_expected.url()).unwrap();
    assert_eq!(&quux_expected, collection.entity(quux_id));
    let quux_edges = collection.edges(quux_id);
    assert_eq!(quux_edges.len(), 1);

    assert_eq!(foo_edges, vec![bar_id, baz_id, quux_id]);
    assert_eq!(bar_edges, vec![foo_id]);
    assert_eq!(baz_edges, vec![foo_id]);
    assert_eq!(quux_edges, vec![foo_id]);
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();

    {
        let expected = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            expected_time,
            Some(Name::from("Foo")),
            Default::default(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://bar.com").unwrap(),
            expected_time,
            Some(Name::from("Bar")),
            Default::default(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();

    {
        let expected = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            expected_time,
            Some(Name::from("Foo")),
            Default::default(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://bar.com").unwrap(),
            expected_time,
            Some(Name::from("Bar")),
            Default::default(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://baz.com").unwrap(),
            expected_time,
            Some(Name::from("Baz")),
            Default::default(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();
    let expected_labels = [Label::from("Foo")];

    {
        let expected = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            expected_time,
            Some(Name::from("Foo")),
            expected_labels.iter().cloned().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://bar.com").unwrap(),
            expected_time,
            Some(Name::from("Bar")),
            expected_labels.into_iter().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();
    let expected_labels = [Label::from("Foo")];

    {
        let expected = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            expected_time,
            Some(Name::from("Foo")),
            expected_labels.iter().cloned().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://bar.com").unwrap(),
            expected_time,
            Some(Name::from("Bar")),
            expected_labels.iter().cloned().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    let expected_labels = [Label::from("Baz")];

    {
        let expected = Entity::new(
            Url::parse("https://baz.com").unwrap(),
            expected_time,
            Some(Name::from("Baz")),
            expected_labels.iter().cloned().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://quux.com").unwrap(),
            expected_time,
            Some(Name::from("Quux")),
            expected_labels.into_iter().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
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

    let expected_time = datetime!(2023-11-15 0:00 UTC).into();

    {
        let expected = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            expected_time,
            Some(Name::from("Foo")),
            [Label::from("Foo")].into_iter().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://bar.com").unwrap(),
            expected_time,
            Some(Name::from("Bar")),
            [Label::from("Foo"), Label::from("Bar")].into_iter().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://baz.com").unwrap(),
            expected_time,
            Some(Name::from("Baz")),
            [Label::from("Foo"), Label::from("Bar"), Label::from("Baz")].into_iter().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }
}

const TEST_UPDATE: &str = "\
# December 5, 2023

## Foo

- [Foo](https://foo.com)

# December 6, 2023

## Bar

- [Bar](https://foo.com)
";

#[test]
fn test_update() {
    let collection = parse(TEST_UPDATE).unwrap();
    assert_eq!(collection.len(), 1);

    {
        let mut expected = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            datetime!(2023-12-5 0:00 UTC).into(),
            Some(Name::from("Foo")),
            [Label::from("Foo")].into_iter().collect(),
        );

        expected.update(
            datetime!(2023-12-6 0:00 UTC).into(),
            [Name::from("Bar")].into_iter().collect(),
            [Label::from("Bar")].into_iter().collect(),
        );

        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
    }
}

const TEST_DESCENDING_DATES: &str = "\
# December 6, 2023

## Foo

- [Foo](https://foo.com)

# December 5, 2023

## Bar

- [Bar](https://foo.com)
";

#[test]
fn test_descending_dates() {
    let collection = parse(TEST_DESCENDING_DATES).unwrap();
    assert_eq!(collection.len(), 1);

    let mut expected = Entity::new(
        Url::parse("https://foo.com").unwrap(),
        datetime!(2023-12-6 0:00 UTC).into(),
        Some(Name::from("Foo")),
        [Label::from("Foo")].into_iter().collect(),
    );

    expected.update(
        datetime!(2023-12-5 0:00 UTC).into(),
        [Name::from("Bar")].into_iter().collect(),
        [Label::from("Bar")].into_iter().collect(),
    );

    let id = collection.id(expected.url()).unwrap();
    let actual = collection.entity(id);
    assert_eq!(&expected, actual);
    assert_eq!(actual.created_at(), &Time::new(datetime!(2023-12-5 0:00 UTC)));
    assert_eq!(actual.updated_at(), &[Time::new(datetime!(2023-12-6 0:00 UTC))]);
}

const TEST_MIXED_DATES: &str = "\
# December 6, 2023

## Foo

- [Foo](https://foo.com)

# December 5, 2023

## Bar

- [Bar](https://foo.com)

# December 7, 2023

## Baz

- [Baz](https://foo.com)
";

#[test]
fn test_mixed_dates() {
    let collection = parse(TEST_MIXED_DATES).unwrap();
    assert_eq!(collection.len(), 1);

    let mut expected = Entity::new(
        Url::parse("https://foo.com").unwrap(),
        datetime!(2023-12-6 0:00 UTC).into(),
        Some(Name::from("Foo")),
        [Label::from("Foo")].into_iter().collect(),
    );

    expected.update(
        datetime!(2023-12-5 0:00 UTC).into(),
        [Name::from("Bar")].into_iter().collect(),
        [Label::from("Bar")].into_iter().collect(),
    );

    expected.update(
        datetime!(2023-12-7 0:00 UTC).into(),
        [Name::from("Baz")].into_iter().collect(),
        [Label::from("Baz")].into_iter().collect(),
    );

    let id = collection.id(expected.url()).unwrap();
    let actual = collection.entity(id);
    assert_eq!(&expected, actual);
    assert_eq!(actual.created_at(), &Time::new(datetime!(2023-12-5 0:00 UTC)));
    assert_eq!(
        actual.updated_at(),
        &[Time::new(datetime!(2023-12-6 0:00 UTC)), Time::new(datetime!(2023-12-7 0:00 UTC))]
    );
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

    let expected_time = datetime!(2023-11-16 0:00 UTC).into();

    {
        let expected = Entity::new(
            Url::parse("https://foo.com").unwrap(),
            expected_time,
            Some(Name::from("Foo")),
            [Label::from("Foo")].into_iter().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://bar.com").unwrap(),
            expected_time,
            Default::default(),
            [Label::from("Foo"), Label::from("Bar")].into_iter().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
    }

    {
        let expected = Entity::new(
            Url::parse("https://example.com").unwrap(),
            expected_time,
            Some(Name::from("Hello, world!")),
            [Label::from("Misc")].into_iter().collect(),
        );
        let id = collection.id(expected.url()).unwrap();
        assert_eq!(&expected, collection.entity(id));
        assert!(collection.edges(id).is_empty());
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

    let expected_time = datetime!(2023-11-17 0:00 UTC).into();
    let expected_labels = [Label::from("Foo")];

    let foo_expected = Entity::new(
        Url::parse("https://foo.com").unwrap(),
        expected_time,
        Some(Name::from("Foo")),
        expected_labels.iter().cloned().collect(),
    );
    let foo_id = collection.id(foo_expected.url()).unwrap();
    assert_eq!(&foo_expected, collection.entity(foo_id));
    let foo_edges = collection.edges(foo_id);
    assert_eq!(foo_edges.len(), 3);

    let bar_expected = Entity::new(
        Url::parse("https://bar.com").unwrap(),
        expected_time,
        Default::default(),
        expected_labels.iter().cloned().collect(),
    );
    let bar_id = collection.id(bar_expected.url()).unwrap();
    assert_eq!(&bar_expected, collection.entity(bar_id));
    let bar_edges = collection.edges(bar_id);
    assert_eq!(bar_edges.len(), 1);

    let hello_expected = Entity::new(
        Url::parse("https://example.com").unwrap(),
        expected_time,
        Some(Name::from("Hello, world!")),
        expected_labels.iter().cloned().collect(),
    );
    let hello_id = collection.id(hello_expected.url()).unwrap();
    assert_eq!(&hello_expected, collection.entity(hello_id));
    let hello_edges = collection.edges(hello_id);
    assert_eq!(hello_edges.len(), 2);

    let quux_expected = Entity::new(
        Url::parse("https://quux.com").unwrap(),
        expected_time,
        Some(Name::from("Quux")),
        expected_labels.iter().cloned().collect(),
    );
    let quux_id = collection.id(quux_expected.url()).unwrap();
    assert_eq!(&quux_expected, collection.entity(quux_id));
    let quux_edges = collection.edges(quux_id);
    assert_eq!(quux_edges.len(), 1);

    let baz_expected = Entity::new(
        Url::parse("https://baz.com").unwrap(),
        expected_time,
        Default::default(),
        expected_labels.into_iter().collect(),
    );
    let baz_id = collection.id(baz_expected.url()).unwrap();
    assert_eq!(&baz_expected, collection.entity(baz_id));
    let baz_edges = collection.edges(baz_id);
    assert_eq!(baz_edges.len(), 1);

    assert_eq!(foo_edges, vec![bar_id, hello_id, baz_id]);
    assert_eq!(bar_edges, vec![foo_id]);
    assert_eq!(hello_edges, vec![foo_id, quux_id]);
    assert_eq!(quux_edges, vec![hello_id]);
    assert_eq!(baz_edges, vec![foo_id]);
}
