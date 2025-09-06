use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::pinboard::Post;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("URL parsing error: {0}")]
    ParseUrl(#[from] url::ParseError),

    #[error("integer parsing error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("time parsing error: {0}")]
    ParseTime(i64),

    #[error("time format parsing error: {0}")]
    ParseTimeFormat(#[from] chrono::ParseError),
}

/// A [`Name`] describes an [`Entity`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct Name(String);

impl Name {
    pub const fn new(name: String) -> Name {
        Name(name)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<String> for Name {
    fn from(name: String) -> Name {
        Name(name)
    }
}

#[cfg(test)]
impl From<&str> for Name {
    fn from(name: &str) -> Name {
        Name(name.into())
    }
}

/// A [`Label`] is text that can be attached to an [`Entity`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct Label(String);

impl Label {
    pub const fn new(label: String) -> Label {
        Label(label)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Hash for Label {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<String> for Label {
    fn from(label: String) -> Label {
        Label(label)
    }
}

impl From<&str> for Label {
    fn from(label: &str) -> Label {
        Label(label.into())
    }
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct Time(
    #[serde(with = "chrono::serde::ts_seconds")]
    #[schemars(with = "i64")]
    DateTime<Utc>,
);

impl Time {
    pub const fn new(time: DateTime<Utc>) -> Time {
        Time(time)
    }

    pub fn parse(time: &str) -> Result<Time, Error> {
        let timestamp: i64 = time.parse()?;
        let time = DateTime::from_timestamp(timestamp, 0).ok_or(Error::ParseTime(timestamp))?;
        Ok(Time(time))
    }

    fn parse_iso8601(time: &str) -> Result<Time, Error> {
        let time = DateTime::parse_from_rfc3339(time)?.with_timezone(&Utc);
        Ok(Time(time))
    }

    pub fn parse_flexible(time: &str) -> Result<Time, Error> {
        // Try Unix timestamp first (all digits, possibly with leading/trailing whitespace)
        if time.trim().chars().all(|c| c.is_ascii_digit()) {
            return Self::parse(time.trim());
        }
        // Try ISO 8601 format
        Self::parse_iso8601(time.trim())
    }
}

impl From<DateTime<Utc>> for Time {
    fn from(time: DateTime<Utc>) -> Time {
        Time(time)
    }
}

impl Default for Time {
    fn default() -> Time {
        Time(DateTime::UNIX_EPOCH)
    }
}

/// A [`Extended`] is text that can be attached to an [`Entity`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct Extended(String);

impl Extended {
    pub const fn new(extended: String) -> Extended {
        Extended(extended)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Hash for Extended {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<String> for Extended {
    fn from(extended: String) -> Extended {
        Extended(extended)
    }
}

#[cfg(test)]
impl From<&str> for Extended {
    fn from(extended: &str) -> Extended {
        Extended(extended.into())
    }
}

/// An [`Entity`] is a page in the collection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    #[serde(rename = "uri")]
    url: Url,
    created_at: Time,
    updated_at: Vec<Time>,
    names: BTreeSet<Name>,
    labels: BTreeSet<Label>,
    shared: bool,
    to_read: bool,
    is_feed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended: Option<Extended>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_visited_at: Option<Time>,
}

impl Entity {
    pub fn new(
        url: Url,
        created_at: Time,
        maybe_name: Option<Name>,
        labels: BTreeSet<Label>,
    ) -> Entity {
        let updated_at = Vec::new();
        let names = maybe_name.into_iter().collect();
        let extended = None;
        let shared = false;
        let to_read = false;
        let last_visited_at = None;
        let is_feed = false;
        Entity {
            url,
            created_at,
            updated_at,
            names,
            labels,
            extended,
            shared,
            to_read,
            last_visited_at,
            is_feed,
        }
    }

    pub fn update(
        &mut self,
        updated_at: Time,
        names: BTreeSet<Name>,
        labels: BTreeSet<Label>,
    ) -> &mut Entity {
        if updated_at < self.created_at {
            self.updated_at.push(self.created_at);
            self.created_at = updated_at;
        } else {
            self.updated_at.push(updated_at);
        }
        // Sort updated_at to maintain chronological order
        self.updated_at.sort();
        self.names.extend(names);
        self.labels.extend(labels);
        self
    }

    pub fn merge(&mut self, other: Entity) -> &mut Entity {
        self.update(other.created_at, other.names, other.labels)
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn created_at(&self) -> &Time {
        &self.created_at
    }

    pub fn updated_at(&self) -> &[Time] {
        &self.updated_at
    }

    pub fn names(&self) -> &BTreeSet<Name> {
        &self.names
    }

    pub fn labels(&self) -> &BTreeSet<Label> {
        &self.labels
    }

    pub fn labels_mut(&mut self) -> &mut BTreeSet<Label> {
        &mut self.labels
    }

    pub fn last_visited_at(&self) -> Option<&Time> {
        self.last_visited_at.as_ref()
    }

    pub fn is_feed(&self) -> bool {
        self.is_feed
    }

    pub fn shared(&self) -> bool {
        self.shared
    }

    pub fn to_read(&self) -> bool {
        self.to_read
    }

    pub fn extended(&self) -> Option<&Extended> {
        self.extended.as_ref()
    }
}

impl TryFrom<Post> for Entity {
    type Error = Error;

    fn try_from(post: Post) -> Result<Entity, Self::Error> {
        let url = Url::parse(&post.href)?;
        let created_at = Time::parse_flexible(&post.time)?;
        let updated_at: Vec<Time> = Vec::new();
        let names = post.description.into_iter().map(Name::new).collect();
        let labels = post.tags.into_iter().map(Label::new).collect();
        let extended = post.extended.map(Extended::new);
        let shared = post.shared;
        let to_read = post.toread;
        let last_visited_at = None;
        let is_feed = false;
        Ok(Entity {
            url,
            created_at,
            updated_at,
            names,
            labels,
            extended,
            shared,
            to_read,
            last_visited_at,
            is_feed,
        })
    }
}

pub mod html {
    use super::{Entity, Error, Extended, Label, Name, Time};
    use std::collections::{BTreeSet, HashMap};
    use url::Url;

    pub type Attributes = HashMap<String, String>;

    fn parse_timestamp(value: &str) -> Result<Time, Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() { Ok(Time::default()) } else { Time::parse(trimmed) }
    }

    fn parse_timestamp_opt(value: &str) -> Result<Option<Time>, Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() { Ok(None) } else { Ok(Some(Time::parse(trimmed)?)) }
    }

    impl Entity {
        pub fn from_html_attributes(
            attrs: &Attributes,
            names: BTreeSet<Name>,
            labels: BTreeSet<Label>,
            extended: Option<Extended>,
        ) -> Result<Entity, Error> {
            const KEY_HREF: &str = "href";
            const KEY_ADD_DATE: &str = "add_date";
            const KEY_LAST_MODIFIED: &str = "last_modified";
            const KEY_LAST_VISIT: &str = "last_visit";
            const KEY_TAGS: &str = "tags";
            const KEY_PRIVATE: &str = "private";
            const KEY_TOREAD: &str = "toread";
            const KEY_FEED: &str = "feed";

            let href = attrs.get(KEY_HREF).ok_or(Error::ParseUrl(url::ParseError::EmptyHost))?;
            let url = Url::parse(href)?;

            let mut entity = Entity {
                url,
                created_at: Time::default(),
                updated_at: Vec::new(),
                names,
                labels,
                extended,
                shared: true, // Default to shared
                to_read: false,
                last_visited_at: None,
                is_feed: false,
            };

            let mut tag_string = String::new();

            for (key, value) in attrs {
                match key.to_lowercase().as_str() {
                    KEY_ADD_DATE => entity.created_at = parse_timestamp(value)?,
                    KEY_LAST_MODIFIED if !value.is_empty() => {
                        if let Some(time) = parse_timestamp_opt(value)? {
                            entity.updated_at = vec![time];
                        }
                    }
                    KEY_LAST_VISIT if !value.is_empty() => {
                        entity.last_visited_at = parse_timestamp_opt(value)?;
                    }
                    KEY_TAGS if !value.is_empty() => {
                        tag_string = value.clone();
                    }
                    KEY_PRIVATE => entity.shared = value != "1",
                    KEY_TOREAD => entity.to_read = value == "1",
                    KEY_FEED => entity.is_feed = value == "true",
                    _ => {}
                }
            }

            if !tag_string.is_empty() {
                const VALUE_TOREAD: &str = "toread";

                let tags: Vec<String> =
                    tag_string.split(',').map(|s| s.trim().to_string()).collect();
                let filtered_tags: Vec<String> =
                    tags.iter().filter(|&tag| tag != VALUE_TOREAD).cloned().collect();
                let tag_labels: BTreeSet<Label> =
                    filtered_tags.into_iter().map(Label::from).collect();
                entity.labels.extend(tag_labels);
                entity.to_read = entity.to_read || tags.contains(&VALUE_TOREAD.to_string());
            }

            Ok(entity)
        }
    }
}
