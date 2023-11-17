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
    let mut reader = TEST_BASIC.as_bytes();
    let entities = parse(&mut reader).unwrap();
    assert_eq!(entities.len(), 3);
    let entity = &entities[0];
    assert_eq!(entity.name(), "Foo");
    assert_eq!(entity.url().as_str(), "https://foo.com/");
    assert_eq!(entity.created_at(), &date!(2023 - 11 - 16));
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::new("Foo".to_string())]);

    let entity = &entities[1];
    assert_eq!(entity.name(), "https://bar.com");
    assert_eq!(entity.url().as_str(), "https://bar.com/");
    assert_eq!(entity.created_at(), &date!(2023 - 11 - 16));
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(
        entity.labels(),
        &[Label::new("Foo".to_string()), Label::new("Bar".to_string())]
    );

    let entity = &entities[2];
    assert_eq!(entity.name(), "Hello, world!");
    assert_eq!(entity.url().as_str(), "https://example.com/");
    assert_eq!(entity.created_at(), &date!(2023 - 11 - 16));
    assert_eq!(entity.updated_at().len(), 0);
    assert_eq!(entity.labels(), &[Label::new("Misc".to_string())]);
}
