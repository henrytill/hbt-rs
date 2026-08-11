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

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
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

    /// Returns the time as a Unix timestamp, the form used on the wire.
    #[must_use]
    pub const fn timestamp(self) -> i64 {
        self.0.timestamp()
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

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct CreatedAt(Time);

impl CreatedAt {
    #[must_use]
    pub fn new(time: Time) -> CreatedAt {
        CreatedAt(time)
    }

    #[must_use]
    pub fn get(self) -> Time {
        self.0
    }
}

impl From<Time> for CreatedAt {
    fn from(time: Time) -> CreatedAt {
        CreatedAt::new(time)
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct UpdatedAt(Time);

impl UpdatedAt {
    #[must_use]
    pub fn new(time: Time) -> UpdatedAt {
        UpdatedAt(time)
    }

    #[must_use]
    pub fn get(self) -> Time {
        self.0
    }
}

impl From<Time> for UpdatedAt {
    fn from(time: Time) -> UpdatedAt {
        UpdatedAt::new(time)
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

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct Flag(Option<bool>);

impl Flag {
    #[must_use]
    pub const fn new(value: bool) -> Flag {
        Flag(Some(value))
    }

    #[must_use]
    pub const fn get(self) -> Option<bool> {
        self.0
    }

    #[must_use]
    pub const fn merge(self, other: Flag) -> Flag {
        match (self.0, other.0) {
            (None, None) => Flag(None),
            (Some(x), None) | (None, Some(x)) => Flag(Some(x)),
            (Some(x), Some(y)) => Flag(Some(x || y)),
        }
    }
}

impl From<bool> for Flag {
    fn from(value: bool) -> Flag {
        Flag::new(value)
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct Shared(Flag);

impl Shared {
    #[must_use]
    pub const fn new(value: bool) -> Shared {
        Shared(Flag::new(value))
    }

    #[must_use]
    pub const fn get(self) -> Option<bool> {
        self.0.get()
    }

    #[must_use]
    pub const fn merge(self, other: Shared) -> Shared {
        Shared(self.0.merge(other.0))
    }
}

impl From<bool> for Shared {
    fn from(value: bool) -> Shared {
        Shared::new(value)
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct ToRead(Flag);

impl ToRead {
    #[must_use]
    pub const fn new(value: bool) -> ToRead {
        ToRead(Flag::new(value))
    }

    #[must_use]
    pub const fn get(self) -> Option<bool> {
        self.0.get()
    }

    #[must_use]
    pub const fn merge(self, other: ToRead) -> ToRead {
        ToRead(self.0.merge(other.0))
    }
}

impl From<bool> for ToRead {
    fn from(value: bool) -> ToRead {
        ToRead::new(value)
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct IsFeed(Flag);

impl IsFeed {
    #[must_use]
    pub const fn new(value: bool) -> IsFeed {
        IsFeed(Flag::new(value))
    }

    #[must_use]
    pub const fn get(self) -> Option<bool> {
        self.0.get()
    }

    #[must_use]
    pub const fn merge(self, other: IsFeed) -> IsFeed {
        IsFeed(self.0.merge(other.0))
    }
}

impl From<bool> for IsFeed {
    fn from(value: bool) -> IsFeed {
        IsFeed::new(value)
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct LastVisitedAt(Option<Time>);

impl LastVisitedAt {
    #[must_use]
    pub const fn new(time: Time) -> LastVisitedAt {
        LastVisitedAt(Some(time))
    }

    #[must_use]
    pub const fn get(self) -> Option<Time> {
        self.0
    }

    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Concat: keeps the most recent (max) time
    #[must_use]
    pub fn merge(self, other: LastVisitedAt) -> LastVisitedAt {
        match (self.0, other.0) {
            (None, None) => LastVisitedAt(None),
            (Some(t), None) | (None, Some(t)) => LastVisitedAt(Some(t)),
            (Some(t1), Some(t2)) => LastVisitedAt(Some(std::cmp::max(t1, t2))),
        }
    }
}

impl From<Time> for LastVisitedAt {
    fn from(time: Time) -> LastVisitedAt {
        LastVisitedAt::new(time)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    #[serde(rename = "uri")]
    url: Url,
    created_at: CreatedAt,
    updated_at: Vec<UpdatedAt>,
    names: BTreeSet<Name>,
    labels: BTreeSet<Label>,
    shared: Shared,
    to_read: ToRead,
    is_feed: IsFeed,
    #[serde(default)]
    extended: Vec<Extended>,
    #[serde(skip_serializing_if = "LastVisitedAt::is_none")]
    last_visited_at: LastVisitedAt,
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
            created_at: CreatedAt::new(created_at),
            updated_at: Vec::new(),
            names: maybe_name.into_iter().collect(),
            labels,
            shared: Shared::default(),
            to_read: ToRead::default(),
            is_feed: IsFeed::default(),
            extended: Vec::new(),
            last_visited_at: LastVisitedAt::default(),
        }
    }

    fn update(
        &mut self,
        updated_at: CreatedAt,
        names: BTreeSet<Name>,
        labels: BTreeSet<Label>,
    ) -> &mut Entity {
        if updated_at < self.created_at {
            self.updated_at.push(UpdatedAt::new(self.created_at.get()));
            self.created_at = updated_at;
        } else {
            self.updated_at.push(UpdatedAt::new(updated_at.get()));
        }
        // Sort updated_at to maintain chronological order
        self.updated_at.sort();
        self.names.extend(names);
        self.labels.extend(labels);
        self
    }

    /// Absorbs `other` into `self`.
    ///
    /// Merging an entity that already equals `self` is a no-op: without that guard, re-absorbing
    /// an identical entity would append a redundant `updated_at` equal to `created_at` and repeat
    /// its extended descriptions, so the result would depend on how many times the same bookmark
    /// appeared in the input.
    pub fn merge(&mut self, other: Entity) -> &mut Entity {
        if *self == other {
            return self;
        }
        self.update(other.created_at, other.names, other.labels);
        self.shared = self.shared.merge(other.shared);
        self.to_read = self.to_read.merge(other.to_read);
        self.is_feed = self.is_feed.merge(other.is_feed);
        self.extended.extend(other.extended);
        self.last_visited_at = self.last_visited_at.merge(other.last_visited_at);
        self
    }

    #[must_use]
    pub fn url(&self) -> &Url {
        &self.url
    }

    #[must_use]
    pub const fn created_at(&self) -> CreatedAt {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> &[UpdatedAt] {
        &self.updated_at
    }

    #[must_use]
    pub fn names(&self) -> &BTreeSet<Name> {
        &self.names
    }

    #[must_use]
    pub fn labels(&self) -> &BTreeSet<Label> {
        &self.labels
    }

    #[must_use]
    pub fn extended(&self) -> &[Extended] {
        &self.extended
    }

    #[must_use]
    pub const fn shared(&self) -> Shared {
        self.shared
    }

    #[must_use]
    pub const fn to_read(&self) -> ToRead {
        self.to_read
    }

    #[must_use]
    pub const fn is_feed(&self) -> IsFeed {
        self.is_feed
    }

    #[must_use]
    pub const fn last_visited_at(&self) -> LastVisitedAt {
        self.last_visited_at
    }

    pub fn labels_mut(&mut self) -> &mut BTreeSet<Label> {
        &mut self.labels
    }
}

impl TryFrom<Post> for Entity {
    type Error = Error;

    fn try_from(post: Post) -> Result<Entity, Error> {
        let url = Url::parse(&post.href)?;
        let created_at = CreatedAt::new(Time::parse_flexible(&post.time)?);
        let extended: Vec<Extended> = post.extended.map(Extended::new).into_iter().collect();

        Ok(Entity {
            url,
            created_at,
            updated_at: Vec::new(),
            names: post.description.into_iter().map(Name::new).collect(),
            labels: post.tags.into_iter().map(Label::new).collect(),
            shared: Shared::new(post.shared),
            to_read: ToRead::new(post.toread),
            is_feed: IsFeed::new(false),
            extended,
            last_visited_at: LastVisitedAt::default(),
        })
    }
}

pub mod html {
    use std::collections::{BTreeSet, HashMap};

    use super::{
        CreatedAt, Entity, Error, Extended, IsFeed, Label, LastVisitedAt, Name, Shared, Time,
        ToRead, UpdatedAt, Url,
    };

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
            // Normalize once. The href lookup below used the key verbatim while the match
            // lowercased it, so an attribute map using the file's own casing - which this
            // public entry point accepts - failed with MissingUrl on an uppercase HREF.
            let attrs: HashMap<String, String> = attrs
                .into_iter()
                .map(|(key, value)| (key.to_lowercase(), value))
                .collect();

            let href = attrs.get(KEY_HREF).ok_or(Error::MissingUrl)?;
            let url = Url::parse(href)?;

            let mut entity = Entity {
                url,
                created_at: CreatedAt::default(),
                updated_at: Vec::new(),
                names,
                labels,
                shared: Shared::default(),
                to_read: ToRead::default(),
                is_feed: IsFeed::default(),
                extended,
                last_visited_at: LastVisitedAt::default(),
            };

            let mut tags = String::new();
            // Carried alongside the entity so the decision does not depend on whether TAGS or
            // TOREAD came first in the attribute list, which for a HashMap is arbitrary.
            let mut tag_to_read = false;

            for (key, value) in attrs {
                let trimmed = value.trim();
                match key.as_str() {
                    KEY_ADD_DATE if !trimmed.is_empty() => {
                        entity.created_at = CreatedAt::new(Time::parse_timestamp(trimmed)?);
                    }
                    KEY_LAST_MODIFIED if !trimmed.is_empty() => {
                        let time = Time::parse_timestamp(trimmed)?;
                        entity.updated_at.push(UpdatedAt::new(time));
                    }
                    KEY_LAST_VISIT if !trimmed.is_empty() => {
                        let time = Time::parse_timestamp(trimmed)?;
                        entity.last_visited_at = LastVisitedAt::new(time);
                    }
                    KEY_TAGS if !trimmed.is_empty() => {
                        tags = value;
                    }
                    KEY_PRIVATE => {
                        entity.shared = Shared::new(trimmed != "1");
                    }
                    KEY_TOREAD => {
                        entity.to_read = ToRead::new(trimmed == "1");
                    }
                    KEY_FEED => {
                        entity.is_feed = IsFeed::new(trimmed == "true");
                    }
                    _ => {}
                }
            }

            for tag in tags.split(',') {
                let s = tag.trim();
                if s.is_empty() {
                    continue;
                }
                // An exact comparison, so a tag like "toreading" stays an ordinary label.
                if s == KEY_TOREAD {
                    tag_to_read = true;
                    continue;
                }
                entity.labels.insert(Label::from(s));
            }

            // An explicit TOREAD attribute is authoritative; the tag decides only in its absence.
            if entity.to_read.get().is_none() && tag_to_read {
                entity.to_read = ToRead::new(true);
            }

            Ok(entity)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use super::{Entity, Error, Extended, Flag, Label, LastVisitedAt, Time, Url};

    fn entity_at(url: &str, secs: i64) -> Entity {
        let url = Url::parse(url).unwrap();
        let time = Time::parse_timestamp(&secs.to_string()).unwrap();
        Entity::new(url, time, None, BTreeSet::default())
    }

    /// `merge` used to drop the incoming extended descriptions entirely.
    #[test]
    fn merge_concatenates_extended() {
        let mut a = entity_at("https://example.com/", 100);
        a.extended.push(Extended::from("first"));

        let mut b = entity_at("https://example.com/", 200);
        b.extended.push(Extended::from("second"));

        a.merge(b);

        assert_eq!(
            a.extended,
            vec![Extended::from("first"), Extended::from("second")]
        );
    }

    #[test]
    fn parses_unix_timestamp_and_iso8601_alike() {
        let from_unix = Time::parse_flexible("1700000000").unwrap();
        let from_iso = Time::parse_flexible("2023-11-14T22:13:20Z").unwrap();
        assert_eq!(from_unix, from_iso);
        assert_eq!(from_unix.timestamp(), 1_700_000_000);
    }

    /// Timestamps are UTC regardless of the caller's TZ. hbt-ocaml parsed them as local time,
    /// which made its output machine-dependent; chrono's `DateTime<Utc>` rules that out here, and
    /// an offset in the input is converted rather than ignored.
    #[test]
    fn parses_iso8601_offset_as_utc() {
        let utc = Time::parse_flexible("2023-11-14T22:13:20Z").unwrap();
        let offset = Time::parse_flexible("2023-11-14T17:13:20-05:00").unwrap();
        assert_eq!(utc, offset);
    }

    #[test]
    fn parses_pre_epoch_timestamp() {
        let time = Time::parse_flexible("-86400").unwrap();
        assert_eq!(time.timestamp(), -86_400);
    }

    #[test]
    fn parse_flexible_trims_surrounding_whitespace() {
        let time = Time::parse_flexible("  1700000000\n").unwrap();
        assert_eq!(time.timestamp(), 1_700_000_000);
    }

    /// A string that is neither form must report the ISO 8601 failure, not the integer one.
    #[test]
    fn parse_flexible_rejects_garbage() {
        let err = Time::parse_flexible("not a date").unwrap_err();
        assert!(matches!(err, Error::Chrono(..)), "{err:?}");
    }

    #[test]
    fn parse_timestamp_rejects_out_of_range() {
        let err = Time::parse_timestamp("999999999999999").unwrap_err();
        assert!(matches!(err, Error::ParseTimestamp(..)), "{err:?}");
    }

    #[test]
    fn flag_merge_absorbs_unset_and_ors_values() {
        assert_eq!(Flag::default().merge(Flag::default()).get(), None);
        assert_eq!(Flag::default().merge(Flag::new(true)).get(), Some(true));
        assert_eq!(Flag::new(false).merge(Flag::default()).get(), Some(false));
        assert_eq!(Flag::new(false).merge(Flag::new(true)).get(), Some(true));
        assert_eq!(Flag::new(false).merge(Flag::new(false)).get(), Some(false));
    }

    #[test]
    fn last_visited_at_merge_keeps_the_later_time() {
        let early = LastVisitedAt::new(Time::parse_timestamp("100").unwrap());
        let late = LastVisitedAt::new(Time::parse_timestamp("200").unwrap());

        assert_eq!(early.merge(late).get(), late.get());
        assert_eq!(late.merge(early).get(), late.get());
        assert_eq!(LastVisitedAt::default().merge(late).get(), late.get());
        assert!(
            LastVisitedAt::default()
                .merge(LastVisitedAt::default())
                .is_none()
        );
    }

    fn from_attrs(pairs: &[(&str, &str)]) -> Entity {
        let attrs: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        Entity::from_attrs(attrs, BTreeSet::default(), BTreeSet::default(), Vec::new()).unwrap()
    }

    fn labels_of(entity: &Entity) -> Vec<&str> {
        entity.labels().iter().map(Label::as_str).collect()
    }

    const HREF: (&str, &str) = ("href", "https://example.com/");

    /// An explicit TOREAD attribute wins over a toread tag, whichever order they appear in.
    /// The tag used to win unconditionally, since the tag loop ran after the attribute loop.
    #[test]
    fn explicit_toread_attribute_overrides_toread_tag() {
        let entity = from_attrs(&[HREF, ("tags", "toread"), ("toread", "0")]);
        assert_eq!(entity.to_read().get(), Some(false));
        assert!(labels_of(&entity).is_empty());
    }

    #[test]
    fn toread_tag_applies_when_attribute_is_absent() {
        let entity = from_attrs(&[HREF, ("tags", "x,toread")]);
        assert_eq!(entity.to_read().get(), Some(true));
        assert_eq!(labels_of(&entity), vec!["x"]);
    }

    #[test]
    fn explicit_toread_attribute_is_kept_when_set() {
        let entity = from_attrs(&[HREF, ("tags", "x"), ("toread", "1")]);
        assert_eq!(entity.to_read().get(), Some(true));
    }

    /// The comparison is exact, so a tag merely containing "toread" is an ordinary label.
    #[test]
    fn toreading_tag_is_an_ordinary_label() {
        let entity = from_attrs(&[HREF, ("tags", "toreading")]);
        assert_eq!(entity.to_read().get(), None);
        assert_eq!(labels_of(&entity), vec!["toreading"]);
    }

    /// Tags are trimmed, so "x, toread" is the tag "x" plus the marker, not a label " toread".
    #[test]
    fn tags_are_trimmed() {
        let entity = from_attrs(&[HREF, ("tags", "x, toread , y")]);
        assert_eq!(entity.to_read().get(), Some(true));
        assert_eq!(labels_of(&entity), vec!["x", "y"]);
    }

    #[test]
    fn to_read_is_unset_without_tag_or_attribute() {
        let entity = from_attrs(&[HREF, ("tags", "x")]);
        assert_eq!(entity.to_read().get(), None);
    }

    /// PRIVATE is inverted: PRIVATE="1" means not shared.
    #[test]
    fn private_attribute_inverts_into_shared() {
        assert_eq!(
            from_attrs(&[HREF, ("private", "1")]).shared().get(),
            Some(false)
        );
        assert_eq!(
            from_attrs(&[HREF, ("private", "0")]).shared().get(),
            Some(true)
        );
        assert_eq!(from_attrs(&[HREF]).shared().get(), None);
    }

    #[test]
    fn feed_attribute_reads_true_literally() {
        assert_eq!(
            from_attrs(&[HREF, ("feed", "true")]).is_feed().get(),
            Some(true)
        );
        assert_eq!(
            from_attrs(&[HREF, ("feed", "false")]).is_feed().get(),
            Some(false)
        );
        assert_eq!(from_attrs(&[HREF]).is_feed().get(), None);
    }

    #[test]
    fn reads_the_timestamp_attributes() {
        let entity = from_attrs(&[
            HREF,
            ("add_date", "100"),
            ("last_modified", "200"),
            ("last_visit", "300"),
        ]);

        assert_eq!(entity.created_at().get().timestamp(), 100);
        assert_eq!(
            entity
                .updated_at()
                .iter()
                .map(|u| u.get().timestamp())
                .collect::<Vec<_>>(),
            vec![200]
        );
        assert_eq!(
            entity.last_visited_at().get().map(Time::timestamp),
            Some(300)
        );
    }

    /// Attribute names arrive in whatever case the file used.
    #[test]
    fn attribute_names_are_matched_case_insensitively() {
        let entity = from_attrs(&[("HREF", "https://example.com/"), ("ADD_DATE", "100")]);
        assert_eq!(entity.created_at().get().timestamp(), 100);
    }

    #[test]
    fn from_attrs_requires_href() {
        let attrs = HashMap::from([("add_date".to_string(), "100".to_string())]);
        let err = Entity::from_attrs(attrs, BTreeSet::default(), BTreeSet::default(), Vec::new())
            .unwrap_err();
        assert!(matches!(err, Error::MissingUrl), "{err:?}");
    }

    /// Absorbing an identical entity used to append a redundant `updated_at` equal to
    /// `created_at` and repeat the extended description once per occurrence.
    #[test]
    fn merge_is_idempotent_for_identical_entities() {
        let mut a = entity_at("https://example.com/", 100);
        a.extended.push(Extended::from("desc"));
        let before = a.clone();

        a.merge(before.clone());
        a.merge(before.clone());

        assert_eq!(a, before);
        assert!(a.updated_at.is_empty());
        assert_eq!(a.extended, vec![Extended::from("desc")]);
    }

    #[test]
    fn merge_keeps_extended_when_other_has_none() {
        let mut a = entity_at("https://example.com/", 100);
        a.extended.push(Extended::from("only"));

        a.merge(entity_at("https://example.com/", 200));

        assert_eq!(a.extended, vec![Extended::from("only")]);
    }
}
