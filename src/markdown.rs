#[cfg(test)]
mod tests;

use std::io::{self, Read};

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};
use time::{macros::format_description, Date};
use url::Url;

use crate::collection::{Entity, Label};

#[derive(Debug)]
enum ErrorImpl {
    Io(io::Error),
    UrlParse(url::ParseError),
    TimeParse(time::error::Parse),
}

#[derive(Debug)]
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
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::new(ErrorImpl::Io(err))
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

pub fn parse(reader: &mut impl Read) -> Result<Vec<Entity>, Error> {
    let mut buffer = String::new();
    let _bytes_read = reader.read_to_string(&mut buffer)?;
    let parser = Parser::new(&buffer);

    let date_format = format_description!("[month repr:long] [day padding:none], [year]");

    let mut ret = Vec::new();
    let mut name: Option<String> = None;
    let mut date: Option<Date> = None;
    let mut url: Option<Url> = None;
    let mut labels: Vec<Label> = Vec::new();

    let mut curr_tag: Option<Tag> = None;
    let mut curr_heading_level: HeadingLevel = HeadingLevel::H1;
    let mut _curr_link_level: usize = 0;

    for event in parser {
        match event {
            // Start
            Event::Start(tag @ Tag::Heading(HeadingLevel::H1, _, _)) => {
                assert_eq!(curr_heading_level, HeadingLevel::H1);
                assert_eq!(labels.len(), 0);
                curr_heading_level = HeadingLevel::H1;
                curr_tag = Some(tag);
            }
            Event::Start(ref tag @ Tag::Heading(ref heading_level, _, _)) => {
                labels.truncate(*heading_level as usize - 2); // let's not do this
                curr_heading_level = *heading_level;
                curr_tag = Some(tag.to_owned());
            }
            Event::Start(tag @ Tag::List(_)) => {
                _curr_link_level += 1;
                curr_tag = Some(tag);
            }
            Event::Start(ref tag @ Tag::Link(_, ref link, ref title)) => {
                curr_tag = Some(tag.to_owned());
                url = Some(Url::parse(link)?);
                assert!(title.is_empty());
            }
            Event::Start(tag) => {
                curr_tag = Some(tag);
            }
            // Text
            Event::Text(text) => match (&curr_tag, curr_heading_level) {
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
                _curr_link_level -= 1;
            }
            Event::End(Tag::Link(_, _, _)) => {
                let entity = Entity::new(
                    name.take().unwrap(),
                    url.take().unwrap(),
                    date.unwrap().clone(),
                    Vec::new(),
                    labels.clone(),
                );
                ret.push(entity);
            }
            _ => {}
        }
    }

    Ok(ret)
}
