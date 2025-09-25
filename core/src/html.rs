use std::{
    collections::{BTreeSet, HashMap},
    io::{self, Write},
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

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
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

type Attrs = HashMap<String, String>;

fn add(
    coll: &mut Collection,
    attrs: Attrs,
    folders: impl IntoIterator<Item = impl Into<Label>>,
    maybe_name: Option<impl Into<Name>>,
    maybe_ext: Option<impl Into<Extended>>,
) -> Result<(), Error> {
    let names = maybe_name.into_iter().map(Into::into).collect();
    let labels: BTreeSet<Label> = folders.into_iter().map(Into::into).collect();
    let maybe_ext = maybe_ext.map(Into::into);
    let entity = Entity::from_attrs(attrs, names, labels, maybe_ext)?;
    coll.upsert(entity);
    Ok(())
}

fn extract_text(elt: ElementRef) -> Option<String> {
    let trimmed = elt.text().collect::<String>().trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

fn extract_attrs(elt: ElementRef) -> Attrs {
    let mut attrs = HashMap::new();
    for (name, value) in elt.value().attrs() {
        attrs.insert(name.to_lowercase(), value.to_string());
    }
    attrs
}

pub fn from_html(html: &str) -> Result<Collection, Error> {
    let document = Html::parse_document(html);
    let root = document.root_element();

    let mut coll = Collection::new();
    let mut stack: Vec<StackItem> = Vec::new();
    let mut folders: Vec<String> = Vec::new();
    let mut pending: Option<(Attrs, Option<String>)> = None;

    const A: &str = "a";
    const H3: &str = "h3";
    const DT: &str = "dt";
    const DD: &str = "dd";
    const DL: &str = "dl";

    let a_selector = Selector::parse(A)?;
    let h3_selector = Selector::parse(H3)?;

    for child in root.children().rev() {
        if let Some(child_elt) = ElementRef::wrap(child) {
            stack.push(StackItem::Element(child_elt));
        }
    }

    while let Some(item) = stack.pop() {
        match item {
            StackItem::Element(elt) => {
                match elt.value().name() {
                    DT => {
                        if let Some((attrs, maybe_desc)) = pending.take() {
                            add(&mut coll, attrs, &folders, maybe_desc, None::<String>)?;
                        }

                        if let Some(h3_elt) = elt.select(&h3_selector).next() {
                            if let Some(folder) = extract_text(h3_elt) {
                                folders.push(folder);
                            }
                        } else if let Some(a_elt) = elt.select(&a_selector).next() {
                            let attrs = extract_attrs(a_elt);
                            let maybe_desc = extract_text(a_elt);
                            pending = Some((attrs, maybe_desc));
                        }
                    }
                    DD => {
                        if let Some((attrs, maybe_desc)) = pending.take() {
                            let maybe_ext = extract_text(elt);
                            add(&mut coll, attrs, &folders, maybe_desc, maybe_ext)?;
                        }
                    }
                    DL => {
                        stack.push(StackItem::PopGroup);
                    }
                    _ => {}
                }
                for child in elt.children().rev() {
                    if let Some(child_elt) = ElementRef::wrap(child) {
                        stack.push(StackItem::Element(child_elt));
                    }
                }
            }
            StackItem::PopGroup => {
                if let Some((attrs, maybe_desc)) = pending.take() {
                    add(&mut coll, attrs, &folders, maybe_desc, None::<String>)?;
                }
                folders.pop();
            }
        }
    }

    assert!(pending.is_none());

    Ok(coll)
}

pub fn to_html(mut writer: impl Write, coll: &Collection) -> Result<(), Error> {
    const TEMPLATE: &str = include_str!("html/netscape_bookmarks.jinja");
    let mut env = Environment::new();
    env.add_template("netscape", TEMPLATE)?;
    let entities = coll.entities();
    let template = env.get_template("netscape")?;
    template.render_to_write(context! { entities }, &mut writer)?;
    writer.write_all(b"\n")?;
    Ok(())
}
