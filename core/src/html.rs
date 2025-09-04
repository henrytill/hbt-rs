use std::collections::{BTreeSet, HashMap};

use minijinja::{Environment, context};
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;
use url::Url;

use crate::collection::{Collection, Entity, Extended, Label, Name, Time};

#[derive(Debug, Error)]
pub enum Error {
    #[error("URL parsing error: {0}")]
    ParseUrl(#[from] url::ParseError),
    #[error("HTML selector error: {0}")]
    HtmlSelector(String),
    #[error("HTML missing required attribute: {0}")]
    HtmlAttribute(String),
    #[error("Template error: {0}")]
    Template(#[from] minijinja::Error),
}

impl From<scraper::error::SelectorErrorKind<'_>> for Error {
    fn from(value: scraper::error::SelectorErrorKind<'_>) -> Self {
        Error::HtmlSelector(value.to_string())
    }
}

#[derive(Debug)]
enum StackItem<'a> {
    Element(ElementRef<'a>),
    PopGroup,
}

type Attributes = HashMap<String, String>;

fn parse_timestamp_attr_opt(attrs: &Attributes, key: &str) -> Result<Option<Time>, Error> {
    if let Some(timestamp_str) = attrs.get(key) {
        let trimmed = timestamp_str.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let time = Time::parse(trimmed).unwrap(); // TODO
        Ok(Some(time))
    } else {
        Ok(None)
    }
}

fn parse_timestamp_attr(attrs: &Attributes, key: &str) -> Result<Time, Error> {
    parse_timestamp_attr_opt(attrs, key).map(Option::unwrap_or_default)
}

fn add_pending(
    collection: &mut Collection,
    folder_stack: &[String],
    attrs: Attributes,
    description: Option<String>,
    extended: Option<String>,
) -> Result<(), Error> {
    const ATTR_HREF: &str = "href";
    const ATTR_ADD_DATE: &str = "add_date";
    const ATTR_LAST_MODIFIED: &str = "last_modified";
    const ATTR_LAST_VISIT: &str = "last_visit";
    const ATTR_TAGS: &str = "tags";
    const ATTR_TOREAD: &str = "toread";
    const ATTR_PRIVATE: &str = "private";
    const ATTR_FEED: &str = "feed";

    let url = {
        let href = attrs.get(ATTR_HREF).ok_or(Error::HtmlAttribute(String::from(ATTR_HREF)))?;
        Url::parse(href)?
    };

    let created_at = parse_timestamp_attr(&attrs, ATTR_ADD_DATE)?;
    let last_modified = parse_timestamp_attr_opt(&attrs, ATTR_LAST_MODIFIED)?;
    let last_visited_at = parse_timestamp_attr_opt(&attrs, ATTR_LAST_VISIT)?;

    let tag_string = attrs.get(ATTR_TAGS).cloned().unwrap_or_default();
    let tags: Vec<String> = if tag_string.is_empty() {
        Vec::new()
    } else {
        tag_string.split(',').map(|s| s.trim().to_string()).collect()
    };

    let labels: BTreeSet<Label> = folder_stack
        .iter()
        .chain(tags.iter())
        .filter(|&tag| tag != ATTR_TOREAD)
        .map(|tag| Label::from(tag.clone()))
        .collect();

    let shared = !matches!(attrs.get(ATTR_PRIVATE), Some(val) if val == "1");

    let to_read =
        attrs.get(ATTR_TOREAD).is_some_and(|val| val == "1") || tag_string.contains(ATTR_TOREAD);

    let is_feed = attrs.get(ATTR_FEED).is_some_and(|val| val == "true");

    let updated_at: Vec<Time> = last_modified.into_iter().collect();

    let entity = Entity::new(url, created_at, description.map(Name::from), labels)
        .with_extended(extended.map(Extended::from))
        .with_shared(shared)
        .with_to_read(to_read)
        .with_last_visited_at(last_visited_at)
        .with_is_feed(is_feed)
        .with_updated_at(updated_at);

    collection.upsert(entity);

    Ok(())
}

fn maybe_element_text(element: ElementRef) -> Option<String> {
    let trimmed = element.text().collect::<String>().trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

fn extract_attributes(element: ElementRef) -> Attributes {
    let mut attrs = HashMap::new();
    for (name, value) in element.value().attrs() {
        attrs.insert(name.to_lowercase(), value.to_string());
    }
    attrs
}

pub fn from_html(html: &str) -> Result<Collection, Error> {
    let document = Html::parse_document(html);
    let root = document.root_element();

    let mut collection = Collection::new();
    let mut stack: Vec<StackItem> = Vec::new();
    let mut folder_stack: Vec<String> = Vec::new();
    let mut pending_bookmark: Option<(Attributes, Option<String>)> = None;

    const A: &str = "a";
    const H3: &str = "h3";
    const DT: &str = "dt";
    const DD: &str = "dd";
    const DL: &str = "dl";

    let a_selector = Selector::parse(A)?;
    let h3_selector = Selector::parse(H3)?;

    for child in root.children().rev() {
        if let Some(child_element) = ElementRef::wrap(child) {
            stack.push(StackItem::Element(child_element));
        }
    }

    while let Some(item) = stack.pop() {
        match item {
            StackItem::Element(element) => {
                match element.value().name() {
                    DT => {
                        if let Some((attrs, maybe_description)) = pending_bookmark.take() {
                            add_pending(
                                &mut collection,
                                &folder_stack,
                                attrs,
                                maybe_description,
                                None, // No extended
                            )?;
                        }
                        if let Some(h3_element) = element.select(&h3_selector).next() {
                            if let Some(folder_name) = maybe_element_text(h3_element) {
                                folder_stack.push(folder_name);
                            }
                        } else if let Some(a_element) = element.select(&a_selector).next() {
                            let attrs = extract_attributes(a_element);
                            let maybe_description = maybe_element_text(a_element);
                            pending_bookmark = Some((attrs, maybe_description));
                        }
                    }
                    DD => {
                        if let Some((attrs, maybe_description)) = pending_bookmark.take() {
                            let maybe_extended = maybe_element_text(element);
                            add_pending(
                                &mut collection,
                                &folder_stack,
                                attrs,
                                maybe_description,
                                maybe_extended,
                            )?;
                        }
                    }
                    DL => {
                        stack.push(StackItem::PopGroup);
                    }
                    _ => {}
                }
                for child in element.children().rev() {
                    if let Some(child_element) = ElementRef::wrap(child) {
                        stack.push(StackItem::Element(child_element));
                    }
                }
            }
            StackItem::PopGroup => {
                if let Some((attrs, maybe_description)) = pending_bookmark.take() {
                    add_pending(&mut collection, &folder_stack, attrs, maybe_description, None)?;
                }
                folder_stack.pop();
            }
        }
    }

    assert!(pending_bookmark.is_none());

    Ok(collection)
}

pub fn to_html(collection: &Collection) -> Result<String, Error> {
    const TEMPLATE: &str = include_str!("html/netscape_bookmarks.jinja");
    let mut env = Environment::new();
    env.add_template("netscape", TEMPLATE)?;
    let entities = collection.entities();
    let template = env.get_template("netscape")?;
    let mut rendered = template.render(context! { entities })?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}
