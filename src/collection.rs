use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};

use serde::Serialize;
use time::Date;
use url::Url;

/// An [`Id`] is a unique identifier for an [`Entity`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

/// A [`Label`] is a label that can be attached to an [`Entity`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Label(String);

impl Label {
    pub const fn new(name: String) -> Label {
        Label(name)
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
    fn from(name: String) -> Label {
        Label(name)
    }
}

#[cfg(test)]
impl From<&str> for Label {
    fn from(name: &str) -> Label {
        Label(name.into())
    }
}

/// An [`Entity`] is a page in the collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entity {
    url: Url,
    created_at: Date,
    updated_at: Vec<Date>,
    names: HashSet<Name>,
    labels: HashSet<Label>,
}

impl Entity {
    pub fn new(
        url: Url,
        created_at: Date,
        maybe_name: Option<Name>,
        labels: HashSet<Label>,
    ) -> Entity {
        let updated_at = Vec::new();
        let names = maybe_name.into_iter().collect();
        Entity { url, created_at, updated_at, names, labels }
    }

    pub fn update(
        &mut self,
        updated_at: Date,
        names: HashSet<Name>,
        labels: HashSet<Label>,
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

    pub fn created_at(&self) -> &Date {
        &self.created_at
    }

    pub fn updated_at(&self) -> &[Date] {
        &self.updated_at
    }

    pub fn names(&self) -> &HashSet<Name> {
        &self.names
    }

    pub fn labels(&self) -> &HashSet<Label> {
        &self.labels
    }
}

pub type Edges = Vec<Id>;

/// A collection of entities.
///
/// This is a graph structure where a nodes are represented by a vector of entities and edges are
/// represented by an adjacency list.
#[derive(Debug)]
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
