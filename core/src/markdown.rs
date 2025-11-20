use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pulldown_cmark::{Event, HeadingLevel, LinkType, Parser, Tag, TagEnd};
use thiserror::Error;

use crate::{
    collection::{Collection, Id},
    entity::{self, Entity, Label, Name, Url},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Entity(#[from] entity::Error),

    #[error("missing URL")]
    MissingUrl,

    #[error("missing date")]
    MissingDate,

    #[error("date parsing error: {0}, {1}")]
    ParseDate(#[source] chrono::ParseError, String),

    #[error("invalid time construction for date: {0}")]
    InvalidTime(String),
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

const DATE_FORMAT: &str = "%B %-d, %Y";

fn parse_date(s: &str) -> Result<DateTime<Utc>, Error> {
    let date = NaiveDate::parse_from_str(s, DATE_FORMAT)
        .map_err(|err| Error::ParseDate(err, s.to_string()))?;
    let datetime = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| Error::InvalidTime(s.to_string()))?;
    Ok(Utc.from_utc_datetime(&datetime))
}

struct ParserState<'a> {
    name: Option<Name>,
    name_parts: Vec<String>,
    date: Option<DateTime<Utc>>,
    url: Option<Url>,
    labels: Vec<Label>,
    current_tag: Option<Tag<'a>>,
    current_heading_level: HeadingLevel,
    maybe_parent: Option<Id>,
    parents: Vec<Id>,
}

impl<'a> ParserState<'a> {
    fn new() -> ParserState<'a> {
        ParserState {
            name: None,
            name_parts: Vec::new(),
            date: None,
            url: None,
            labels: Vec::new(),
            current_tag: None,
            current_heading_level: HeadingLevel::H1,
            maybe_parent: None,
            parents: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.name = None;
        self.name_parts.clear();
        self.date = None;
        self.url = None;
        self.labels.clear();
        self.current_heading_level = HeadingLevel::H1;
        self.maybe_parent = None;
        self.parents.clear();
    }

    fn save_entity(&mut self, coll: &mut Collection) -> Result<(), Error> {
        let url = self.url.take().ok_or(Error::MissingUrl)?;
        let date = self.date.ok_or(Error::MissingDate)?;
        let name = if self.name_parts.is_empty() {
            self.name.take()
        } else {
            Some(Name::new(self.name_parts.join("")))
        };
        self.name_parts.clear();
        let labels = self.labels.iter().cloned().collect();
        let entity = Entity::new(url, date.into(), name, labels);
        let id = coll.upsert(entity);
        if let Some(parent) = self.parents.last() {
            coll.add_edges(*parent, id);
        }
        self.maybe_parent = Some(id);
        Ok(())
    }
}

impl Collection {
    pub fn from_markdown(input: &str) -> Result<Collection, Error> {
        let parser = Parser::new(input);

        let mut coll = Collection::new();
        let mut state = ParserState::new();

        for event in parser {
            match event {
                // Start
                Event::Start(
                    tag @ Tag::Heading {
                        level: HeadingLevel::H1,
                        ..
                    },
                ) => {
                    state.reset();
                    state.current_tag = Some(tag);
                }
                Event::Start(tag @ Tag::Heading { level, .. }) => {
                    state.current_tag = Some(tag);
                    state.current_heading_level = level;
                    let level = usize::from(HeadingLevelExt::from(level));
                    state.labels.truncate(level - 2);
                }
                Event::Start(tag @ Tag::List(_)) => {
                    state.current_tag = Some(tag);
                    if let Some(parent) = state.maybe_parent {
                        state.parents.push(parent);
                    }
                }
                Event::Start(
                    ref tag @ Tag::Link {
                        link_type: LinkType::Inline,
                        ref dest_url,
                        ..
                    },
                ) => {
                    state.current_tag = Some(tag.to_owned());
                    state.name_parts.clear();
                    state.url = Some(Url::parse(dest_url)?);
                }
                Event::Start(
                    ref tag @ Tag::Link {
                        link_type: LinkType::Autolink,
                        ref dest_url,
                        ..
                    },
                ) => {
                    state.current_tag = Some(tag.to_owned());
                    state.name = None;
                    state.name_parts.clear();
                    state.url = Some(Url::parse(dest_url)?);
                }
                Event::Start(tag) => {
                    state.current_tag = Some(tag);
                }
                // Text
                Event::Text(text) => match (&state.current_tag, state.current_heading_level) {
                    (Some(Tag::Heading { .. }), HeadingLevel::H1) => {
                        let parsed = parse_date(text.as_ref())?;
                        state.date = Some(parsed);
                    }
                    (Some(Tag::Heading { .. }), _) => {
                        let label = Label::new(text.to_string());
                        state.labels.push(label);
                    }
                    (
                        Some(Tag::Link {
                            link_type: LinkType::Inline,
                            ..
                        }),
                        _,
                    ) => {
                        state.name_parts.push(text.to_string());
                    }
                    _ => {}
                },
                // Code (for handling backticks in link text)
                Event::Code(text) => {
                    if let Some(Tag::Link {
                        link_type: LinkType::Inline,
                        ..
                    }) = &state.current_tag
                    {
                        state.name_parts.push(format!("`{}`", text));
                    }
                }
                // End
                Event::End(TagEnd::List(_)) => {
                    let _ = state.parents.pop();
                    state.maybe_parent = None;
                }
                Event::End(TagEnd::Link) => {
                    state.save_entity(&mut coll)?;
                }
                _ => {}
            }
        }

        Ok(coll)
    }
}
