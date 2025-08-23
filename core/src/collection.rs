#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};

use minijinja::{Environment, context};
use serde::{Deserialize, Serialize};
use thiserror::Error;
#[cfg(feature = "pinboard")]
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, serde::timestamp};
use url::Url;

#[cfg(feature = "pinboard")]
use crate::pinboard::Post;

#[derive(Debug, Error)]
pub enum Error {
    #[error("incompatible version: {0}, expected: {1}")]
    IncompatibleVersion(String, String),
    #[error("version parsing error: {0}")]
    ParseSemver(#[from] semver::Error),
    #[error("URL parsing error: {0}")]
    ParseUrl(#[from] url::ParseError),
    #[error("integer parsing error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("time parsing error: {0}")]
    ParseTime(#[from] time::error::ComponentRange),
    #[error("time format parsing error: {0}")]
    ParseTimeFormat(#[from] time::error::Parse),
    #[error("HTML selector error: {0}")]
    HtmlSelector(String),
    #[error("template error: {0}")]
    Template(#[from] minijinja::Error),
}

impl From<scraper::error::SelectorErrorKind<'_>> for Error {
    fn from(value: scraper::error::SelectorErrorKind<'_>) -> Self {
        Error::HtmlSelector(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct Version(semver::Version);

impl Version {
    const fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version(semver::Version::new(major, minor, patch))
    }

    fn matches_requirement(&self) -> Result<bool, semver::Error> {
        let req = semver::VersionReq::parse(Self::EXPECTED_REQ)?;
        Ok(req.matches(&self.0))
    }

    const EXPECTED: Version = Version::new(0, 1, 0);
    const EXPECTED_REQ: &str = "^0.1.0";
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An [`Id`] is a unique identifier for an [`Entity`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id(usize);

impl Id {
    const fn new(id: usize) -> Id {
        Id(id)
    }
}

impl From<Id> for usize {
    fn from(id: Id) -> usize {
        id.0
    }
}

/// A [`Name`] describes an [`Entity`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Time(#[serde(with = "timestamp")] OffsetDateTime);

impl Time {
    pub const fn new(time: OffsetDateTime) -> Time {
        Time(time)
    }

    #[cfg(feature = "pinboard")]
    fn parse(time: &str) -> Result<Time, Error> {
        let timestamp: i64 = time.parse()?;
        let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
        Ok(Time(time))
    }

    #[cfg(feature = "pinboard")]
    fn parse_iso8601(time: &str) -> Result<Time, Error> {
        let time = OffsetDateTime::parse(time, &Rfc3339)?;
        Ok(Time(time))
    }

    #[cfg(feature = "pinboard")]
    fn parse_flexible(time: &str) -> Result<Time, Error> {
        // Try Unix timestamp first (all digits, possibly with leading/trailing whitespace)
        if time.trim().chars().all(|c| c.is_ascii_digit()) {
            Self::parse(time.trim())
        } else {
            // Try ISO 8601 format
            Self::parse_iso8601(time.trim())
        }
    }
}

impl From<OffsetDateTime> for Time {
    fn from(time: OffsetDateTime) -> Time {
        Time(time)
    }
}

impl Default for Time {
    fn default() -> Time {
        Time(OffsetDateTime::UNIX_EPOCH)
    }
}

/// A [`Extended`] is text that can be attached to an [`Entity`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    #[serde(rename = "uri")]
    url: Url,
    created_at: Time,
    updated_at: Vec<Time>,
    names: BTreeSet<Name>,
    labels: BTreeSet<Label>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended: Option<Extended>,
    shared: bool,
    to_read: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_visited_at: Option<Time>,
    is_feed: bool,
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

    pub fn with_extended(mut self, extended: Option<Extended>) -> Entity {
        self.extended = extended;
        self
    }

    pub fn with_shared(mut self, shared: bool) -> Entity {
        self.shared = shared;
        self
    }

    pub fn with_to_read(mut self, to_read: bool) -> Entity {
        self.to_read = to_read;
        self
    }

    pub fn with_last_visited_at(mut self, last_visited_at: Option<Time>) -> Entity {
        self.last_visited_at = last_visited_at;
        self
    }

    pub fn with_is_feed(mut self, is_feed: bool) -> Entity {
        self.is_feed = is_feed;
        self
    }

    pub fn with_updated_at(mut self, updated_at: Vec<Time>) -> Entity {
        self.updated_at = updated_at;
        self
    }
}

#[cfg(feature = "pinboard")]
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

pub type Edges = Vec<Id>;

/// A collection of entities.
///
/// This is a graph structure where a nodes are represented by a vector of entities and edges are
/// represented by an adjacency list.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Collection {
    nodes: Vec<Entity>,
    edges: Vec<Edges>,
    urls: HashMap<Url, Id>,
}

impl Index<Id> for Vec<Entity> {
    type Output = Entity;

    fn index(&self, id: Id) -> &Self::Output {
        &self[id.0]
    }
}

impl IndexMut<Id> for Vec<Entity> {
    fn index_mut(&mut self, id: Id) -> &mut Self::Output {
        &mut self[id.0]
    }
}

impl Index<Id> for Vec<Edges> {
    type Output = Edges;

    fn index(&self, id: Id) -> &Self::Output {
        &self[id.0]
    }
}

impl IndexMut<Id> for Vec<Edges> {
    fn index_mut(&mut self, id: Id) -> &mut Self::Output {
        &mut self[id.0]
    }
}

impl Collection {
    pub fn new() -> Collection {
        let nodes = Vec::new();
        let edges = Vec::new();
        let urls = HashMap::new();
        Collection { nodes, edges, urls }
    }

    pub fn with_capacity(capacity: usize) -> Collection {
        let nodes = Vec::with_capacity(capacity);
        let edges = Vec::with_capacity(capacity);
        let urls = HashMap::with_capacity(capacity);
        Collection { nodes, edges, urls }
    }

    pub fn len(&self) -> usize {
        let len = self.nodes.len();
        assert_eq!(len, self.edges.len());
        len
    }

    pub fn is_empty(&self) -> bool {
        let is_empty = self.nodes.is_empty();
        assert_eq!(is_empty, self.edges.is_empty());
        is_empty
    }

    pub fn contains(&self, url: &Url) -> bool {
        self.urls.contains_key(url)
    }

    pub fn id(&self, url: &Url) -> Option<Id> {
        self.urls.get(url).copied()
    }

    pub fn insert(&mut self, entity: Entity) -> Id {
        let id = Id::new(self.len());
        self.nodes.push(entity);
        self.edges.push(Vec::new());
        let url = self.nodes[id].url().to_owned();
        self.urls.insert(url, id);
        id
    }

    pub fn upsert(&mut self, other: Entity) -> Id {
        if let Some(id) = self.id(other.url()) {
            let entity = &mut self.nodes[id];
            entity.merge(other);
            id
        } else {
            self.insert(other)
        }
    }

    pub fn add_edge(&mut self, from: Id, to: Id) {
        let from_edges = &mut self.edges[from];
        if from_edges.contains(&to) {
            return;
        }
        from_edges.push(to);
    }

    pub fn add_edges(&mut self, from: Id, to: Id) {
        self.add_edge(from, to);
        self.add_edge(to, from)
    }

    pub fn entity(&self, id: Id) -> &Entity {
        &self.nodes[id]
    }

    pub fn entity_mut(&mut self, id: Id) -> &mut Entity {
        &mut self.nodes[id]
    }

    pub fn edges(&self, id: Id) -> &[Id] {
        &self.edges[id]
    }

    pub fn entities(&self) -> &[Entity] {
        &self.nodes
    }

    pub fn update_labels<M>(&mut self, mappings: M) -> Result<(), Error>
    where
        M: IntoIterator<Item = (String, String)>,
    {
        let mapping: BTreeMap<Label, Label> =
            mappings.into_iter().map(|(k, v)| (Label::from(k), Label::from(v))).collect();

        for node in self.nodes.iter_mut() {
            let labels = node.labels_mut();
            let to_add: BTreeSet<Label> =
                labels.iter().filter_map(|label| mapping.get(label).cloned()).collect();
            labels.retain(|label| !mapping.contains_key(label));
            labels.extend(to_add);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedNode {
    id: Id,
    entity: Entity,
    edges: Vec<Id>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedCollection {
    version: Version,
    length: usize,
    value: Vec<SerializedNode>,
}

impl From<&Collection> for SerializedCollection {
    fn from(collection: &Collection) -> SerializedCollection {
        let version = Version::EXPECTED;

        let length = collection.len();

        let value: Vec<_> = (0..length)
            .map(|i| {
                let id = Id::new(i);
                let entity = collection.entity(id).clone();
                let edges = collection.edges(id).to_vec();
                SerializedNode { id, entity, edges }
            })
            .collect();

        SerializedCollection { version, length, value }
    }
}

impl TryFrom<SerializedCollection> for Collection {
    type Error = Error;

    fn try_from(serialized_collection: SerializedCollection) -> Result<Collection, Self::Error> {
        let SerializedCollection { version, length, mut value } = serialized_collection;

        let is_compatible_version = version.matches_requirement()?;

        if !is_compatible_version {
            return Err(Error::IncompatibleVersion(
                version.to_string(),
                Version::EXPECTED_REQ.to_string(),
            ));
        }

        let mut ret = Collection::with_capacity(length);

        value.sort();

        for SerializedNode { id, entity, edges } in value {
            assert_eq!(id.0, ret.len());
            let url = entity.url.clone();
            ret.nodes.push(entity);
            ret.edges.push(edges);
            ret.urls.insert(url, id);
        }

        Ok(ret)
    }
}

impl Serialize for Collection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerializedCollection::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Collection {
    fn deserialize<D>(deserializer: D) -> Result<Collection, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let collection = SerializedCollection::deserialize(deserializer)?;
        Collection::try_from(collection).map_err(serde::de::Error::custom)
    }
}

mod netscape {
    use std::collections::HashMap;

    use scraper::{ElementRef, Html, Selector};

    use super::*;

    #[derive(Debug)]
    enum StackItem<'a> {
        Element(ElementRef<'a>),
        PopGroup,
    }

    type Attributes = HashMap<String, String>;

    pub fn from_html_str(html: &str) -> Result<Collection, Error> {
        let document = Html::parse_document(html);
        let root = document.root_element();

        let mut collection = Collection::new();
        let mut stack: Vec<StackItem> = Vec::new();
        let mut folder_stack: Vec<String> = Vec::new();
        let mut pending_bookmark: Option<(Attributes, Option<String>)> = None;

        const A: &str = "a";
        const H3: &str = "h3";
        const DT: &str = "dt";
        const DD: &str = "dd";
        const DL: &str = "dl";

        let a_selector = Selector::parse(A)?;
        let h3_selector = Selector::parse(H3)?;

        for child in root.children().rev() {
            if let Some(child_element) = ElementRef::wrap(child) {
                stack.push(StackItem::Element(child_element));
            }
        }

        while let Some(item) = stack.pop() {
            match item {
                StackItem::Element(element) => {
                    match element.value().name() {
                        DT => {
                            if let Some((attrs, maybe_description)) = pending_bookmark.take() {
                                add_pending(
                                    &mut collection,
                                    &folder_stack,
                                    attrs,
                                    maybe_description,
                                    None, // No extended
                                )?;
                            }
                            if let Some(h3_element) = element.select(&h3_selector).next() {
                                if let Some(folder_name) = maybe_element_text(h3_element) {
                                    folder_stack.push(folder_name);
                                }
                            } else if let Some(a_element) = element.select(&a_selector).next() {
                                let attrs = extract_attributes(a_element);
                                let maybe_description = maybe_element_text(a_element);
                                pending_bookmark = Some((attrs, maybe_description));
                            }
                        }
                        DD => {
                            if let Some((attrs, maybe_description)) = pending_bookmark.take() {
                                let maybe_extended = maybe_element_text(element);
                                add_pending(
                                    &mut collection,
                                    &folder_stack,
                                    attrs,
                                    maybe_description,
                                    maybe_extended,
                                )?;
                            }
                        }
                        DL => {
                            stack.push(StackItem::PopGroup);
                        }
                        _ => {}
                    }
                    for child in element.children().rev() {
                        if let Some(child_element) = ElementRef::wrap(child) {
                            stack.push(StackItem::Element(child_element));
                        }
                    }
                }
                StackItem::PopGroup => {
                    if let Some((attrs, maybe_description)) = pending_bookmark.take() {
                        add_pending(
                            &mut collection,
                            &folder_stack,
                            attrs,
                            maybe_description,
                            None,
                        )?;
                    }
                    folder_stack.pop();
                }
            }
        }

        assert!(pending_bookmark.is_none());

        Ok(collection)
    }

    fn maybe_element_text(element: ElementRef) -> Option<String> {
        let trimmed = element.text().collect::<String>().trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    }

    fn extract_attributes(element: ElementRef) -> Attributes {
        let mut attrs = HashMap::new();
        for (name, value) in element.value().attrs() {
            attrs.insert(name.to_lowercase(), value.to_string());
        }
        attrs
    }

    fn add_pending(
        collection: &mut Collection,
        folder_stack: &[String],
        attrs: Attributes,
        description: Option<String>,
        extended: Option<String>,
    ) -> Result<(), Error> {
        const ATTR_HREF: &str = "href";
        const ATTR_ADD_DATE: &str = "add_date";
        const ATTR_LAST_MODIFIED: &str = "last_modified";
        const ATTR_LAST_VISIT: &str = "last_visit";
        const ATTR_TAGS: &str = "tags";
        const ATTR_TOREAD: &str = "toread";
        const ATTR_PRIVATE: &str = "private";
        const ATTR_FEED: &str = "feed";

        let url = {
            let href = attrs.get(ATTR_HREF).ok_or(Error::ParseUrl(url::ParseError::EmptyHost))?;
            Url::parse(href)?
        };

        let created_at = parse_timestamp_attr(&attrs, ATTR_ADD_DATE)?;
        let last_modified = parse_timestamp_attr_opt(&attrs, ATTR_LAST_MODIFIED)?;
        let last_visited_at = parse_timestamp_attr_opt(&attrs, ATTR_LAST_VISIT)?;

        let tag_string = attrs.get(ATTR_TAGS).cloned().unwrap_or_default();
        let tags: Vec<String> = if tag_string.is_empty() {
            Vec::new()
        } else {
            tag_string.split(',').map(|s| s.trim().to_string()).collect()
        };

        let labels: BTreeSet<Label> = folder_stack
            .iter()
            .chain(tags.iter())
            .filter(|&tag| tag != ATTR_TOREAD)
            .map(|tag| Label::from(tag.clone()))
            .collect();

        let shared = !matches!(attrs.get(ATTR_PRIVATE), Some(val) if val == "1");

        let to_read = attrs.get(ATTR_TOREAD).is_some_and(|val| val == "1")
            || tag_string.contains(ATTR_TOREAD);

        let is_feed = attrs.get(ATTR_FEED).is_some_and(|val| val == "true");

        let updated_at: Vec<Time> = last_modified.into_iter().collect();

        let entity = Entity::new(url, created_at, description.map(Name::from), labels)
            .with_extended(extended.map(Extended::from))
            .with_shared(shared)
            .with_to_read(to_read)
            .with_last_visited_at(last_visited_at)
            .with_is_feed(is_feed)
            .with_updated_at(updated_at);

        collection.upsert(entity);

        Ok(())
    }

    fn parse_timestamp_attr(attrs: &Attributes, key: &str) -> Result<Time, Error> {
        parse_timestamp_attr_opt(attrs, key).map(Option::unwrap_or_default)
    }

    fn parse_timestamp_attr_opt(attrs: &Attributes, key: &str) -> Result<Option<Time>, Error> {
        if let Some(timestamp_str) = attrs.get(key) {
            let trimmed = timestamp_str.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            let timestamp: i64 = trimmed.parse()?;
            let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
            Ok(Some(Time::new(time)))
        } else {
            Ok(None)
        }
    }

    pub fn to_html(collection: &Collection) -> Result<String, Error> {
        const TEMPLATE: &str = include_str!("collection/netscape_bookmarks.jinja");
        let mut env = Environment::new();
        env.add_template("netscape", TEMPLATE)?;
        let entities = collection.entities();
        let template = env.get_template("netscape")?;
        let rendered = template.render(context! { entities })?;
        Ok(rendered)
    }
}

impl Collection {
    pub fn from_html_str(html: &str) -> Result<Collection, Error> {
        netscape::from_html_str(html)
    }

    pub fn to_html(&self) -> Result<String, Error> {
        netscape::to_html(self)
    }
}
