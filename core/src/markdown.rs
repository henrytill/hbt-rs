use chrono::{NaiveDate, TimeZone, Utc};
use pulldown_cmark::{Event, HeadingLevel, LinkType, Parser, Tag, TagEnd};
use thiserror::Error;
use url::Url;

use crate::{
    collection::{Collection, Id},
    entity::{Entity, Label, Name},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing URL")]
    MissingUrl,

    #[error("missing date")]
    MissingDate,

    #[error("URL parsing error: {0}, {1}")]
    ParseUrl(#[source] url::ParseError, String),

    #[error("date parsing error: {0}, {1}")]
    ParseDate(#[source] chrono::ParseError, String),
}

#[derive(Copy, Clone)]
struct HeadingLevelExt(HeadingLevel);

impl From<HeadingLevel> for HeadingLevelExt {
    fn from(level: HeadingLevel) -> HeadingLevelExt {
        HeadingLevelExt(level)
    }
}

impl From<HeadingLevelExt> for usize {
    fn from(level: HeadingLevelExt) -> usize {
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

    let date_format = "%B %-d, %Y";

    let mut ret = Collection::new();

    let mut name: Option<Name> = None;
    let mut name_parts: Vec<String> = Vec::new();
    let mut date: Option<NaiveDate> = None;
    let mut url: Option<Url> = None;
    let mut labels: Vec<Label> = Vec::new();

    let mut current_tag: Option<Tag> = None;
    let mut current_heading_level: HeadingLevel = HeadingLevel::H1;
    let mut maybe_parent: Option<Id> = None;
    let mut parents: Vec<Id> = Vec::new();

    for event in parser {
        match event {
            // Start
            Event::Start(tag @ Tag::Heading { level: HeadingLevel::H1, .. }) => {
                name = None;
                name_parts.clear();
                date = None;
                url = None;
                labels.clear();
                current_tag = Some(tag);
                current_heading_level = HeadingLevel::H1;
                maybe_parent = None;
                parents.clear();
            }
            Event::Start(tag @ Tag::Heading { level, .. }) => {
                current_tag = Some(tag);
                current_heading_level = level;
                let level = usize::from(HeadingLevelExt::from(level));
                labels.truncate(level - 2);
            }
            Event::Start(tag @ Tag::List(_)) => {
                current_tag = Some(tag);
                if let Some(last_id) = maybe_parent {
                    parents.push(last_id);
                }
            }
            Event::Start(
                ref tag @ Tag::Link { link_type: LinkType::Inline, ref dest_url, ref title, .. },
            ) => {
                current_tag = Some(tag.to_owned());
                name_parts.clear();
                let parsed =
                    Url::parse(dest_url).map_err(|e| Error::ParseUrl(e, dest_url.to_string()))?;
                url = Some(parsed);
                assert!(title.is_empty());
            }
            Event::Start(
                ref tag @ Tag::Link {
                    link_type: LinkType::Autolink, ref dest_url, ref title, ..
                },
            ) => {
                current_tag = Some(tag.to_owned());
                name = None;
                name_parts.clear();
                let parsed =
                    Url::parse(dest_url).map_err(|e| Error::ParseUrl(e, dest_url.to_string()))?;
                url = Some(parsed);
                assert!(title.is_empty());
            }
            Event::Start(tag) => {
                current_tag = Some(tag);
            }
            // Text
            Event::Text(text) => match (&current_tag, current_heading_level) {
                (Some(Tag::Heading { .. }), HeadingLevel::H1) => {
                    let parsed = NaiveDate::parse_from_str(text.as_ref(), date_format)
                        .map_err(|err| Error::ParseDate(err, text.to_string()))?;
                    date = Some(parsed);
                }
                (Some(Tag::Heading { .. }), _) => {
                    let label = Label::new(text.to_string());
                    labels.push(label);
                }
                (Some(Tag::Link { link_type: LinkType::Inline, .. }), _) => {
                    name_parts.push(text.to_string());
                }
                _ => {}
            },
            // Code (for handling backticks in link text)
            Event::Code(text) => {
                if let Some(Tag::Link { link_type: LinkType::Inline, .. }) = &current_tag {
                    name_parts.push(format!("`{}`", text));
                }
            }
            // End
            Event::End(TagEnd::List(_)) => {
                let _ = parents.pop();
                maybe_parent = None;
            }
            Event::End(TagEnd::Link) => {
                let url = url.take().ok_or(Error::MissingUrl)?;
                let date = date.ok_or(Error::MissingDate)?;
                let datetime = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
                let name = if name_parts.is_empty() {
                    name.take()
                } else {
                    Some(Name::new(name_parts.join("")))
                };
                name_parts.clear();
                let labels = labels.iter().cloned().collect();
                let entity = Entity::new(url, datetime.into(), name, labels);
                let id = ret.upsert(entity);
                if let Some(parent) = parents.last() {
                    ret.add_edges(*parent, id);
                }
                maybe_parent = Some(id);
            }
            _ => {}
        }
    }

    Ok(ret)
}
