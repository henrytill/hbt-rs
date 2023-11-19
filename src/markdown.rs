#[cfg(test)]
mod tests;

use std::io;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};
use time::{macros::format_description, Date};
use url::Url;

use crate::collection::{Collection, Entity, Id, Label};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
enum ErrorImpl {
    Io(String),
    UrlParse(url::ParseError),
    TimeParse(time::error::Parse),
    MissingName,
    MissingUrl,
    MissingDate,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]

pub struct Error {
    inner: Box<ErrorImpl>,
}

impl Error {
    fn new(inner: ErrorImpl) -> Self {
        let inner = Box::new(inner);
        Self { inner }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self.inner {
            ErrorImpl::Io(err) => write!(f, "IO error: {}", err),
            ErrorImpl::UrlParse(err) => write!(f, "URL parse error: {}", err),
            ErrorImpl::TimeParse(err) => write!(f, "Time parse error: {}", err),
            ErrorImpl::MissingName => write!(f, "Missing name"),
            ErrorImpl::MissingUrl => write!(f, "Missing URL"),
            ErrorImpl::MissingDate => write!(f, "Missing date"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::new(ErrorImpl::Io(err.to_string()))
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Self::new(ErrorImpl::UrlParse(err))
    }
}

impl From<time::error::Parse> for Error {
    fn from(err: time::error::Parse) -> Self {
        Self::new(ErrorImpl::TimeParse(err))
    }
}

pub fn parse(input: &str) -> Result<Collection, Error> {
    let parser = Parser::new(input);

    let date_format = format_description!("[month repr:long] [day padding:none], [year]");

    let mut ret = Collection::new();

    let mut name: Option<String> = None;
    let mut date: Option<Date> = None;
    let mut url: Option<Url> = None;
    let mut labels: Vec<Label> = Vec::new();

    let mut current_tag: Option<Tag> = None;
    let mut current_heading_level: HeadingLevel = HeadingLevel::H1;
    let mut maybe_parent: Option<Id> = None;
    let mut parents: Vec<Id> = Vec::new();

    for event in parser {
        match event {
            // Start
            Event::Start(tag @ Tag::Heading(HeadingLevel::H1, _, _)) => {
                assert_eq!(current_heading_level, HeadingLevel::H1);
                assert_eq!(labels.len(), 0);
                current_heading_level = HeadingLevel::H1;
                current_tag = Some(tag);
            }
            Event::Start(ref tag @ Tag::Heading(ref heading_level, _, _)) => {
                labels.truncate(*heading_level as usize - 2); // let's not do this
                current_heading_level = *heading_level;
                current_tag = Some(tag.to_owned());
            }
            Event::Start(tag @ Tag::List(_)) => {
                current_tag = Some(tag);
                if let Some(last_id) = maybe_parent {
                    parents.push(last_id);
                }
            }
            Event::Start(ref tag @ Tag::Link(_, ref link, ref title)) => {
                current_tag = Some(tag.to_owned());
                url = Some(Url::parse(link)?);
                assert!(title.is_empty());
            }
            Event::Start(tag) => {
                current_tag = Some(tag);
            }
            // Text
            Event::Text(text) => match (&current_tag, current_heading_level) {
                (Some(Tag::Heading(_, _, _)), HeadingLevel::H1) => {
                    date = Some(Date::parse(text.as_ref(), date_format)?);
                }
                (Some(Tag::Heading(_, _, _)), _) => {
                    let label = Label::new(text.to_string());
                    labels.push(label);
                }
                (Some(Tag::Link(_, _, _)), _) => {
                    name = Some(text.to_string());
                }
                _ => {}
            },
            // End
            Event::End(Tag::List(_)) => {
                let _ = parents.pop();
                maybe_parent = None;
            }
            Event::End(Tag::Link(_, _, _)) => {
                let name = name.take().ok_or(Error::new(ErrorImpl::MissingName))?;
                let url = url.take().ok_or(Error::new(ErrorImpl::MissingUrl))?;
                let date = date.ok_or(Error::new(ErrorImpl::MissingDate))?;
                let entity = Entity::new(Some(name), url, date, labels.clone());
                let id = ret.add_node(entity);
                if let Some(parent) = parents.last() {
                    ret.add_edge(*parent, id);
                    ret.add_edge(id, *parent);
                }
                maybe_parent = Some(id);
            }
            _ => {}
        }
    }

    Ok(ret)
}
