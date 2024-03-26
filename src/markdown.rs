#[cfg(test)]
mod tests;

use std::{io, fmt};

use pulldown_cmark::{Event, HeadingLevel, LinkType, Parser, Tag};
use time::{macros::format_description, Date};
use url::Url;

use crate::collection::{Collection, Entity, Id, Label, Name};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
enum ErrorImpl {
    Io(String),
    UrlParse(url::ParseError),
    TimeParse(time::error::Parse, String),
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.inner.as_ref() {
            ErrorImpl::Io(err) => write!(f, "IO error: {}", err),
            ErrorImpl::UrlParse(err) => write!(f, "URL parse error: {}", err),
            ErrorImpl::TimeParse(err, str) => write!(f, "Time parse error: {}: {}", err, str),
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

impl std::error::Error for Error {}

struct HeadingLevelExt(HeadingLevel);

impl From<HeadingLevel> for HeadingLevelExt {
    fn from(level: HeadingLevel) -> Self {
        Self(level)
    }
}

impl From<HeadingLevelExt> for usize {
    fn from(level: HeadingLevelExt) -> Self {
        match level.0 {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        }
    }
}

pub fn parse(input: &str) -> Result<Collection, Error> {
    let parser = Parser::new(input);

    let date_format = format_description!("[month repr:long] [day padding:none], [year]");

    let mut ret = Collection::new();

    let mut name: Option<Name> = None;
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
                name = None;
                date = None;
                url = None;
                labels.clear();
                current_tag = Some(tag);
                current_heading_level = HeadingLevel::H1;
                maybe_parent = None;
                parents.clear();
            }
            Event::Start(ref tag @ Tag::Heading(heading_level, _, _)) => {
                current_tag = Some(tag.to_owned());
                current_heading_level = heading_level;
                let level = usize::from(HeadingLevelExt::from(heading_level));
                labels.truncate(level - 2);
            }
            Event::Start(tag @ Tag::List(_)) => {
                current_tag = Some(tag);
                if let Some(last_id) = maybe_parent {
                    parents.push(last_id);
                }
            }
            Event::Start(ref tag @ Tag::Link(LinkType::Inline, ref link, ref title)) => {
                current_tag = Some(tag.to_owned());
                url = Some(Url::parse(link)?);
                assert!(title.is_empty());
            }
            Event::Start(ref tag @ Tag::Link(LinkType::Autolink, ref link, ref title)) => {
                current_tag = Some(tag.to_owned());
                name = None;
                url = Some(Url::parse(link)?);
                assert!(title.is_empty());
            }
            Event::Start(tag) => {
                current_tag = Some(tag);
            }
            // Text
            Event::Text(text) => match (&current_tag, current_heading_level) {
                (Some(Tag::Heading(_, _, _)), HeadingLevel::H1) => {
                    let parsed = Date::parse(text.as_ref(), date_format)
                        .map_err(|err| Error::new(ErrorImpl::TimeParse(err, text.to_string())))?;
                    date = Some(parsed);
                }
                (Some(Tag::Heading(_, _, _)), _) => {
                    let label = Label::new(text.to_string());
                    labels.push(label);
                }
                (Some(Tag::Link(LinkType::Inline, _, _)), _) => {
                    name = Some(Name::new(text.to_string()));
                }
                _ => {}
            },
            // End
            Event::End(Tag::List(_)) => {
                let _ = parents.pop();
                maybe_parent = None;
            }
            Event::End(Tag::Link(_, _, _)) => {
                let url = url.take().ok_or(Error::new(ErrorImpl::MissingUrl))?;
                let date = date.ok_or(Error::new(ErrorImpl::MissingDate))?;
                let name = name.take();
                let labels = labels.iter().cloned().collect();
                let entity = Entity::new(url, date, name, labels);
                let id = ret.upsert(entity);
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
