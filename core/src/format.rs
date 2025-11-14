use std::{
    io::{self, BufRead, Write},
    path::Path,
};

#[cfg(feature = "clap")]
use clap::{ValueEnum, builder::PossibleValue};

use strum::{IntoStaticStr, VariantArray};

use hbt_pinboard::{self, Post};

use crate::{collection::Collection, entity, html, markdown};

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Entity(#[from] entity::Error),

    #[error(transparent)]
    Html(#[from] html::Error),

    #[error(transparent)]
    Markdown(#[from] markdown::Error),

    #[error(transparent)]
    Pinboard(#[from] hbt_pinboard::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr, VariantArray)]
#[strum(serialize_all = "lowercase")]
pub enum InputFormat {
    Json,
    Xml,
    #[strum(serialize = "md")]
    Markdown,
    Html,
}

impl InputFormat {
    pub fn detect(path: impl AsRef<Path>) -> Option<InputFormat> {
        match path.as_ref().extension()?.to_str()? {
            "json" => Some(InputFormat::Json),
            "xml" => Some(InputFormat::Xml),
            "md" => Some(InputFormat::Markdown),
            "html" => Some(InputFormat::Html),
            _ => None,
        }
    }

    pub fn parse(&self, reader: &mut impl BufRead) -> Result<Collection, ParseError> {
        match self {
            InputFormat::Json => {
                let posts = Post::from_json(reader)?;
                Collection::from_posts(posts).map_err(Into::into)
            }
            InputFormat::Xml => {
                let posts = Post::from_xml(reader)?;
                Collection::from_posts(posts).map_err(Into::into)
            }
            InputFormat::Markdown => {
                let mut buf = String::new();
                reader.read_to_string(&mut buf)?;
                Collection::from_markdown(&buf).map_err(Into::into)
            }
            InputFormat::Html => {
                let mut buf = String::new();
                reader.read_to_string(&mut buf)?;
                Collection::from_html(&buf).map_err(Into::into)
            }
        }
    }
}

#[cfg(feature = "clap")]
impl ValueEnum for InputFormat {
    fn value_variants<'a>() -> &'a [InputFormat] {
        InputFormat::VARIANTS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let s: &'static str = self.into();
        Some(PossibleValue::new(s))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UnparseError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Html(#[from] html::Error),

    #[error(transparent)]
    Yaml(#[from] serde_norway::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr, VariantArray)]
#[strum(serialize_all = "lowercase")]
pub enum OutputFormat {
    Html,
    Yaml,
}

impl OutputFormat {
    pub fn detect(path: impl AsRef<Path>) -> Option<OutputFormat> {
        match path.as_ref().extension()?.to_str()? {
            "html" => Some(OutputFormat::Html),
            "yaml" | "yml" => Some(OutputFormat::Yaml),
            _ => None,
        }
    }

    pub fn unparse(&self, writer: &mut impl Write, coll: &Collection) -> Result<(), UnparseError> {
        match self {
            OutputFormat::Html => coll.to_html(writer)?,
            OutputFormat::Yaml => serde_norway::to_writer(writer, coll)?,
        }
        Ok(())
    }
}

#[cfg(feature = "clap")]
impl ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [OutputFormat] {
        OutputFormat::VARIANTS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let s: &'static str = self.into();
        Some(PossibleValue::new(s))
    }
}
