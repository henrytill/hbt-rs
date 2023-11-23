use std::collections::HashMap;

use time::Date;
use url::Url;

/// A [`Id`] is a unique identifier for an [`Entity`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(usize);

impl Id {
    const fn new(id: usize) -> Self {
        Self(id)
    }
}

impl From<Id> for usize {
    fn from(handle: Id) -> Self {
        handle.0
    }
}

/// A [`Label`] is a label that can be attached to an [`Entity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(String);

impl Label {
    pub const fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Label {
    fn from(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl From<String> for Label {
    fn from(name: String) -> Self {
        Self(name)
    }
}

/// An [`Entity`] is a page in the collection.
#[derive(Debug, PartialEq, Eq)]
pub struct Entity {
    names: Vec<String>,
    url: Url,
    created_at: Date,
    updated_at: Vec<Date>,
    labels: Vec<Label>,
}

impl Entity {
    pub const fn new(names: Vec<String>, url: Url, created_at: Date, labels: Vec<Label>) -> Self {
        let updated_at = Vec::new();
        Self {
            names,
            url,
            created_at,
            updated_at,
            labels,
        }
    }

    pub fn update(&mut self, updated_at: Date, labels: Vec<Label>) {
        self.updated_at.push(updated_at);
        self.labels = labels;
    }

    pub fn names(&self) -> &[String] {
        &self.names
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

    pub fn labels(&self) -> &[Label] {
        &self.labels
    }
}

/// A collection of entities.
///
/// This is a graph structure where a nodes are represented by a vector of entities and edges are
/// represented by an adjacency list.
#[derive(Debug)]
pub struct Collection {
    nodes: Vec<Entity>,
    edges: Vec<Vec<Id>>,
    sites: HashMap<Url, Id>,
}

impl Collection {
    pub fn new() -> Self {
        let nodes = Vec::new();
        let edges = Vec::new();
        let sites = HashMap::new();
        Self {
            nodes,
            edges,
            sites,
        }
    }

    pub fn len(&self) -> usize {
        let len = self.nodes.len();
        assert_eq!(len, self.edges.len());
        len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn add_node(&mut self, entity: Entity) -> Id {
        let id = Id::new(self.len());
        self.nodes.push(entity);
        self.edges.push(Vec::new());
        let url = self.nodes[usize::from(id)].url().to_owned();
        self.sites.insert(url, id);
        id
    }

    pub fn add_edge(&mut self, from: Id, to: Id) {
        let idx = usize::from(from);
        if let Some(vec) = self.edges.get_mut(idx) {
            vec.push(to);
        } else {
            panic!("Index out of bounds");
        }
    }

    pub fn node(&self, idx: usize) -> Option<&Entity> {
        self.nodes.get(idx)
    }

    pub fn edges(&self, idx: usize) -> Option<&[Id]> {
        self.edges.get(idx).map(|vec| vec.as_slice())
    }
}

impl Default for Collection {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for Collection {
    type Item = Entity;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

impl std::ops::Index<usize> for Collection {
    type Output = Entity;

    fn index(&self, idx: usize) -> &Self::Output {
        if let Some(entity) = self.nodes.get(idx) {
            entity
        } else {
            panic!("Index out of bounds");
        }
    }
}
