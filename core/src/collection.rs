use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    ops::{Index, IndexMut},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::entity::{Entity, Label, Url};

#[derive(Debug, Error)]
pub enum Error {
    #[error("incompatible version: {0}, expected: {1}")]
    IncompatibleVersion(String, String),

    #[error("version parsing error: {0}")]
    ParseSemver(#[from] semver::Error),
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[schemars(transparent)]
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

pub type Edges = Vec<Id>;

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
        let Some(id) = self.id(other.url()) else {
            return self.insert(other);
        };
        let entity = &mut self.nodes[id];
        entity.merge(other);
        id
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

    pub fn update_labels(
        &mut self,
        mappings: impl IntoIterator<Item = (String, String)>,
    ) -> Result<(), Error> {
        let mapping: BTreeMap<Label, Label> = mappings
            .into_iter()
            .map(|(k, v)| (Label::from(k), Label::from(v)))
            .collect();

        for node in self.nodes.iter_mut() {
            let labels = node.labels_mut();
            let to_add: BTreeSet<Label> = labels
                .iter()
                .filter_map(|label| mapping.get(label).cloned())
                .collect();
            labels.retain(|label| !mapping.contains_key(label));
            labels.extend(to_add);
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct NodeRepr {
    id: Id,
    entity: Entity,
    edges: Vec<Id>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectionRepr {
    version: Version,
    length: usize,
    value: Vec<NodeRepr>,
}

impl From<&Collection> for CollectionRepr {
    fn from(coll: &Collection) -> CollectionRepr {
        let version = Version::EXPECTED;

        let length = coll.len();

        let value: Vec<_> = (0..length)
            .map(|i| {
                let id = Id::new(i);
                let entity = coll.entity(id).clone();
                let edges = coll.edges(id).to_vec();
                NodeRepr { id, entity, edges }
            })
            .collect();

        CollectionRepr {
            version,
            length,
            value,
        }
    }
}

impl TryFrom<CollectionRepr> for Collection {
    type Error = Error;

    fn try_from(mut repr: CollectionRepr) -> Result<Collection, Self::Error> {
        if !repr.version.matches_requirement()? {
            return Err(Error::IncompatibleVersion(
                repr.version.to_string(),
                Version::EXPECTED_REQ.to_string(),
            ));
        }

        let mut ret = Collection::with_capacity(repr.length);

        repr.value.sort();

        for NodeRepr { id, entity, edges } in repr.value {
            assert_eq!(id.0, ret.len());
            let url = entity.url().clone();
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
        CollectionRepr::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Collection {
    fn deserialize<D>(deserializer: D) -> Result<Collection, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let coll = CollectionRepr::deserialize(deserializer)?;
        Collection::try_from(coll).map_err(serde::de::Error::custom)
    }
}
