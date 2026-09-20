use std::{
    cmp::min,
    collections::BTreeSet,
    hash::{Hash, Hasher},
};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
    /// Wraps a `DateTime`, truncated to whole seconds.
    ///
    /// The wire form is `ts_seconds`, so a `Time` holding sub-second precision would compare and
    /// deduplicate in memory differently from how it reads on the page. `Eq` and `Ord` are
    /// derived, and `updated_at` is a `BTreeSet` whose schema declares `uniqueItems`, so keeping
    /// the sub-second part let two instants that serialize identically sit in one entity: a
    /// history repeating its own `created_at` (which `normalize` could not see, since in memory
    /// the two differed), and duplicate entries in a set the schema says has none. Both were
    /// reachable from Pinboard input, whose `time` is RFC 3339 and may carry a fraction.
    ///
    /// Truncating here makes the in-memory value exactly what serializing will emit and decoding
    /// would give back, so every comparison in the program agrees with the wire.
    #[must_use]
    pub fn new(time: DateTime<Utc>) -> Time {
        // In range by construction -- the timestamp came from a `DateTime` -- so the fallback is
        // unreachable, and is spelled this way to keep the constructor panic-free.
        Time(DateTime::from_timestamp(time.timestamp(), 0).unwrap_or(time))
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
        Ok(Time::new(time))
    }

    fn parse_iso8601(time: &str) -> Result<Time, Error> {
        let time = DateTime::parse_from_rfc3339(time)
            .map_err(|err| Error::Chrono(err, time.to_string()))?
            .with_timezone(&Utc);
        Ok(Time::new(time))
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
        Time::new(time)
    }
}

impl Default for Time {
    fn default() -> Time {
        Time(DateTime::UNIX_EPOCH)
    }
}

// The derived `Ord` sorts `None` below every `Some`, which is what `merge` must not do and
// exactly the behaviour henrytill/hbt-data#37 removed. It is kept only so `CreatedAt` can be a
// sort key for a set of entities that all have one (`Collection::from_posts`); to combine two
// creation times, call `merge`, never `min`.
#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct CreatedAt(Option<Time>);

impl CreatedAt {
    #[must_use]
    pub const fn new(time: Time) -> CreatedAt {
        CreatedAt(Some(time))
    }

    #[must_use]
    pub const fn get(self) -> Option<Time> {
        self.0
    }

    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Merges two creation times, keeping the earlier one.
    ///
    /// An absent creation time contributes nothing: an undated mention says nothing about when
    /// the bookmark was created, so it neither claims the creation time nor pushes a real one
    /// into the update history. `None` is therefore the identity, not a very old instant --
    /// which is the whole of henrytill/hbt-data#37, and why this cannot be `min` over the
    /// derived `Ord`, where `None` sorts below every `Some`.
    #[must_use]
    pub fn merge(self, other: CreatedAt) -> CreatedAt {
        match (self.0, other.0) {
            (None, None) => CreatedAt(None),
            (Some(t), None) | (None, Some(t)) => CreatedAt(Some(t)),
            (Some(a), Some(b)) => CreatedAt(Some(min(a, b))),
        }
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
    pub const fn is_none(&self) -> bool {
        self.0.get().is_none()
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
    pub const fn is_none(&self) -> bool {
        self.0.get().is_none()
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
    pub const fn is_none(&self) -> bool {
        self.0.get().is_none()
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
#[serde(rename_all = "camelCase", remote = "Self")]
pub struct Entity {
    #[serde(rename = "uri")]
    url: Url,
    // An absent creation time is omitted rather than written as the epoch, the way `shared` and
    // `last_visited_at` are: the wire had no way to say "undated", so an undated mention used to
    // round-trip into one created on 1970-01-01 and merge differently afterwards.
    // henrytill/hbt-data#37.
    #[serde(default, skip_serializing_if = "CreatedAt::is_none")]
    created_at: CreatedAt,
    // Updates, never including `created_at`. HTML reads ADD_DATE and LAST_MODIFIED
    // independently, so one anchor may state the same instant in both; `normalize` is what makes
    // that not survive, and html/bookmarks_simple pins it. The two are separate fields because a
    // creation time is not an update, and an entity may have been created without ever being
    // updated. A doc comment here would land in the generated schema, which no other field
    // carries; see the `shared` note above.
    updated_at: BTreeSet<UpdatedAt>,
    names: BTreeSet<Name>,
    labels: BTreeSet<Label>,
    // The shared wire format omits an optional field rather than writing it as null or empty; see
    // the fixtures in test-data, where neither ever appears. Deserializing tolerates both.
    #[serde(default, skip_serializing_if = "Shared::is_none")]
    shared: Shared,
    #[serde(default, skip_serializing_if = "ToRead::is_none")]
    to_read: ToRead,
    #[serde(default, skip_serializing_if = "IsFeed::is_none")]
    is_feed: IsFeed,
    // schemars drops a `default` annotation that its own skip_serializing_if would skip, so
    // restate it: the published schema documents that an absent `extended` means the empty list.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    #[schemars(extend("default" = [] as [Extended; 0]))]
    extended: BTreeSet<Extended>,
    #[serde(default, skip_serializing_if = "LastVisitedAt::is_none")]
    last_visited_at: LastVisitedAt,
}

// `remote = "Self"` turns the two derives into inherent `Entity::serialize` and
// `Entity::deserialize` functions, leaving the trait impls to be written here. Serializing just
// forwards. Deserializing normalizes: a serialized history is input like any other, so a
// collection read back must not reintroduce an entity whose `updated_at` holds its `created_at`.
// The corpus cannot pin this half -- there is no YAML *input* format -- so
// `decoding_normalizes_the_update_history` does. See `Entity::normalize`.
impl Serialize for Entity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // The normal form is maintained by three call sites rather than by the type, so nothing
        // stops a fourth construction site forgetting it -- and the two shapes that would expose
        // that (a decoded entity, a hand-written one) are exactly the ones no fixture can reach.
        // Serializing is the universal exit: every YAML output and every conformance comparison
        // passes through here, so a debug build turns the convention into something checked.
        // Closing the representation is the stronger form; see the note in AGENTS.md.
        debug_assert!(self.is_normal(), "un-normalized entity: {self:?}");
        Entity::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Entity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Entity, D::Error> {
        let mut entity = Entity::deserialize(deserializer)?;
        entity.normalize();
        Ok(entity)
    }
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
            updated_at: BTreeSet::new(),
            names: maybe_name.into_iter().collect(),
            labels,
            shared: Shared::default(),
            to_read: ToRead::default(),
            is_feed: IsFeed::default(),
            extended: BTreeSet::new(),
            last_visited_at: LastVisitedAt::default(),
        }
    }

    /// Drops an update that merely repeats `created_at`.
    ///
    /// A timestamp equal to `created_at` carries no information that `created_at` does not
    /// (henrytill/hbt-go#57). An update strictly *below* `created_at` is a different thing and is
    /// untouched: henrytill/hbt-data#34.
    ///
    /// This is the whole of the normal form (henrytill/hbt-data#38), and three places maintain
    /// it. `merge` ends here, so a merge that demotes the later creation time to an update does
    /// not then record the earlier one twice. `Deserialize` and `from_attrs` end here because
    /// both take a history from input: HTML reads `ADD_DATE` and `LAST_MODIFIED` independently,
    /// so one anchor may state the same instant in both -- `html/bookmarks_simple`. The
    /// remaining constructors, `new` and `TryFrom<Post>`, are normal for a weaker reason: they
    /// record no updates at all. One that learns to would have to normalize too, and nothing but
    /// this note says so.
    fn normalize(&mut self) {
        if let Some(created_at) = self.created_at.get() {
            self.updated_at.remove(&UpdatedAt::new(created_at));
        }
    }

    /// Whether the normal form holds. Only `debug_assert!` in `Serialize` asks.
    fn is_normal(&self) -> bool {
        self.created_at
            .get()
            .is_none_or(|created_at| !self.updated_at.contains(&UpdatedAt::new(created_at)))
    }

    /// The merged update history: both histories and both creation times that exist.
    ///
    /// Adding both creation times before removing the winner is what makes merging associative.
    /// Each merge puts its operands' creation times back into the history, so however a sequence
    /// of mentions is bracketed the result is every history and every creation time in it, minus
    /// the smallest creation time. An absent creation time is not one of them: it contributes
    /// nothing to either half, which is henrytill/hbt-data#37. Removing the winner only when the two creation times differ
    /// is not associative, and neither is removing every update at or below `created_at`; both
    /// counterexamples are in henrytill/hbt-data#36, which pins this rule.
    ///
    /// Putting *both* times in, and leaving it to `normalize` to take the winner back out, is
    /// what keeps merging associative -- and is why the removal is not spelled here. So an update
    /// equal to the winner goes, carrying no information that `created_at` does not
    /// (henrytill/hbt-go#57), while one strictly below it stays -- a shape HTML can state, since
    /// it reads `ADD_DATE` and `LAST_MODIFIED` independently.
    fn merged_updates(&self, other: &Entity) -> (CreatedAt, BTreeSet<UpdatedAt>) {
        let created_at = self.created_at.merge(other.created_at);
        let mut updated_at: BTreeSet<UpdatedAt> =
            self.updated_at.union(&other.updated_at).copied().collect();
        // Only a creation time that exists goes back into the history: an absent one has nothing
        // to contribute and must not arrive as an epoch update. henrytill/hbt-data#37.
        updated_at.extend(
            [self.created_at, other.created_at]
                .into_iter()
                .filter_map(CreatedAt::get)
                .map(UpdatedAt::new),
        );
        (created_at, updated_at)
    }

    /// Absorbs `other` into `self`.
    ///
    /// Merging is field-wise, then `normalize`d: the merged history holds both creation times,
    /// and normalizing removes the one that won. The instant that survives is typically another
    /// mention's creation time -- `html/bookmarks_superseded_creation`. Do not re-split it by
    /// giving `merged_updates` a removal of its own; two spellings of one rule is what a later
    /// change would have to keep in step.
    ///
    /// Merging an entity that already equals `self` is a no-op. Since normalizing at parse and at
    /// decode makes an entity whose history repeats its own `created_at` unreachable, the guard
    /// no longer changes the result for anything the program can build -- but nothing in the type
    /// stops such an entity being written down, and for one the guard is what keeps that update.
    /// `merge_is_idempotent_for_identical_entities` pins that, and
    /// `merge_is_idempotent_without_the_guard` pins the stronger claim the normal form buys. The
    /// other three implementations guard the same way -- hbt-hs in `absorb`, outside the
    /// `Semigroup` instance -- so this is parity, not a local quirk, and it cannot affect
    /// associativity: any later merge puts both creation times back regardless.
    ///
    /// Entities that differ bypass the guard, so `updated_at` and `extended` are sets: a
    /// timestamp or a description shared by two of them is kept once rather than once per
    /// occurrence.
    pub fn merge(&mut self, other: Entity) -> &mut Entity {
        if *self == other {
            return self;
        }
        self.merge_unguarded(other)
    }

    /// The field-wise merge, then `normalize`. Split out from `merge` so the equality guard can
    /// be stated as the one line it is, and so a test can ask what the merge does without it.
    fn merge_unguarded(&mut self, other: Entity) -> &mut Entity {
        (self.created_at, self.updated_at) = self.merged_updates(&other);
        self.names.extend(other.names);
        self.labels.extend(other.labels);
        self.shared = self.shared.merge(other.shared);
        self.to_read = self.to_read.merge(other.to_read);
        self.is_feed = self.is_feed.merge(other.is_feed);
        self.extended.extend(other.extended);
        self.last_visited_at = self.last_visited_at.merge(other.last_visited_at);
        self.normalize();
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
    pub fn updated_at(&self) -> &BTreeSet<UpdatedAt> {
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
    pub fn extended(&self) -> &BTreeSet<Extended> {
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
        let extended: BTreeSet<Extended> = post.extended.map(Extended::new).into_iter().collect();

        Ok(Entity {
            url,
            created_at,
            updated_at: BTreeSet::new(),
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
            extended: BTreeSet<Extended>,
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
                updated_at: BTreeSet::new(),
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
                        entity.updated_at.insert(UpdatedAt::new(time));
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

            // ADD_DATE and LAST_MODIFIED are read independently above, so an anchor stating the
            // same instant in both lands here with the repeat -- the `html/bookmarks_simple`
            // shape. Normalizing once the whole anchor is read is what drops it.
            entity.normalize();

            Ok(entity)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use super::{Entity, Error, Extended, Flag, Label, LastVisitedAt, Name, Time, UpdatedAt, Url};

    fn entity_at(url: &str, secs: i64) -> Entity {
        let url = Url::parse(url).unwrap();
        let time = Time::parse_timestamp(&secs.to_string()).unwrap();
        Entity::new(url, time, None, BTreeSet::default())
    }

    fn update_at(secs: i64) -> UpdatedAt {
        UpdatedAt::new(Time::parse_timestamp(&secs.to_string()).unwrap())
    }

    fn created_of(entity: &Entity) -> Option<i64> {
        entity.created_at.get().map(Time::timestamp)
    }

    fn updates_of(entity: &Entity) -> Vec<i64> {
        entity
            .updated_at
            .iter()
            .map(|u| u.get().timestamp())
            .collect()
    }

    /// `merge` used to drop the incoming extended descriptions entirely.
    #[test]
    fn merge_unions_extended() {
        let mut a = entity_at("https://example.com/", 100);
        a.extended.insert(Extended::from("first"));

        let mut b = entity_at("https://example.com/", 200);
        b.extended.insert(Extended::from("second"));

        a.merge(b);

        assert_eq!(
            a.extended,
            BTreeSet::from([Extended::from("first"), Extended::from("second")])
        );
    }

    /// Entities that differ in any field bypass the equality guard in `merge`, so a description
    /// they share used to land once per occurrence. See henrytill/hbt-rs#52.
    #[test]
    fn merge_keeps_a_shared_description_once_for_differing_entities() {
        let mut a = entity_at("https://example.com/", 100);
        a.extended.insert(Extended::from("desc"));
        a.labels.insert(Label::from("a"));

        let mut b = entity_at("https://example.com/", 100);
        b.extended.insert(Extended::from("desc"));
        b.labels.insert(Label::from("b"));

        a.merge(b);

        assert_eq!(a.extended, BTreeSet::from([Extended::from("desc")]));
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

    /// The wire form is whole seconds, so a `Time` must not carry a fraction into memory: two
    /// instants in the same second would compare unequal while serializing identically. Pinboard
    /// `time` is RFC 3339 and may carry one.
    #[test]
    fn parse_flexible_truncates_sub_second_precision() {
        let early = Time::parse_flexible("2010-06-18T20:27:37.100Z").unwrap();
        let late = Time::parse_flexible("2010-06-18T20:27:37.900Z").unwrap();

        assert_eq!(early, late);
        assert_eq!(early.timestamp(), late.timestamp());
    }

    /// A pre-epoch fraction truncates the same way serializing does -- towards the floor second,
    /// which is what `timestamp()` returns -- so the two still agree.
    #[test]
    fn parse_flexible_truncates_pre_epoch_sub_second_precision() {
        let time = Time::parse_flexible("1969-12-31T23:59:59.500Z").unwrap();
        assert_eq!(time.timestamp(), -1);
    }

    /// Two mentions in the same second used to merge into an entity whose history repeated its
    /// own `created_at` on the wire while differing from it in memory, so `normalize` could not
    /// see it and the `debug_assert!` in `Serialize` did not fire. A third mention made the
    /// emitted `updatedAt` hold the same timestamp twice, which the schema forbids
    /// (`uniqueItems`). Truncating at construction is what closes both.
    #[test]
    fn entities_in_one_second_merge_to_a_normal_entity() {
        let url = Url::parse("https://example.com/").unwrap();
        let at = |s: &str| {
            Entity::new(
                url.clone(),
                Time::parse_flexible(s).unwrap(),
                None,
                BTreeSet::default(),
            )
        };

        let mut a = at("2010-06-18T20:27:37.100Z");
        a.names.insert(Name::from("a"));
        let mut b = at("2010-06-18T20:27:37.500Z");
        b.names.insert(Name::from("b"));
        let mut c = at("2010-06-18T20:27:37.900Z");
        c.names.insert(Name::from("c"));

        a.merge(b);
        a.merge(c);

        assert!(a.updated_at.is_empty(), "{:?}", a.updated_at);
        assert!(a.is_normal());
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
        Entity::from_attrs(
            attrs,
            BTreeSet::default(),
            BTreeSet::default(),
            BTreeSet::default(),
        )
        .unwrap()
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

        assert_eq!(created_of(&entity), Some(100));
        assert_eq!(updates_of(&entity), vec![200]);
        assert_eq!(
            entity.last_visited_at().get().map(Time::timestamp),
            Some(300)
        );
    }

    /// Attribute names arrive in whatever case the file used.
    #[test]
    fn attribute_names_are_matched_case_insensitively() {
        let entity = from_attrs(&[("HREF", "https://example.com/"), ("ADD_DATE", "100")]);
        assert_eq!(created_of(&entity), Some(100));
    }

    #[test]
    fn from_attrs_requires_href() {
        let attrs = HashMap::from([("add_date".to_string(), "100".to_string())]);
        let err = Entity::from_attrs(
            attrs,
            BTreeSet::default(),
            BTreeSet::default(),
            BTreeSet::default(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::MissingUrl), "{err:?}");
    }

    /// Absorbing an identical entity used to append a redundant `updated_at` equal to
    /// `created_at` and repeat the extended description once per occurrence. The update equal to
    /// `created_at` is what keeps this test load-bearing: that is the one element `normalize`
    /// removes, so this is the shape the guard still changes -- and, since parsing and decoding
    /// normalize, one only a hand-written entity can have. See `merge`.
    #[test]
    fn merge_is_idempotent_for_identical_entities() {
        let mut a = entity_at("https://example.com/", 100);
        a.updated_at.insert(update_at(100));
        a.extended.insert(Extended::from("desc"));
        let before = a.clone();

        a.merge(before.clone());
        a.merge(before.clone());

        assert_eq!(a, before);
        assert_eq!(updates_of(&a), vec![100]);
        assert_eq!(a.extended, BTreeSet::from([Extended::from("desc")]));
    }

    /// Distinct entities sharing a `created_at` record no update: the timestamp would only repeat
    /// `created_at`. Resolved as henrytill/hbt-go#57, where this implementation was the one that
    /// appended; the equality guard above does not cover it, since the entities differ.
    #[test]
    fn merge_does_not_record_an_update_for_an_equal_timestamp() {
        let mut a = entity_at("https://example.com/", 100);
        a.names.insert(Name::from("a"));

        let mut b = entity_at("https://example.com/", 100);
        b.names.insert(Name::from("b"));

        a.merge(b);

        assert!(a.updated_at.is_empty(), "{:?}", a.updated_at);
        assert_eq!(created_of(&a), Some(100));
        assert_eq!(
            a.names.iter().map(Name::as_str).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn merge_records_a_later_timestamp_as_an_update() {
        let mut a = entity_at("https://example.com/", 100);
        a.merge(entity_at("https://example.com/", 200));

        assert_eq!(created_of(&a), Some(100));
        assert_eq!(updates_of(&a), vec![200]);
    }

    /// Entities that differ in any field bypass the equality guard in `merge`, so an update
    /// timestamp they share used to land once per occurrence. Three occurrences rather than two,
    /// since the count is what varied. See henrytill/hbt-rs#54.
    #[test]
    fn merge_keeps_a_shared_timestamp_once_for_differing_entities() {
        let mut a = entity_at("https://example.com/", 100);
        a.labels.insert(Label::from("a"));

        for label in ["b", "c", "d"] {
            let mut other = entity_at("https://example.com/", 200);
            other.labels.insert(Label::from(label));
            a.merge(other);
        }

        assert_eq!(updates_of(&a), vec![200]);
    }

    /// An earlier timestamp takes over `created_at` and displaces it into `updated_at`.
    #[test]
    fn merge_keeps_the_earliest_timestamp_as_created_at() {
        let mut a = entity_at("https://example.com/", 200);
        a.merge(entity_at("https://example.com/", 100));

        assert_eq!(created_of(&a), Some(100));
        assert_eq!(updates_of(&a), vec![200]);
    }

    /// The incoming entity's own history is kept, not discarded: a second mention of a URL can
    /// state a `LAST_MODIFIED` of its own, and it is an update like any other. Was henrytill/hbt-rs#64,
    /// and `html/bookmarks_incoming_update` pins it.
    #[test]
    fn merge_keeps_the_incoming_history() {
        let mut a = entity_at("https://example.com/", 100);
        let mut b = entity_at("https://example.com/", 200);
        b.updated_at.insert(update_at(300));

        a.merge(b);

        assert_eq!(created_of(&a), Some(100));
        assert_eq!(updates_of(&a), vec![200, 300]);
    }

    /// Merging is associative, which is what decides the rule: see henrytill/hbt-data#36. The
    /// shape that discriminates is a history holding an instant equal to its own `created_at`,
    /// which a single anchor states by repeating `ADD_DATE` in `LAST_MODIFIED`. Removing the winning
    /// creation time only when the two differ passes every other case and fails this one.
    #[test]
    fn merge_is_associative() {
        let mut a = entity_at("https://example.com/", 100);
        a.updated_at.insert(update_at(100));
        let b = entity_at("https://example.com/", 100);
        let c = entity_at("https://example.com/", 200);

        let mut right = b.clone();
        right.merge(c.clone());
        let mut right_all = a.clone();
        right_all.merge(right);

        let mut left = a;
        left.merge(b);
        left.merge(c);

        assert_eq!(left, right_all);
    }

    /// The displaced timestamp is the only update left. A mention that stated the timestamp that
    /// later becomes `created_at` as its own `LAST_MODIFIED` used to leave it there, repeating
    /// `created_at`. See henrytill/hbt-rs#65 and the `html/bookmarks_superseded_creation` fixture.
    #[test]
    fn merge_drops_an_update_the_lowered_created_at_supersedes() {
        let mut a = from_attrs(&[HREF, ("add_date", "200"), ("last_modified", "100")]);
        a.merge(entity_at(HREF.1, 100));

        assert_eq!(created_of(&a), Some(100));
        assert_eq!(updates_of(&a), vec![200]);
    }

    /// The normal form makes the equality guard redundant for everything the program can build:
    /// for a normalized entity the field-wise merge is already idempotent, because the creation
    /// time it puts back is the one `normalize` then removes. Calling `merge_unguarded` is the
    /// point -- through `merge` the guard would answer, and the test could not tell.
    #[test]
    fn merge_is_idempotent_without_the_guard() {
        let mut a = entity_at("https://example.com/", 200);
        a.updated_at.insert(update_at(100));
        a.extended.insert(Extended::from("desc"));
        let before = a.clone();

        a.merge_unguarded(before.clone());

        assert_eq!(a, before);
    }

    /// An anchor may state the same instant in `ADD_DATE` and `LAST_MODIFIED` -- the
    /// `html/bookmarks_simple` shape -- and the parse must not keep the repeat
    /// (henrytill/hbt-data#38). An update strictly below `created_at` is a different thing and
    /// stays, which is the half that tells `normalize` from a rule dropping everything at or
    /// below `created_at`.
    #[test]
    fn parsing_drops_an_update_that_repeats_the_creation_time() {
        let repeat = from_attrs(&[HREF, ("add_date", "100"), ("last_modified", "100")]);
        assert!(repeat.updated_at.is_empty(), "{:?}", repeat.updated_at);
        assert_eq!(created_of(&repeat), Some(100));

        let below = from_attrs(&[HREF, ("add_date", "200"), ("last_modified", "100")]);
        assert_eq!(updates_of(&below), vec![100]);
    }

    /// Decoding normalizes, so a serialized collection cannot reintroduce an entity whose history
    /// repeats its own creation time. The corpus cannot pin this: `hbt` has no YAML *input*
    /// format, so nothing round-trips there.
    ///
    /// Both halves are asserted for the same reason as in the parse test: an implementation that
    /// dropped every update at or below `createdAt` would pass on `100` alone, and only the `50`
    /// separates it from the rule that removes exactly `createdAt`.
    #[test]
    fn decoding_normalizes_the_update_history() {
        let yaml = concat!(
            "uri: https://example.com/\n",
            "createdAt: 100\n",
            "updatedAt: [50, 100, 300]\n",
            "names: []\n",
            "labels: []\n",
        );

        let entity: Entity = serde_norway::from_str(yaml).unwrap();

        assert_eq!(created_of(&entity), Some(100));
        assert_eq!(updates_of(&entity), vec![50, 300]);
    }

    /// An undated mention says nothing about when the bookmark was created, so a dated one wins
    /// outright: the dated instant stays `created_at` rather than being demoted to an update by
    /// an absence standing in as the epoch. henrytill/hbt-data#37, and the shape HTML allows
    /// because `ADD_DATE` is optional.
    #[test]
    fn merge_lets_a_dated_mention_win_over_an_undated_one() {
        let undated = from_attrs(&[HREF, ("tags", "a")]);
        let dated = from_attrs(&[HREF, ("add_date", "1609459200"), ("tags", "b")]);

        let mut a = undated.clone();
        a.merge(dated.clone());
        assert_eq!(created_of(&a), Some(1_609_459_200));
        assert!(a.updated_at.is_empty(), "{:?}", a.updated_at);

        // The other order agrees, which is what makes absence an identity rather than a value.
        let mut b = dated;
        b.merge(undated);
        assert_eq!(created_of(&b), Some(1_609_459_200));
        assert!(b.updated_at.is_empty(), "{:?}", b.updated_at);
    }

    /// Merging two undated mentions cannot invent a creation time.
    #[test]
    fn merge_of_two_undated_mentions_stays_undated() {
        let mut a = from_attrs(&[HREF, ("tags", "a")]);
        a.merge(from_attrs(&[HREF, ("tags", "b")]));

        assert_eq!(created_of(&a), None);
        assert!(a.updated_at.is_empty(), "{:?}", a.updated_at);
    }

    /// Associativity has to survive an absent creation time too, since `CreatedAt::merge` is the
    /// one place the rule stops being `min`.
    #[test]
    fn merge_is_associative_with_an_undated_mention() {
        let a = from_attrs(&[HREF, ("tags", "a")]);
        let b = from_attrs(&[HREF, ("add_date", "200"), ("tags", "b")]);
        let c = from_attrs(&[HREF, ("add_date", "100"), ("tags", "c")]);

        let mut left = a.clone();
        left.merge(b.clone());
        left.merge(c.clone());

        let mut right_inner = b;
        right_inner.merge(c);
        let mut right = a;
        right.merge(right_inner);

        assert_eq!(left, right);
        assert_eq!(created_of(&left), Some(100));
        assert_eq!(updates_of(&left), vec![200]);
    }

    /// An anchor with no `ADD_DATE` parses to an absent creation time, not the epoch, and the
    /// absence survives a round-trip: the wire omits the field rather than writing 0, so decoding
    /// gives back an undated entity instead of one created on 1970-01-01. Before
    /// henrytill/hbt-data#37 that round-trip changed what a later merge produced.
    #[test]
    fn an_undated_entity_round_trips_without_a_creation_time() {
        let undated = from_attrs(&[HREF, ("tags", "a")]);
        assert_eq!(created_of(&undated), None);

        let yaml = serde_norway::to_string(&undated).unwrap();
        assert!(!yaml.contains("createdAt"), "{yaml}");

        let decoded: Entity = serde_norway::from_str(&yaml).unwrap();
        assert_eq!(decoded, undated);
        assert_eq!(created_of(&decoded), None);
    }

    /// A creation time of 0 is a real instant and stays on the wire -- only absence is omitted.
    /// The two were indistinguishable before henrytill/hbt-data#37.
    #[test]
    fn an_epoch_creation_time_is_not_treated_as_absent() {
        let epoch = from_attrs(&[HREF, ("add_date", "0"), ("tags", "a")]);
        assert_eq!(created_of(&epoch), Some(0));

        let yaml = serde_norway::to_string(&epoch).unwrap();
        assert!(yaml.contains("createdAt: 0"), "{yaml}");
    }

    #[test]
    fn merge_keeps_extended_when_other_has_none() {
        let mut a = entity_at("https://example.com/", 100);
        a.extended.insert(Extended::from("only"));

        a.merge(entity_at("https://example.com/", 200));

        assert_eq!(a.extended, BTreeSet::from([Extended::from("only")]));
    }
}
