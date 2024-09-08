#[cfg(test)]
mod tests;

use std::collections::{hash_set::Iter, HashSet};

use anyhow::Error;
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq)]
pub struct Tags(HashSet<String>);

impl Tags {
    #[cfg(test)]
    const fn new(inner: HashSet<String>) -> Tags {
        Tags(inner)
    }

    pub fn contains(&self, value: impl AsRef<str>) -> bool {
        self.0.contains(value.as_ref())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, String> {
        self.0.iter()
    }
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Post {
    href: String,
    time: String,
    #[serde(deserialize_with = "json::deserialize_empty_string")]
    description: Option<String>,
    #[serde(deserialize_with = "json::deserialize_empty_string")]
    extended: Option<String>,
    #[serde(deserialize_with = "json::deserialize_tags", default)]
    tags: Vec<String>,
    #[serde(deserialize_with = "json::deserialize_empty_string")]
    hash: Option<String>,
    #[serde(deserialize_with = "json::deserialize_yes_no")]
    shared: bool,
    #[serde(deserialize_with = "json::deserialize_yes_no")]
    toread: bool,
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

    pub fn from_html(input: &str) -> Result<Vec<Post>, Error> {
        html::parse(input)
    }

    pub fn from_json(input: &str) -> Result<Vec<Post>, Error> {
        serde_json::from_str(input).map_err(Into::into)
    }

    pub fn from_xml(input: &str) -> Result<Vec<Post>, Error> {
        xml::parse(input)
    }

    pub fn tags(ps: &[Post]) -> Tags {
        let mut inner = HashSet::new();
        for p in ps {
            inner.extend(p.tags.iter().cloned());
        }
        Tags(inner)
    }

    pub fn href(&self) -> &String {
        &self.href
    }
}

mod html {
    use anyhow::Error;
    use scraper::{Element, Html, Selector};

    use super::Post;

    #[inline]
    pub fn parse(input: &str) -> Result<Vec<Post>, Error> {
        let document = Html::parse_document(input);
        let dt_selector = Selector::parse("dt").unwrap();

        let posts = document
            .select(&dt_selector)
            .filter_map(|dt_element| {
                // NOTE: These question marks are for Option
                let a_selector = Selector::parse("a").ok()?;
                let a_element = dt_element.select(&a_selector).next()?;

                let href = a_element.value().attr("href")?.to_string();
                let add_date = a_element.value().attr("add_date")?;
                let private = a_element.value().attr("private")?;
                let toread = a_element.value().attr("toread")?;
                let tags = a_element.value().attr("tags")?;
                let description = a_element.text().collect::<String>();
                let time = add_date.parse().ok()?;
                let shared = private == "0";
                let toread = toread == "1";
                let tags = tags.split(',').map(ToString::to_string).collect();

                let mut post = Post {
                    href,
                    time,
                    description: Some(description),
                    extended: None,
                    tags,
                    hash: None,
                    shared,
                    toread,
                };

                if let Some(dd_element) = dt_element.next_sibling_element() {
                    if dd_element.value().name() == "dd" {
                        let extended_text = dd_element.text().collect::<String>();
                        post.extended = Some(extended_text.trim().to_string());
                    }
                }

                Some(post)
            })
            .collect();

        Ok(posts)
    }
}

mod json {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize_empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
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
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "yes" => Ok(true),
            "no" => Ok(false),
            _ => Err(serde::de::Error::custom("expected 'yes' or 'no'")),
        }
    }
}

mod xml {
    use anyhow::Error;
    use quick_xml::{
        events::{
            attributes::{Attribute, Attributes},
            Event,
        },
        reader::Reader,
    };

    use super::Post;

    pub fn extract_post(attrs: Attributes) -> Result<Post, Error> {
        let mut ret = Post::default();

        for attr in attrs {
            let Attribute { key, value } = attr?;
            match key.local_name().as_ref() {
                b"href" => ret.href = String::from_utf8(value.into_owned())?,
                b"time" => ret.time = String::from_utf8(value.into_owned())?,
                b"description" => {
                    ret.description = if value.is_empty() {
                        None
                    } else {
                        let s = String::from_utf8(value.into_owned())?;
                        Some(s)
                    };
                }
                b"extended" => {
                    ret.extended = if value.is_empty() {
                        None
                    } else {
                        let s = String::from_utf8(value.into_owned())?;
                        Some(s)
                    };
                }
                b"tag" => {
                    ret.tags = if value.is_empty() {
                        Vec::new()
                    } else {
                        let s = String::from_utf8(value.into_owned())?;
                        s.split_whitespace().map(ToOwned::to_owned).collect()
                    }
                }
                b"hash" => {
                    ret.hash = if value.is_empty() {
                        None
                    } else {
                        let s = String::from_utf8(value.into_owned())?;
                        Some(s)
                    };
                }
                b"shared" => {
                    ret.shared = value.as_ref() == b"yes";
                }
                b"toread" => {
                    ret.toread = value.as_ref() == b"yes";
                }
                _ => (),
            }
        }

        Ok(ret)
    }

    #[inline]
    pub fn parse(input: &str) -> Result<Vec<Post>, Error> {
        let mut ret = Vec::new();
        let mut reader = Reader::from_str(input);
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event()? {
                Event::Start(e) if e.name().as_ref() == b"posts" => {
                    // nothing at the moment
                }
                Event::Empty(e) if e.name().as_ref() == b"post" => {
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
