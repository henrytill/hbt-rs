use std::{
    collections::{BTreeSet, HashMap},
    num,
};

use minijinja::{Environment, context};
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;

use crate::{
    collection::Collection,
    entity::{self, Entity, Extended, Label, Name},
};

#[derive(Debug, Error)]
pub enum Error {
    // Entity-related variants
    #[error("URL parsing error: {0}")]
    ParseUrl(#[from] url::ParseError),

    #[error("integer parsing error: {0}")]
    ParseInt(#[from] num::ParseIntError),

    #[error("time parsing error: {0}")]
    ParseTime(i64),

    #[error("time format parsing error: {0}")]
    ParseTimeFormat(#[from] chrono::ParseError),

    // Local variants
    #[error("HTML selector error: {0}")]
    HtmlSelector(String),

    #[error("HTML missing required attribute: {0}")]
    HtmlAttribute(String),

    #[error("Template error: {0}")]
    Template(#[from] minijinja::Error),
}

impl From<entity::Error> for Error {
    fn from(err: entity::Error) -> Self {
        match err {
            entity::Error::ParseUrl(e) => Error::ParseUrl(e),
            entity::Error::ParseInt(e) => Error::ParseInt(e),
            entity::Error::ParseTime(t) => Error::ParseTime(t),
            entity::Error::ParseTimeFormat(e) => Error::ParseTimeFormat(e),
        }
    }
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

fn add_pending(
    collection: &mut Collection,
    attrs: Attributes,
    folders: impl IntoIterator<Item = impl Into<Label>>,
    maybe_name: Option<impl Into<Name>>,
    maybe_extended: Option<impl Into<Extended>>,
) -> Result<(), Error> {
    let names = maybe_name.into_iter().map(Into::into).collect();
    let labels: BTreeSet<Label> = folders.into_iter().map(Into::into).collect();
    let maybe_extended = maybe_extended.map(Into::into);
    let entity = Entity::from_attrs(attrs, names, labels, maybe_extended)?;
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
                                attrs,
                                &folder_stack,
                                maybe_description,
                                None::<String>, // No extended
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
                                attrs,
                                &folder_stack,
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
                    add_pending(
                        &mut collection,
                        attrs,
                        &folder_stack,
                        maybe_description,
                        None::<String>,
                    )?;
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
