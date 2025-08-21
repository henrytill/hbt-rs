#[cfg(test)]
mod tests;

use std::collections::{HashSet, hash_set::Iter};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTML selector error: {0}")]
    HtmlSelector(String),
    #[error("XML attribute error: {0}")]
    XmlAttribute(#[from] quick_xml::events::attributes::AttrError),
    #[error("XML parsing error: {0}")]
    ParseXml(#[from] quick_xml::Error),
    #[error("invalid UTF-8: {0}")]
    ParseUtf8(#[from] std::string::FromUtf8Error),
    #[error("JSON parsing error: {0}")]
    ParseJson(#[from] serde_json::Error),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Tags<'a>(HashSet<&'a str>);

impl Tags<'_> {
    pub fn contains(&self, value: impl AsRef<str>) -> bool {
        self.0.contains(value.as_ref())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, &str> {
        self.0.iter()
    }
}

#[cfg(test)]
impl<'a> From<&'a [String]> for Tags<'a> {
    fn from(tags: &'a [String]) -> Tags<'a> {
        let inner = tags.iter().map(String::as_str).collect();
        Tags(inner)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Post {
    pub href: String,
    pub time: String,
    #[serde(deserialize_with = "json::deserialize_empty_string")]
    pub description: Option<String>,
    #[serde(deserialize_with = "json::deserialize_empty_string")]
    pub extended: Option<String>,
    #[serde(deserialize_with = "json::deserialize_tags", default)]
    pub tags: Vec<String>,
    #[serde(deserialize_with = "json::deserialize_empty_string")]
    pub hash: Option<String>,
    #[serde(deserialize_with = "json::deserialize_yes_no")]
    pub shared: bool,
    #[serde(deserialize_with = "json::deserialize_yes_no")]
    pub toread: bool,
}

impl<'a> From<&'a [Post]> for Tags<'a> {
    fn from(posts: &'a [Post]) -> Tags<'a> {
        let mut inner = HashSet::new();
        for post in posts {
            inner.extend(post.tags.iter().map(String::as_str));
        }
        Tags(inner)
    }
}

impl Post {
    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    const fn new(
        href: String,
        time: String,
        description: Option<String>,
        extended: Option<String>,
        tags: Vec<String>,
        hash: Option<String>,
        shared: bool,
        toread: bool,
    ) -> Post {
        Post { href, time, description, extended, tags, hash, shared, toread }
    }

    pub fn from_json(input: &str) -> Result<Vec<Post>, Error> {
        serde_json::from_str(input).map_err(Into::into)
    }

    pub fn from_xml(input: &str) -> Result<Vec<Post>, Error> {
        xml::parse(input)
    }
}

mod json {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize_empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
    }

    pub fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
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

    pub fn deserialize_yes_no<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        const YES: &str = "yes";
        const NO: &str = "no";

        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            YES => Ok(true),
            NO => Ok(false),
            _ => Err(serde::de::Error::custom(format!("expected '{YES}' or '{NO}'"))),
        }
    }
}

mod xml {
    use quick_xml::{
        events::{
            Event,
            attributes::{Attribute, Attributes},
        },
        reader::Reader,
    };

    use super::{Error, Post};

    pub fn extract_post(attrs: Attributes) -> Result<Post, Error> {
        const KEY_HREF: &[u8] = b"href";
        const KEY_TIME: &[u8] = b"time";
        const KEY_DESCRIPTION: &[u8] = b"description";
        const KEY_EXTENDED: &[u8] = b"extended";
        const KEY_TAG: &[u8] = b"tag";
        const KEY_HASH: &[u8] = b"hash";
        const KEY_SHARED: &[u8] = b"shared";
        const KEY_TOREAD: &[u8] = b"toread";
        const YES: &[u8] = b"yes";

        let mut ret = Post::default();

        for attr in attrs {
            let Attribute { key, value } = attr?;
            match key.local_name().as_ref() {
                KEY_HREF => ret.href = String::from_utf8(value.into_owned())?,
                KEY_TIME => ret.time = String::from_utf8(value.into_owned())?,
                KEY_DESCRIPTION => {
                    ret.description = if value.is_empty() {
                        None
                    } else {
                        let s = String::from_utf8(value.into_owned())?;
                        Some(s)
                    };
                }
                KEY_EXTENDED => {
                    ret.extended = if value.is_empty() {
                        None
                    } else {
                        let s = String::from_utf8(value.into_owned())?;
                        Some(s)
                    };
                }
                KEY_TAG => {
                    ret.tags = if value.is_empty() {
                        Vec::new()
                    } else {
                        let s = String::from_utf8(value.into_owned())?;
                        s.split_whitespace().map(ToOwned::to_owned).collect()
                    }
                }
                KEY_HASH => {
                    ret.hash = if value.is_empty() {
                        None
                    } else {
                        let s = String::from_utf8(value.into_owned())?;
                        Some(s)
                    };
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

    #[inline]
    pub fn parse(input: &str) -> Result<Vec<Post>, Error> {
        const EVENT_POSTS: &[u8] = b"posts";
        const EVENT_POST: &[u8] = b"post";

        let mut ret = Vec::new();
        let mut reader = Reader::from_str(input);
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event()? {
                Event::Start(e) if e.name().as_ref() == EVENT_POSTS => {
                    // nothing at the moment
                }
                Event::Empty(e) if e.name().as_ref() == EVENT_POST => {
                    let pinboard = extract_post(e.attributes())?;
                    ret.push(pinboard);
                }
                Event::Eof => break,
                _ => (),
            }
        }

        Ok(ret)
    }
}
