use std::{
    io::{self, BufRead, Write},
    path::Path,
};

#[cfg(feature = "clap")]
use clap::{ValueEnum, builder::PossibleValue};

use crate::{
    collection::Collection,
    entity, html, markdown,
    pinboard::{self, Post},
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum FormatKind {
    Json,
    Xml,
    Markdown,
    Html,
    Yaml,
}

impl std::fmt::Display for FormatKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatKind::Json => write!(f, "json"),
            FormatKind::Xml => write!(f, "xml"),
            FormatKind::Markdown => write!(f, "md"),
            FormatKind::Html => write!(f, "html"),
            FormatKind::Yaml => write!(f, "yaml"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Format<const CAPS: u8>(FormatKind);

pub const INPUT: u8 = 0b01;
pub const OUTPUT: u8 = 0b10;

impl<const CAPS: u8> Format<CAPS> {
    const fn is_input() -> bool {
        CAPS & INPUT != 0
    }

    const fn is_output() -> bool {
        CAPS & OUTPUT != 0
    }

    const fn as_input(self) -> Format<INPUT> {
        assert!(Format::<CAPS>::is_input());
        unsafe { std::mem::transmute(self) }
    }

    const fn as_output(self) -> Format<OUTPUT> {
        assert!(Format::<CAPS>::is_output());
        unsafe { std::mem::transmute(self) }
    }
}

impl Format<INPUT> {
    const JSON: Self = Format(FormatKind::Json);
    const XML: Self = Format(FormatKind::Xml);
    const MARKDOWN: Self = Format(FormatKind::Markdown);
}

impl Format<OUTPUT> {
    const YAML: Self = Format(FormatKind::Yaml);
}

impl Format<{ INPUT | OUTPUT }> {
    const HTML: Self = Format(FormatKind::Html);
}

pub const ALL_INPUT_FORMATS: &[Format<INPUT>] = &[
    Format::<INPUT>::JSON,
    Format::<INPUT>::XML,
    Format::<INPUT>::MARKDOWN,
    Format::<{ INPUT | OUTPUT }>::HTML.as_input(),
];

pub const ALL_OUTPUT_FORMATS: &[Format<OUTPUT>] = &[
    Format::<{ INPUT | OUTPUT }>::HTML.as_output(),
    Format::<OUTPUT>::YAML,
];

#[cfg(feature = "clap")]
impl ValueEnum for Format<INPUT> {
    fn value_variants<'a>() -> &'a [Self] {
        ALL_INPUT_FORMATS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self.0 {
            FormatKind::Json | FormatKind::Xml | FormatKind::Markdown | FormatKind::Html => {
                Some(PossibleValue::new(self.0.to_string()))
            }
            FormatKind::Yaml => None,
        }
    }
}

#[cfg(feature = "clap")]
impl ValueEnum for Format<OUTPUT> {
    fn value_variants<'a>() -> &'a [Self] {
        ALL_OUTPUT_FORMATS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self.0 {
            FormatKind::Html | FormatKind::Yaml => Some(PossibleValue::new(self.0.to_string())),
            FormatKind::Json | FormatKind::Xml | FormatKind::Markdown => None,
        }
    }
}

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
    Pinboard(#[from] pinboard::Error),
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

impl Format<INPUT> {
    pub fn detect(path: impl AsRef<Path>) -> Option<Format<INPUT>> {
        match path.as_ref().extension()?.to_str()? {
            "json" => Some(Format::<INPUT>::JSON),
            "xml" => Some(Format::<INPUT>::XML),
            "md" => Some(Format::<INPUT>::MARKDOWN),
            "html" => Some(Format::<{ INPUT | OUTPUT }>::HTML.as_input()),
            _ => None,
        }
    }

    pub fn parse(&self, reader: &mut impl BufRead) -> Result<Collection, ParseError> {
        match self.0 {
            FormatKind::Json => {
                let posts = Post::from_json(reader)?;
                Collection::from_posts(posts).map_err(Into::into)
            }
            FormatKind::Xml => {
                let posts = Post::from_xml(reader)?;
                Collection::from_posts(posts).map_err(Into::into)
            }
            FormatKind::Markdown => {
                let mut buf = String::new();
                reader.read_to_string(&mut buf)?;
                Collection::from_markdown(&buf).map_err(Into::into)
            }
            FormatKind::Html => {
                let mut buf = String::new();
                reader.read_to_string(&mut buf)?;
                Collection::from_html(&buf).map_err(Into::into)
            }
            FormatKind::Yaml => {
                panic!(
                    "Invariant violated: Format<INPUT> contains output-only format {:?}",
                    self.0
                )
            }
        }
    }
}

impl Format<OUTPUT> {
    pub fn detect(path: impl AsRef<Path>) -> Option<Format<OUTPUT>> {
        match path.as_ref().extension()?.to_str()? {
            "html" => Some(Format::<{ INPUT | OUTPUT }>::HTML.as_output()),
            "yaml" | "yml" => Some(Format::<OUTPUT>::YAML),
            _ => None,
        }
    }

    pub fn unparse(&self, writer: &mut impl Write, coll: &Collection) -> Result<(), UnparseError> {
        match self.0 {
            FormatKind::Yaml => serde_norway::to_writer(writer, coll)?,
            FormatKind::Html => coll.to_html(writer)?,
            FormatKind::Json | FormatKind::Xml | FormatKind::Markdown => {
                panic!(
                    "Invariant violated: Format<OUTPUT> contains input-only format {:?}",
                    self.0
                )
            }
        };
        Ok(())
    }
}
