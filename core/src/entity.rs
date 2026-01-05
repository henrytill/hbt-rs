use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use hbt_pinboard::Post;

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing URL")]
    MissingUrl,

    #[error("URL parsing error: {0}, {1}")]
    ParseUrl(#[source] url::ParseError, String),

    #[error("integer parsing error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("timestamp parsing error: {1}, {1}")]
    ParseTimestamp(i64, String),

    #[error("chrono parsing error: {0}, {1}")]
    Chrono(#[source] chrono::ParseError, String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[schemars(transparent)]
pub struct Url(url::Url);

impl Url {
    /// Parses a string into a URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid URL according to the URL specification.
    pub fn parse(s: &str) -> Result<Url, Error> {
        url::Url::parse(s)
            .map(Url)
            .map_err(|err| Error::ParseUrl(err, s.to_string()))
    }
}

impl Hash for Url {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct Name(String);

impl Name {
    #[must_use]
    pub const fn new(name: String) -> Name {
        Name(name)
    }

    #[must_use]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct Label(String);

impl Label {
    #[must_use]
    pub const fn new(label: String) -> Label {
        Label(label)
    }

    #[must_use]
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

impl From<&String> for Label {
    fn from(label: &String) -> Label {
        Label(label.to_owned())
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
    #[must_use]
    pub const fn new(time: DateTime<Utc>) -> Time {
        Time(time)
    }

    /// Parses a Unix timestamp string into a `Time`.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid integer or the timestamp is out of range.
    pub fn parse_timestamp(time: &str) -> Result<Time, Error> {
        let timestamp: i64 = time.parse()?;
        let time = DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| Error::ParseTimestamp(timestamp, time.to_string()))?;
        Ok(Time(time))
    }

    fn parse_iso8601(time: &str) -> Result<Time, Error> {
        let time = DateTime::parse_from_rfc3339(time)
            .map_err(|err| Error::Chrono(err, time.to_string()))?
            .with_timezone(&Utc);
        Ok(Time(time))
    }

    /// Parses a time string that could be either a Unix timestamp or ISO 8601 format.
    ///
    /// Tries Unix timestamp first, falls back to ISO 8601 if that fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed as either a Unix timestamp or ISO 8601 date.
    pub fn parse_flexible(time: &str) -> Result<Time, Error> {
        match Time::parse_timestamp(time.trim()) {
            Ok(time) => return Ok(time),
            Err(Error::ParseInt(_)) => (),
            err => return err,
        }
        Time::parse_iso8601(time.trim())
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct Extended(String);

impl Extended {
    #[must_use]
    pub const fn new(extended: String) -> Extended {
        Extended(extended)
    }

    #[must_use]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    #[serde(rename = "uri")]
    url: Url,
    created_at: Time,
    updated_at: Vec<Time>,
    names: BTreeSet<Name>,
    labels: BTreeSet<Label>,
    shared: Option<bool>,
    to_read: Option<bool>,
    is_feed: Option<bool>,
    #[serde(default)]
    extended: Vec<Extended>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_visited_at: Option<Time>,
}

impl Entity {
    #[must_use]
    pub fn new(
        url: Url,
        created_at: Time,
        maybe_name: Option<Name>,
        labels: BTreeSet<Label>,
    ) -> Entity {
        Entity {
            url,
            created_at,
            updated_at: Vec::new(),
            names: maybe_name.into_iter().collect(),
            labels,
            shared: None,
            to_read: None,
            extended: Vec::new(),
            last_visited_at: None,
            is_feed: None,
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

    #[must_use]
    pub fn url(&self) -> &Url {
        &self.url
    }

    #[must_use]
    pub fn labels(&self) -> &BTreeSet<Label> {
        &self.labels
    }

    pub fn labels_mut(&mut self) -> &mut BTreeSet<Label> {
        &mut self.labels
    }
}

impl TryFrom<Post> for Entity {
    type Error = Error;

    fn try_from(post: Post) -> Result<Self, Self::Error> {
        let url = Url::parse(&post.href)?;
        let created_at = Time::parse_flexible(&post.time)?;
        let extended: Vec<Extended> = post.extended.map(Extended::new).into_iter().collect();

        Ok(Entity {
            url,
            created_at,
            updated_at: Vec::new(),
            names: post.description.into_iter().map(Name::new).collect(),
            labels: post.tags.into_iter().map(Label::new).collect(),
            shared: Some(post.shared),
            to_read: Some(post.toread),
            extended,
            last_visited_at: None,
            is_feed: Some(false),
        })
    }
}

pub mod html {
    use super::{Entity, Error, Extended, Label, Name, Time, Url};
    use std::collections::{BTreeSet, HashMap};

    const KEY_HREF: &str = "href";
    const KEY_ADD_DATE: &str = "add_date";
    const KEY_LAST_MODIFIED: &str = "last_modified";
    const KEY_LAST_VISIT: &str = "last_visit";
    const KEY_TAGS: &str = "tags";
    const KEY_PRIVATE: &str = "private";
    const KEY_TOREAD: &str = "toread";
    const KEY_FEED: &str = "feed";

    impl Entity {
        /// Creates an entity from HTML bookmark attributes.
        ///
        /// # Errors
        ///
        /// Returns an error if required attributes are missing (e.g., `href`) or if values cannot be parsed
        /// (e.g., invalid URL, invalid timestamp).
        pub fn from_attrs(
            attrs: HashMap<String, String>,
            names: BTreeSet<Name>,
            labels: BTreeSet<Label>,
            extended: Vec<Extended>,
        ) -> Result<Entity, Error> {
            let href = attrs.get(KEY_HREF).ok_or(Error::MissingUrl)?;
            let url = Url::parse(href)?;

            let mut entity = Entity {
                url,
                created_at: Time::default(),
                updated_at: Vec::new(),
                names,
                labels,
                shared: None,
                to_read: None,
                is_feed: None,
                extended,
                last_visited_at: None,
            };

            let mut tags = String::new();

            for (key, value) in attrs {
                let trimmed = value.trim();
                match key.to_lowercase().as_str() {
                    KEY_ADD_DATE if !trimmed.is_empty() => {
                        entity.created_at = Time::parse_timestamp(trimmed)?;
                    }
                    KEY_LAST_MODIFIED if !trimmed.is_empty() => {
                        let time = Time::parse_timestamp(trimmed)?;
                        entity.updated_at.push(time);
                    }
                    KEY_LAST_VISIT if !trimmed.is_empty() => {
                        let time = Time::parse_timestamp(trimmed)?;
                        entity.last_visited_at = Some(time);
                    }
                    KEY_TAGS if !trimmed.is_empty() => {
                        tags = value;
                    }
                    KEY_PRIVATE => {
                        entity.shared = Some(trimmed != "1");
                    }
                    KEY_TOREAD => {
                        entity.to_read = Some(trimmed == "1");
                    }
                    KEY_FEED => {
                        entity.is_feed = Some(trimmed == "true");
                    }
                    _ => {}
                }
            }

            for tag in tags.split(',') {
                let s = tag.trim();
                if s.is_empty() {
                    continue;
                }
                if s == "toread" {
                    entity.to_read = Some(true);
                    continue;
                }
                entity.labels.insert(Label::from(s));
            }

            Ok(entity)
        }
    }
}
