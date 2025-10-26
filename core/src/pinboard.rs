use std::io::BufRead;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    collection::Collection,
    entity::{self, Entity},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("XML attribute error: {0}")]
    XmlAttribute(#[from] quick_xml::events::attributes::AttrError),

    #[error("XML parsing error: {0}")]
    ParseXml(#[from] quick_xml::Error),

    #[error("invalid UTF-8: {0}")]
    ParseUtf8(#[from] std::string::FromUtf8Error),

    #[error("JSON parsing error: {0}")]
    ParseJson(#[from] serde_json::Error),
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Post {
    pub href: String,

    pub time: String,

    #[serde(deserialize_with = "json::empty_string")]
    pub description: Option<String>,

    #[serde(deserialize_with = "json::empty_string")]
    pub extended: Option<String>,

    #[serde(deserialize_with = "json::tags", default)]
    pub tags: Vec<String>,

    #[serde(deserialize_with = "json::empty_string")]
    pub meta: Option<String>,

    #[serde(deserialize_with = "json::empty_string")]
    pub hash: Option<String>,

    #[serde(deserialize_with = "json::yes_no")]
    pub shared: bool,

    #[serde(deserialize_with = "json::yes_no")]
    pub toread: bool,
}

impl Post {
    pub fn from_json(input: &mut impl BufRead) -> Result<Vec<Post>, Error> {
        serde_json::from_reader(input).map_err(Into::into)
    }
}

impl Collection {
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

mod json {
    use serde::{Deserialize, Deserializer};

    pub fn empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
    }

    pub fn tags<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(s.split_whitespace().map(ToOwned::to_owned).collect())
        }
    }

    pub fn yes_no<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        const YES: &str = "yes";
        const NO: &str = "no";

        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            YES => Ok(true),
            NO => Ok(false),
            _ => Err(serde::de::Error::custom(format!(
                "expected '{YES}' or '{NO}'"
            ))),
        }
    }
}

pub mod xml {
    use std::io::BufRead;

    use quick_xml::{
        events::{Event, attributes::Attributes},
        reader::Reader,
    };

    use super::{Error, Post};

    const KEY_HREF: &[u8] = b"href";
    const KEY_TIME: &[u8] = b"time";
    const KEY_DESCRIPTION: &[u8] = b"description";
    const KEY_EXTENDED: &[u8] = b"extended";
    const KEY_TAG: &[u8] = b"tag";
    const KEY_META: &[u8] = b"meta";
    const KEY_HASH: &[u8] = b"hash";
    const KEY_SHARED: &[u8] = b"shared";
    const KEY_TOREAD: &[u8] = b"toread";

    const YES: &str = "yes";

    const EVENT_POSTS: &[u8] = b"posts";
    const EVENT_POST: &[u8] = b"post";

    impl Post {
        fn from_attrs(attrs: Attributes) -> Result<Post, Error> {
            let mut ret = Post::default();

            for result in attrs {
                let attr = result?;
                let key = attr.key;
                let value = attr.unescape_value()?;
                match key.local_name().as_ref() {
                    KEY_HREF => {
                        ret.href = value.into_owned();
                    }
                    KEY_TIME => {
                        ret.time = value.into_owned();
                    }
                    KEY_DESCRIPTION if !value.is_empty() => {
                        ret.description = Some(value.into_owned());
                    }
                    KEY_EXTENDED if !value.is_empty() => {
                        ret.extended = Some(value.into_owned());
                    }
                    KEY_TAG if !value.is_empty() => {
                        ret.tags = value
                            .into_owned()
                            .split_whitespace()
                            .map(ToOwned::to_owned)
                            .collect();
                    }
                    KEY_META if !value.is_empty() => {
                        ret.meta = Some(value.into_owned());
                    }
                    KEY_HASH if !value.is_empty() => {
                        ret.hash = Some(value.into_owned());
                    }
                    KEY_SHARED => {
                        ret.shared = value.as_ref() == YES;
                    }
                    KEY_TOREAD => {
                        ret.toread = value.as_ref() == YES;
                    }
                    _ => (),
                }
            }

            Ok(ret)
        }

        pub fn from_xml(reader: &mut impl BufRead) -> Result<Vec<Post>, Error> {
            let mut ret = Vec::new();
            let mut reader = Reader::from_reader(reader);
            reader.config_mut().trim_text(true);

            let mut buf = Vec::new();

            loop {
                match reader.read_event_into(&mut buf)? {
                    Event::Start(e) if e.name().as_ref() == EVENT_POSTS => {
                        // nothing at the moment
                    }
                    Event::Empty(e) if e.name().as_ref() == EVENT_POST => {
                        let post = Post::from_attrs(e.attributes())?;
                        ret.push(post);
                    }
                    Event::Eof => break,
                    _ => (),
                }
            }

            Ok(ret)
        }
    }
}
