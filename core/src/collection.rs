use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    ops::{Index, IndexMut},
    rc::{Rc, Weak},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use hbt_pinboard::Post;

use crate::entity::{self, Entity, Label, Url};

#[derive(Debug, Error)]
pub enum Error {
    #[error("incompatible version: {0}, expected: {1}")]
    IncompatibleVersion(String, String),

    #[error("version parsing error: {0}")]
    ParseSemver(#[from] semver::Error),

    #[error("integer conversion error: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),
}

#[derive(Debug, Clone)]
pub struct Id {
    index: usize,
    owner: Weak<()>,
}

impl PartialEq for Id {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && Weak::ptr_eq(&self.owner, &other.owner)
    }
}

impl Eq for Id {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[schemars(transparent)]
struct Version(semver::Version);

impl Version {
    const fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version(semver::Version::new(major, minor, patch))
    }

    fn matches_requirement(&self) -> Result<bool, semver::Error> {
        let req = semver::VersionReq::parse(Version::EXPECTED_REQ)?;
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

type Edges = Vec<usize>;

#[derive(Debug)]
pub struct Collection {
    token: Rc<()>,
    nodes: Vec<Entity>,
    edges: Vec<Edges>,
    urls: HashMap<Url, usize>,
}

impl Index<&Id> for Vec<Entity> {
    type Output = Entity;

    fn index(&self, id: &Id) -> &Self::Output {
        &self[id.index]
    }
}

impl IndexMut<&Id> for Vec<Entity> {
    fn index_mut(&mut self, id: &Id) -> &mut Self::Output {
        &mut self[id.index]
    }
}

impl Index<&Id> for Vec<Edges> {
    type Output = Edges;

    fn index(&self, id: &Id) -> &Self::Output {
        &self[id.index]
    }
}

impl IndexMut<&Id> for Vec<Edges> {
    fn index_mut(&mut self, id: &Id) -> &mut Self::Output {
        &mut self[id.index]
    }
}

impl Collection {
    fn make_id(&self, index: usize) -> Id {
        Id {
            index,
            owner: Rc::downgrade(&self.token),
        }
    }

    fn check_id(&self, id: &Id) {
        if let Some(rc) = id.owner.upgrade() {
            assert!(
                Rc::ptr_eq(&rc, &self.token),
                "Id belongs to a different collection"
            );
        } else {
            panic!("Id's collection has been dropped");
        }
    }

    #[must_use]
    pub fn new() -> Collection {
        Collection {
            token: Rc::new(()),
            nodes: Vec::new(),
            edges: Vec::new(),
            urls: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Collection {
        Collection {
            token: Rc::new(()),
            nodes: Vec::with_capacity(capacity),
            edges: Vec::with_capacity(capacity),
            urls: HashMap::with_capacity(capacity),
        }
    }

    /// Returns the number of entities in the collection.
    ///
    /// # Panics
    ///
    /// Panics if the internal invariant is violated (nodes and edges length mismatch).
    #[must_use]
    pub fn len(&self) -> usize {
        let len = self.nodes.len();
        assert_eq!(len, self.edges.len());
        len
    }

    /// Returns `true` if the collection contains no entities.
    ///
    /// # Panics
    ///
    /// Panics if the internal invariant is violated (nodes and edges emptiness mismatch).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let is_empty = self.nodes.is_empty();
        assert_eq!(is_empty, self.edges.is_empty());
        is_empty
    }

    #[must_use]
    pub fn contains(&self, url: &Url) -> bool {
        self.urls.contains_key(url)
    }

    #[must_use]
    pub fn id(&self, url: &Url) -> Option<Id> {
        self.urls.get(url).map(|&idx| self.make_id(idx))
    }

    pub fn insert(&mut self, entity: Entity) -> Id {
        let index = self.len();
        self.nodes.push(entity);
        self.edges.push(Vec::new());
        let url = self.nodes[index].url().to_owned();
        self.urls.insert(url, index);
        self.make_id(index)
    }

    pub fn upsert(&mut self, other: Entity) -> Id {
        let Some(id) = self.id(other.url()) else {
            return self.insert(other);
        };
        let entity = &mut self.nodes[&id];
        entity.merge(other);
        id
    }

    pub fn add_edge(&mut self, from: &Id, to: &Id) {
        self.check_id(from);
        self.check_id(to);
        let from_edges = &mut self.edges[from];
        if from_edges.contains(&to.index) {
            return;
        }
        from_edges.push(to.index);
    }

    pub fn add_edges(&mut self, from: &Id, to: &Id) {
        self.add_edge(from, to);
        self.add_edge(to, from);
    }

    #[must_use]
    pub fn entity(&self, id: &Id) -> &Entity {
        self.check_id(id);
        &self.nodes[id]
    }

    pub fn entity_mut(&mut self, id: &Id) -> &mut Entity {
        self.check_id(id);
        &mut self.nodes[id]
    }

    #[must_use]
    pub fn edges(&self, id: &Id) -> Vec<Id> {
        self.check_id(id);
        self.edges[id]
            .iter()
            .map(|&idx| self.make_id(idx))
            .collect()
    }

    #[must_use]
    pub fn entities(&self) -> &[Entity] {
        &self.nodes
    }

    /// Updates entity labels according to the provided mappings.
    ///
    /// Replaces labels matching the mapping keys with their corresponding values.
    pub fn update_labels(&mut self, mappings: impl IntoIterator<Item = (String, String)>) {
        let mapping: BTreeMap<Label, Label> = mappings
            .into_iter()
            .map(|(k, v)| (Label::from(k), Label::from(v)))
            .collect();

        for node in &mut self.nodes {
            let labels = node.labels_mut();
            let to_add: BTreeSet<Label> = labels
                .iter()
                .filter_map(|label| mapping.get(label).cloned())
                .collect();
            labels.retain(|label| !mapping.contains_key(label));
            labels.extend(to_add);
        }
    }

    /// Creates a collection from a vector of Pinboard posts.
    ///
    /// Posts are sorted by time before being converted to entities.
    ///
    /// # Errors
    ///
    /// Returns an error if any post cannot be converted to a valid `Entity` (e.g., invalid URL or timestamp).
    pub fn from_posts(mut posts: Vec<Post>) -> Result<Collection, entity::Error> {
        posts.sort_by(|a, b| a.time.cmp(&b.time));
        let mut coll = Collection::with_capacity(posts.len());
        for post in posts {
            let entity = Entity::try_from(post)?;
            coll.insert(entity);
        }
        Ok(coll)
    }
}

impl Default for Collection {
    fn default() -> Collection {
        Collection::new()
    }
}

impl PartialEq for Collection {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes && self.edges == other.edges && self.urls == other.urls
    }
}

impl Eq for Collection {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct NodeRepr {
    id: u32,
    entity: Entity,
    edges: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectionRepr {
    version: Version,
    length: u32,
    value: Vec<NodeRepr>,
}

impl TryFrom<&Collection> for CollectionRepr {
    type Error = Error;

    fn try_from(coll: &Collection) -> Result<CollectionRepr, Error> {
        let version = Version::EXPECTED;

        let length = coll.len();

        let value: Vec<_> = (0..length)
            .map(|i| {
                let id = u32::try_from(i)?;
                let entity = coll.nodes[i].clone();
                let edges = coll.edges[i]
                    .iter()
                    .map(|&i| u32::try_from(i))
                    .collect::<Result<Vec<u32>, std::num::TryFromIntError>>()?;
                Ok(NodeRepr { id, entity, edges })
            })
            .collect::<Result<Vec<NodeRepr>, Error>>()?;

        let length = u32::try_from(length)?;

        Ok(CollectionRepr {
            version,
            length,
            value,
        })
    }
}

impl TryFrom<CollectionRepr> for Collection {
    type Error = Error;

    fn try_from(mut repr: CollectionRepr) -> Result<Self, Self::Error> {
        if !repr.version.matches_requirement()? {
            return Err(Error::IncompatibleVersion(
                repr.version.to_string(),
                Version::EXPECTED_REQ.to_string(),
            ));
        }

        let mut ret = Collection::with_capacity(usize::try_from(repr.length)?);

        repr.value.sort();

        for NodeRepr { id, entity, edges } in repr.value {
            assert_eq!(id, u32::try_from(ret.len())?);
            let url = entity.url().clone();
            ret.nodes.push(entity);
            ret.edges.push(
                edges
                    .into_iter()
                    .map(|e| usize::try_from(e))
                    .collect::<Result<Vec<usize>, std::num::TryFromIntError>>()?,
            );
            ret.urls.insert(url, usize::try_from(id)?);
        }

        Ok(ret)
    }
}

impl Serialize for Collection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CollectionRepr::try_from(self)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::entity::{Entity, Time, Url};

    use super::Collection;

    fn make_entity(url: &str) -> Entity {
        let url = Url::parse(url).unwrap();
        let now = Time::new(Utc::now());
        Entity::new(url, now, None, Default::default())
    }

    #[test]
    #[should_panic(expected = "Id belongs to a different collection")]
    fn check_id_wrong_collection() {
        let mut coll1 = Collection::new();
        let id1 = coll1.insert(make_entity("https://example.com/1"));

        let mut coll2 = Collection::new();
        coll2.insert(make_entity("https://example.com/2"));

        let _ = coll2.entity(&id1);
    }

    #[test]
    #[should_panic(expected = "Id's collection has been dropped")]
    fn check_id_dropped_collection() {
        let id = {
            let mut coll = Collection::new();
            coll.insert(make_entity("https://example.com/"))
        };

        let mut coll2 = Collection::new();
        coll2.insert(make_entity("https://example.com/2"));

        let _ = coll2.entity(&id);
    }
}
