#[cfg(test)]
mod tests;

use anyhow::Error;
use pulldown_cmark::{Event, HeadingLevel, LinkType, Parser, Tag, TagEnd};
use time::{macros::format_description, Date};
use url::Url;

use crate::collection::{Collection, Entity, Id, Label, Name};

const MSG_MISSING_URL: &str = "missing URL";
const MSG_MISSING_DATE: &str = "missing date";

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
            Event::Start(tag @ Tag::Heading { level: HeadingLevel::H1, .. }) => {
                name = None;
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
                url = Some(Url::parse(dest_url)?);
                assert!(title.is_empty());
            }
            Event::Start(
                ref tag @ Tag::Link {
                    link_type: LinkType::Autolink, ref dest_url, ref title, ..
                },
            ) => {
                current_tag = Some(tag.to_owned());
                name = None;
                url = Some(Url::parse(dest_url)?);
                assert!(title.is_empty());
            }
            Event::Start(tag) => {
                current_tag = Some(tag);
            }
            // Text
            Event::Text(text) => match (&current_tag, current_heading_level) {
                (Some(Tag::Heading { .. }), HeadingLevel::H1) => {
                    let parsed = Date::parse(text.as_ref(), date_format).map_err(|err| {
                        Error::msg(format!("Time parse error: {}, {}", err, text))
                    })?;
                    date = Some(parsed);
                }
                (Some(Tag::Heading { .. }), _) => {
                    let label = Label::new(text.to_string());
                    labels.push(label);
                }
                (Some(Tag::Link { link_type: LinkType::Inline, .. }), _) => {
                    name = Some(Name::new(text.to_string()));
                }
                _ => {}
            },
            // End
            Event::End(TagEnd::List(_)) => {
                let _ = parents.pop();
                maybe_parent = None;
            }
            Event::End(TagEnd::Link) => {
                let url = url.take().ok_or_else(|| Error::msg(MSG_MISSING_URL))?;
                let date = date.ok_or_else(|| Error::msg(MSG_MISSING_DATE))?;
                let name = name.take();
                let labels = labels.iter().cloned().collect();
                let entity = Entity::new(url, date, name, labels);
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
