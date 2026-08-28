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

    #[error("declared length {declared} does not match node count {actual}")]
    LengthMismatch { declared: u32, actual: usize },

    #[error("node at position {position} has id {id}, expected {position}")]
    UnexpectedId { position: usize, id: u32 },

    #[error("node {node}: edge index {edge} is out of bounds for {length} nodes")]
    EdgeOutOfBounds {
        node: usize,
        edge: usize,
        length: usize,
    },

    #[error("nodes {first} and {second} share a URL")]
    DuplicateUrl { first: usize, second: usize },
}

#[derive(Debug, Clone)]
pub struct Id {
    index: usize,
    owner: Weak<()>,
}

impl PartialEq for Id {
    fn eq(&self, other: &Id) -> bool {
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

    fn index(&self, id: &Id) -> &Entity {
        &self[id.index]
    }
}

impl IndexMut<&Id> for Vec<Entity> {
    fn index_mut(&mut self, id: &Id) -> &mut Entity {
        &mut self[id.index]
    }
}

impl Index<&Id> for Vec<Edges> {
    type Output = Edges;

    fn index(&self, id: &Id) -> &Edges {
        &self[id.index]
    }
}

impl IndexMut<&Id> for Vec<Edges> {
    fn index_mut(&mut self, id: &Id) -> &mut Edges {
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
    /// Posts are ordered by creation time, earliest first, so that an entity keeps the earliest
    /// timestamp for a URL and later ones are recorded as updates. Posts sharing a URL are merged
    /// into one entity.
    ///
    /// # Errors
    ///
    /// Returns an error if any post cannot be converted to a valid `Entity` (e.g., invalid URL or timestamp).
    pub fn from_posts(posts: Vec<Post>) -> Result<Collection, entity::Error> {
        let mut entities = posts
            .into_iter()
            .map(Entity::try_from)
            .collect::<Result<Vec<Entity>, entity::Error>>()?;

        // Sort on the parsed timestamp rather than the raw `time` string: a post carries either an
        // ISO 8601 date or a Unix timestamp, and those do not share a lexicographic order. The sort
        // is stable, so posts with equal timestamps keep their input order.
        entities.sort_by_key(Entity::created_at);

        let mut coll = Collection::with_capacity(entities.len());
        for entity in entities {
            // upsert, not insert: an export may list the same href twice, and two nodes for one URL
            // leave `urls` pointing at only the last of them.
            coll.upsert(entity);
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
    fn eq(&self, other: &Collection) -> bool {
        self.nodes == other.nodes && self.edges == other.edges && self.urls == other.urls
    }
}

impl Eq for Collection {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct NodeRepr {
    id: u32,
    entity: Entity,
    #[schemars(extend("uniqueItems" = true))]
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

    fn try_from(mut repr: CollectionRepr) -> Result<Collection, Error> {
        if !repr.version.matches_requirement()? {
            return Err(Error::IncompatibleVersion(
                repr.version.to_string(),
                Version::EXPECTED_REQ.to_string(),
            ));
        }

        repr.value.sort();

        // Nothing downstream re-checks these, so a collection that gets past here is trusted: an
        // out-of-range edge index would later hand out an Id that panics on lookup, and a repeated
        // URL would leave an entity unreachable through `urls`.
        let length = repr.value.len();

        if usize::try_from(repr.length)? != length {
            return Err(Error::LengthMismatch {
                declared: repr.length,
                actual: length,
            });
        }

        let mut ret = Collection::with_capacity(length);

        for (position, NodeRepr { id, entity, edges }) in repr.value.into_iter().enumerate() {
            // The ids are sorted, so requiring id == position rejects gaps, out-of-range ids and
            // duplicates in one comparison. This used to be an assert_eq!, i.e. a panic.
            if usize::try_from(id)? != position {
                return Err(Error::UnexpectedId { position, id });
            }

            let edges = edges
                .into_iter()
                .map(usize::try_from)
                .collect::<Result<Vec<usize>, std::num::TryFromIntError>>()?;

            for &edge in &edges {
                if edge >= length {
                    return Err(Error::EdgeOutOfBounds {
                        node: position,
                        edge,
                        length,
                    });
                }
            }

            let url = entity.url().clone();
            if let Some(first) = ret.urls.insert(url, position) {
                return Err(Error::DuplicateUrl {
                    first,
                    second: position,
                });
            }

            ret.nodes.push(entity);
            ret.edges.push(edges);
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
    use std::collections::BTreeSet;

    use hbt_pinboard::Post;

    use crate::entity::{CreatedAt, Entity, Label, Time, Url};

    use super::Collection;

    fn make_entity(url: &str) -> Entity {
        let url = Url::parse(url).unwrap();
        // A whole-second time, since the wire format carries Unix seconds and a `Utc::now()` would
        // not survive a round trip.
        let time = Time::parse_timestamp("1700000000").unwrap();
        Entity::new(url, time, None, BTreeSet::default())
    }

    fn post(href: &str, time: &str, description: &str, tags: &[&str]) -> Post {
        Post {
            href: href.to_string(),
            time: time.to_string(),
            description: Some(description.to_string()),
            tags: tags.iter().map(ToString::to_string).collect(),
            ..Post::default()
        }
    }

    /// `from_posts` used to `insert`, so an export listing the same href twice produced two nodes
    /// for one URL, with `urls` indexing only the last of them.
    #[test]
    fn from_posts_merges_duplicate_urls() {
        let posts = vec![
            post(
                "https://example.com/",
                "2024-02-01T00:00:00Z",
                "second",
                &["b"],
            ),
            post(
                "https://example.com/",
                "2024-01-01T00:00:00Z",
                "first",
                &["a"],
            ),
        ];

        let coll = Collection::from_posts(posts).unwrap();

        assert_eq!(coll.len(), 1);
        let id = coll
            .id(&Url::parse("https://example.com/").unwrap())
            .unwrap();
        let entity = coll.entity(&id);
        // The earliest post keeps created_at, whichever order the posts arrived in.
        let earliest = CreatedAt::new(Time::parse_flexible("2024-01-01T00:00:00Z").unwrap());
        assert_eq!(entity.created_at(), earliest);
        // Both posts' labels survive the merge.
        assert_eq!(
            entity
                .labels()
                .iter()
                .map(Label::as_str)
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    /// Every entity must be reachable through the `urls` index by its own URL.
    #[test]
    fn from_posts_leaves_every_entity_indexed() {
        let posts = vec![
            post(
                "https://example.com/",
                "2024-02-01T00:00:00Z",
                "second",
                &[],
            ),
            post("https://example.com/", "2024-01-01T00:00:00Z", "first", &[]),
            post("https://other.test/", "2024-01-01T00:00:00Z", "other", &[]),
        ];

        let coll = Collection::from_posts(posts).unwrap();

        assert_eq!(coll.len(), 2);
        for entity in coll.entities() {
            let id = coll
                .id(entity.url())
                .expect("entity missing from urls index");
            assert_eq!(coll.entity(&id).url(), entity.url());
        }
    }

    /// Ordering must come from the parsed timestamp: a Unix timestamp and an ISO 8601 date do not
    /// sort lexicographically against each other.
    #[test]
    fn from_posts_orders_by_parsed_time() {
        let posts = vec![
            post("https://b.test/", "2024-01-01T00:00:00Z", "b", &[]),
            post("https://a.test/", "1000000000", "a", &[]),
        ];

        let coll = Collection::from_posts(posts).unwrap();

        let urls: Vec<&Url> = coll.entities().iter().map(Entity::url).collect();
        let expected = [
            Url::parse("https://a.test/").unwrap(),
            Url::parse("https://b.test/").unwrap(),
        ];
        assert_eq!(urls, expected.iter().collect::<Vec<_>>());
    }

    fn entity_with_labels(url: &str, labels: &[&str]) -> Entity {
        let url = Url::parse(url).unwrap();
        let time = Time::parse_timestamp("1700000000").unwrap();
        let labels = labels.iter().copied().map(Label::from).collect();
        Entity::new(url, time, None, labels)
    }

    fn labels_of(coll: &Collection, index: usize) -> Vec<&str> {
        coll.entities()[index]
            .labels()
            .iter()
            .map(Label::as_str)
            .collect()
    }

    #[test]
    fn update_labels_rewrites_mapped_labels() {
        let mut coll = Collection::new();
        coll.insert(entity_with_labels("https://a.test/", &["old", "kept"]));

        coll.update_labels([("old".to_string(), "new".to_string())]);

        assert_eq!(labels_of(&coll, 0), vec!["kept", "new"]);
    }

    /// Two labels mapping onto one name collapse into a single label, since labels are a set.
    #[test]
    fn update_labels_collapses_labels_onto_one_name() {
        let mut coll = Collection::new();
        coll.insert(entity_with_labels("https://a.test/", &["js", "javascript"]));

        coll.update_labels([
            ("js".to_string(), "JavaScript".to_string()),
            ("javascript".to_string(), "JavaScript".to_string()),
        ]);

        assert_eq!(labels_of(&coll, 0), vec!["JavaScript"]);
    }

    /// A mapping whose target is another mapping's source must not be applied twice in one pass.
    #[test]
    fn update_labels_does_not_chain_mappings() {
        let mut coll = Collection::new();
        coll.insert(entity_with_labels("https://a.test/", &["a"]));

        coll.update_labels([
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
        ]);

        assert_eq!(labels_of(&coll, 0), vec!["b"]);
    }

    #[test]
    fn update_labels_leaves_unmapped_labels_alone() {
        let mut coll = Collection::new();
        coll.insert(entity_with_labels("https://a.test/", &["x", "y"]));

        coll.update_labels([("z".to_string(), "w".to_string())]);

        assert_eq!(labels_of(&coll, 0), vec!["x", "y"]);
    }

    #[test]
    fn upsert_returns_the_existing_id_and_merges() {
        let mut coll = Collection::new();
        let first = coll.upsert(entity_with_labels("https://a.test/", &["a"]));
        let second = coll.upsert(entity_with_labels("https://a.test/", &["b"]));

        assert_eq!(first, second);
        assert_eq!(coll.len(), 1);
        assert_eq!(labels_of(&coll, 0), vec!["a", "b"]);
    }

    #[test]
    fn add_edge_is_idempotent_and_add_edges_is_symmetric() {
        let mut coll = Collection::new();
        let a = coll.insert(make_entity("https://a.test/"));
        let b = coll.insert(make_entity("https://b.test/"));

        coll.add_edges(&a, &b);
        coll.add_edges(&a, &b);

        assert_eq!(coll.edges(&a), vec![b.clone()]);
        assert_eq!(coll.edges(&b), vec![a]);
    }

    fn node_yaml(id: u32, uri: &str, edges: &str) -> String {
        format!(
            concat!(
                "- id: {id}\n",
                "  entity:\n",
                "    uri: {uri}\n",
                "    createdAt: 0\n",
                "    updatedAt: []\n",
                "    names: []\n",
                "    labels: []\n",
                "    shared: null\n",
                "    toRead: null\n",
                "    isFeed: null\n",
                "    extended: []\n",
                "  edges: {edges}\n",
            ),
            id = id,
            uri = uri,
            edges = edges
        )
    }

    fn collection_yaml(length: u32, nodes: &[String]) -> String {
        format!(
            "version: 0.1.0\nlength: {length}\nvalue:\n{}",
            nodes.concat()
        )
    }

    fn load(yaml: &str) -> Result<Collection, serde_norway::Error> {
        serde_norway::from_str(yaml)
    }

    #[test]
    fn deserialize_round_trips_entities_and_edges() {
        let mut coll = Collection::new();
        let a = coll.insert(make_entity("https://a.test/"));
        let b = coll.insert(make_entity("https://b.test/"));
        coll.add_edges(&a, &b);

        let yaml = serde_norway::to_string(&coll).unwrap();
        let back: Collection = serde_norway::from_str(&yaml).unwrap();

        assert_eq!(back, coll);
        assert_eq!(back.len(), 2);
        let a = back.id(&Url::parse("https://a.test/").unwrap()).unwrap();
        assert_eq!(back.edges(&a).len(), 1);
    }

    /// Used to be `assert_eq!(id, ...)`, i.e. a panic on malformed input.
    #[test]
    fn deserialize_rejects_unexpected_id() {
        let yaml = collection_yaml(1, &[node_yaml(7, "https://a.test/", "[]")]);
        let err = load(&yaml).unwrap_err().to_string();
        assert!(err.contains("has id 7, expected 0"), "{err}");
    }

    /// A duplicate id also arrives here: sorted, the second one cannot sit at its own position.
    #[test]
    fn deserialize_rejects_duplicate_id() {
        let yaml = collection_yaml(
            2,
            &[
                node_yaml(0, "https://a.test/", "[]"),
                node_yaml(0, "https://b.test/", "[]"),
            ],
        );
        let err = load(&yaml).unwrap_err().to_string();
        assert!(err.contains("has id 0, expected 1"), "{err}");
    }

    /// Was accepted silently, producing a graph whose edges index nodes that do not exist.
    #[test]
    fn deserialize_rejects_out_of_bounds_edge() {
        let yaml = collection_yaml(1, &[node_yaml(0, "https://a.test/", "[99]")]);
        let err = load(&yaml).unwrap_err().to_string();
        assert!(err.contains("edge index 99 is out of bounds"), "{err}");
    }

    /// `length` was read but never compared against the actual node count.
    #[test]
    fn deserialize_rejects_length_mismatch() {
        let yaml = collection_yaml(5, &[node_yaml(0, "https://a.test/", "[]")]);
        let err = load(&yaml).unwrap_err().to_string();
        assert!(
            err.contains("declared length 5 does not match node count 1"),
            "{err}"
        );
    }

    /// Two nodes for one URL left the first unreachable through the `urls` index.
    #[test]
    fn deserialize_rejects_duplicate_url() {
        let yaml = collection_yaml(
            2,
            &[
                node_yaml(0, "https://a.test/", "[]"),
                node_yaml(1, "https://a.test/", "[]"),
            ],
        );
        let err = load(&yaml).unwrap_err().to_string();
        assert!(err.contains("share a URL"), "{err}");
    }

    /// A URI is intrinsic to an entity. Unlike the Go and OCaml implementations, which had to add
    /// this check, `Url` cannot hold an empty value, so serde rejects it while building the entity.
    #[test]
    fn deserialize_rejects_missing_uri() {
        let yaml = collection_yaml(1, &[node_yaml(0, r#""""#, "[]")]);
        assert!(load(&yaml).is_err());
    }

    #[test]
    fn deserialize_rejects_incompatible_version() {
        let yaml = "version: 9.9.9\nlength: 0\nvalue: []\n";
        let err = load(yaml).unwrap_err().to_string();
        assert!(err.contains("incompatible version: 9.9.9"), "{err}");
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
