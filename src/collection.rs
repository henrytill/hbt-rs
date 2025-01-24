#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeSet, HashMap},
    fmt,
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};

use serde::{Deserialize, Serialize};
use time::{serde::timestamp, OffsetDateTime};
use url::Url;

#[cfg(feature = "pinboard")]
use crate::pinboard::Post;

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

#[cfg(test)]
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
    fn parse(time: &str) -> Result<Time, anyhow::Error> {
        let timestamp: i64 = time.parse()?;
        let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
        Ok(Time(time))
    }
}

impl From<OffsetDateTime> for Time {
    fn from(time: OffsetDateTime) -> Time {
        Time(time)
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
    toread: bool,
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
        let toread = false;
        Entity { url, created_at, updated_at, names, labels, extended, shared, toread }
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
}

#[cfg(feature = "pinboard")]
impl TryFrom<Post> for Entity {
    type Error = anyhow::Error;

    fn try_from(post: Post) -> Result<Entity, Self::Error> {
        let url = Url::parse(&post.href)?;
        let created_at = Time::parse(&post.time)?;
        let updated_at: Vec<Time> = Vec::new();
        let names = {
            let mut tmp = BTreeSet::new();
            if let Some(name) = post.description.map(Name::new) {
                tmp.insert(name);
            }
            tmp
        };
        let labels = {
            let mut tmp = BTreeSet::new();
            tmp.extend(post.tags.into_iter().map(Label::new));
            tmp
        };
        let extended = post.extended.map(Extended::new);
        let shared = post.shared;
        let toread = post.toread;
        Ok(Entity { url, created_at, updated_at, names, labels, extended, shared, toread })
    }
}

pub type Edges = Vec<Id>;

/// A collection of entities.
///
/// This is a graph structure where a nodes are represented by a vector of entities and edges are
/// represented by an adjacency list.
#[derive(Debug, PartialEq, Eq)]
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

    fn with_capacity(capacity: usize) -> Collection {
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
}

impl Default for Collection {
    fn default() -> Collection {
        Collection::new()
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
    type Error = String;

    fn try_from(serialized_collection: SerializedCollection) -> Result<Collection, Self::Error> {
        let SerializedCollection { version, length, mut value } = serialized_collection;

        let is_compatible_version = version.matches_requirement().map_err(|err| err.to_string())?;

        if !is_compatible_version {
            return Err(format!(
                "incompatible version {}, expected {}",
                Version::EXPECTED,
                Version::EXPECTED_REQ
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
