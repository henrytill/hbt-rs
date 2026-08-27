use std::{
    collections::{BTreeSet, HashMap},
    io::{self, Write},
};

use minijinja::{Environment, context};
use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use thiserror::Error;

use crate::{
    collection::Collection,
    entity::{self, Entity, Extended, Label, Name, Time},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Entity(#[from] entity::Error),

    #[error("HTML selector error: {0}")]
    HtmlSelector(String),

    #[error("HTML missing required attribute: {0}")]
    HtmlAttribute(String),

    #[error("Template error: {0}")]
    Template(#[from] minijinja::Error),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

impl From<scraper::error::SelectorErrorKind<'_>> for Error {
    fn from(value: scraper::error::SelectorErrorKind<'_>) -> Error {
        Error::HtmlSelector(value.to_string())
    }
}

#[derive(Debug)]
enum StackItem<'a> {
    Element(ElementRef<'a>),
    PopGroup,
}

type Attrs = HashMap<String, String>;

fn add(
    coll: &mut Collection,
    attrs: Attrs,
    folders: impl IntoIterator<Item = impl Into<Label>>,
    maybe_name: Option<impl Into<Name>>,
    ext: Vec<impl Into<Extended>>,
) -> Result<(), Error> {
    let names = maybe_name.into_iter().map(Into::into).collect();
    let labels: BTreeSet<Label> = folders.into_iter().map(Into::into).collect();
    let ext = ext.into_iter().map(Into::into).collect();
    let entity = Entity::from_attrs(attrs, names, labels, ext)?;
    coll.upsert(entity);
    Ok(())
}

fn extract_text(elt: ElementRef) -> Option<String> {
    let trimmed = elt.text().collect::<String>().trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn extract_attrs(elt: ElementRef) -> Attrs {
    let mut attrs = HashMap::new();
    for (name, value) in elt.value().attrs() {
        attrs.insert(name.to_lowercase(), value.to_string());
    }
    attrs
}

const TAG_A: &str = "a";
const TAG_H3: &str = "h3";
const TAG_DT: &str = "dt";
const TAG_DD: &str = "dd";
const TAG_DL: &str = "dl";

impl Collection {
    /// Parses a Netscape bookmark HTML file into a collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTML is malformed or contains invalid bookmark data (e.g., missing URLs,
    /// invalid timestamps).
    pub fn from_html(html: &str) -> Result<Collection, Error> {
        let document = Html::parse_document(html);
        let root = document.root_element();

        let mut coll = Collection::new();
        let mut stack: Vec<StackItem> = Vec::new();
        let mut folders: Vec<String> = Vec::new();
        let mut pending: Option<(Attrs, Option<String>)> = None;

        let a_selector = Selector::parse(TAG_A)?;
        let h3_selector = Selector::parse(TAG_H3)?;

        for child in root.children().rev() {
            if let Some(child_elt) = ElementRef::wrap(child) {
                stack.push(StackItem::Element(child_elt));
            }
        }

        while let Some(item) = stack.pop() {
            match item {
                StackItem::Element(elt) => {
                    match elt.value().name() {
                        TAG_DT => {
                            if let Some((attrs, maybe_desc)) = pending.take() {
                                add(
                                    &mut coll,
                                    attrs,
                                    &folders,
                                    maybe_desc,
                                    Vec::<Extended>::new(),
                                )?;
                            }

                            if let Some(h3_elt) = elt.select(&h3_selector).next() {
                                if let Some(folder) = extract_text(h3_elt) {
                                    folders.push(folder);
                                }
                            } else if let Some(a_elt) = elt.select(&a_selector).next() {
                                let attrs = extract_attrs(a_elt);
                                let maybe_desc = extract_text(a_elt);
                                pending = Some((attrs, maybe_desc));
                            }
                        }
                        TAG_DD => {
                            if let Some((attrs, maybe_desc)) = pending.take() {
                                let maybe_ext = extract_text(elt).into_iter().collect();
                                add(&mut coll, attrs, &folders, maybe_desc, maybe_ext)?;
                            }
                        }
                        TAG_DL => {
                            stack.push(StackItem::PopGroup);
                        }
                        _ => {}
                    }
                    for child in elt.children().rev() {
                        if let Some(child_elt) = ElementRef::wrap(child) {
                            stack.push(StackItem::Element(child_elt));
                        }
                    }
                }
                StackItem::PopGroup => {
                    if let Some((attrs, maybe_desc)) = pending.take() {
                        add(
                            &mut coll,
                            attrs,
                            &folders,
                            maybe_desc,
                            Vec::<Extended>::new(),
                        )?;
                    }
                    folders.pop();
                }
            }
        }

        // A bookmark stays pending until the next DT, a DD, or the end of its DL tells us whether a
        // description follows. When the input ends with none of those still to come - a fragment
        // with no enclosing DL, say - the last bookmark is simply description-less, so record it.
        if let Some((attrs, maybe_desc)) = pending.take() {
            add(
                &mut coll,
                attrs,
                &folders,
                maybe_desc,
                Vec::<Extended>::new(),
            )?;
        }

        Ok(coll)
    }

    /// Writes the collection as a Netscape bookmark HTML file.
    ///
    /// # Errors
    ///
    /// Returns an error if template rendering fails or writing to the output fails.
    pub fn to_html(&self, mut writer: impl Write) -> Result<(), Error> {
        const TEMPLATE: &str = include_str!("html/netscape_bookmarks.jinja");
        let mut env = Environment::new();
        env.add_template("netscape", TEMPLATE)?;
        let entities: Vec<EntityView> = self.entities().iter().map(EntityView::new).collect();
        let template = env.get_template("netscape")?;
        template.render_captured_to(context! { entities }, &mut writer)?;
        writer.write_all(b"\n")?;
        Ok(())
    }
}

/// Escapes the characters that would otherwise be read as markup.
///
/// `quote` selects the context. Attribute values are delimited by double quotes, so those must be
/// escaped there; in text content a double quote is an ordinary character and is left alone. The
/// apostrophe passes through in both contexts: attributes here are always double-quoted, `&apos;`
/// is not HTML 4, and bookmark titles like "O'Reilly Radar" should survive a round trip unchanged.
fn escape(s: &str, quote: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if quote => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_attr(s: &str) -> String {
    escape(s, true)
}

fn escape_text(s: &str) -> String {
    escape(s, false)
}

/// One bookmark, with every string already escaped for the context it is rendered in.
///
/// Escaping happens here rather than in the template because only the code knows which context a
/// value lands in. minijinja's own autoescaping is not usable for this: it rewrites `'` to `&#x27;`
/// and `/` to `&#x2f;`, which would mangle every URL and every apostrophe in a title.
#[derive(Debug, Serialize)]
struct EntityView {
    uri: String,
    created_at: i64,
    last_modified: Option<i64>,
    tags: Option<String>,
    shared: Option<bool>,
    to_read: Option<bool>,
    is_feed: Option<bool>,
    last_visited_at: Option<i64>,
    title: String,
    extended: Option<String>,
}

impl EntityView {
    fn new(entity: &Entity) -> EntityView {
        let url = entity.url().as_str();

        let tags = {
            let labels = entity.labels();
            if labels.is_empty() {
                None
            } else {
                let joined = labels
                    .iter()
                    .map(Label::as_str)
                    .collect::<Vec<_>>()
                    .join(",");
                Some(escape_attr(&joined))
            }
        };

        // The anchor text falls back to the URL when the bookmark has no name.
        let title = entity
            .names()
            .iter()
            .next()
            .map_or_else(|| escape_text(url), |name| escape_text(name.as_str()));

        EntityView {
            uri: escape_attr(url),
            created_at: entity.created_at().get().timestamp(),
            last_modified: entity.updated_at().first().map(|u| u.get().timestamp()),
            tags,
            shared: entity.shared().get(),
            to_read: entity.to_read().get(),
            is_feed: entity.is_feed().get(),
            last_visited_at: entity.last_visited_at().get().map(Time::timestamp),
            title,
            extended: entity
                .extended()
                .iter()
                .next()
                .map(|e| escape_text(e.as_str())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use crate::{
        collection::Collection,
        entity::{Entity, Extended, Label, Name},
    };

    fn render(url: &str, name: &str, labels: &[&str], extended: &[&str]) -> String {
        let mut attrs = HashMap::new();
        attrs.insert("href".to_string(), url.to_string());
        attrs.insert("add_date".to_string(), "100".to_string());

        let names: BTreeSet<Name> = std::iter::once(Name::from(name)).collect();
        let labels: BTreeSet<Label> = labels.iter().copied().map(Label::from).collect();
        let extended: BTreeSet<Extended> = extended.iter().copied().map(Extended::from).collect();

        let entity = Entity::from_attrs(attrs, names, labels, extended).unwrap();
        let mut coll = Collection::new();
        coll.insert(entity);

        let mut out = Vec::new();
        coll.to_html(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    /// A bookmark fragment with no enclosing DL used to trip `assert!(pending.is_none())`, so the
    /// CLI panicked and the bookmark was lost.
    #[test]
    fn parses_trailing_bookmark_without_enclosing_dl() {
        let html = concat!(
            "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n",
            r#"<DT><A HREF="https://example.com/z" ADD_DATE="1700000000">Z</A>"#,
            "\n"
        );

        let coll = Collection::from_html(html).unwrap();

        assert_eq!(coll.len(), 1);
        assert_eq!(
            coll.entities()[0]
                .names()
                .iter()
                .map(Name::as_str)
                .collect::<Vec<_>>(),
            vec!["Z"]
        );
    }

    /// The same, one level in: a DL that the input never closes.
    #[test]
    fn parses_trailing_bookmark_in_unclosed_dl() {
        let html = concat!(
            "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n<DL><p>\n",
            r#"<DT><A HREF="https://example.com/a" ADD_DATE="1700000000">A</A>"#,
            "\n"
        );

        let coll = Collection::from_html(html).unwrap();

        assert_eq!(coll.len(), 1);
    }

    #[test]
    fn escapes_ampersand_in_url_attribute() {
        let html = render("https://example.com/?a=1&b=2", "x", &[], &[]);
        assert!(
            html.contains(r#"HREF="https://example.com/?a=1&amp;b=2""#),
            "{html}"
        );
    }

    #[test]
    fn escapes_markup_in_anchor_text() {
        let html = render("https://example.com/", "Tom & Jerry <b>bold</b>", &[], &[]);
        assert!(
            html.contains(">Tom &amp; Jerry &lt;b&gt;bold&lt;/b&gt;</A>"),
            "{html}"
        );
    }

    /// The whole point of escaping: the collection must survive a trip through HTML unchanged.
    /// Before, `<b>` was emitted raw and swallowed as a tag on reparse.
    #[test]
    fn round_trips_markup_in_title() {
        let name = "Tom & Jerry <b>bold</b>";
        let html = render("https://example.com/", name, &[], &[]);

        let reparsed = Collection::from_html(&html).unwrap();
        let entity = &reparsed.entities()[0];

        assert_eq!(
            entity.names().iter().map(Name::as_str).collect::<Vec<_>>(),
            vec![name]
        );
    }

    #[test]
    fn escapes_quote_in_attribute_but_not_in_text() {
        let html = render("https://example.com/", r#"say "hi""#, &[r#"a"b"#], &[]);
        // The double quote delimits the attribute, so it must go.
        assert!(html.contains(r#"TAGS="a&quot;b""#), "{html}");
        // In text content it is an ordinary character.
        assert!(html.contains(r#">say "hi"</A>"#), "{html}");
    }

    /// Apostrophes must pass through in both contexts: `&apos;` is not HTML 4, and titles like
    /// "O'Reilly Radar" appear in the shared fixtures.
    #[test]
    fn preserves_apostrophe() {
        let html = render("https://example.com/o'r", "O'Reilly Radar", &["it's"], &[]);
        assert!(html.contains(">O'Reilly Radar</A>"), "{html}");
        assert!(html.contains(r#"TAGS="it's""#), "{html}");
        assert!(!html.contains("&#x27;"), "{html}");
    }

    /// No URL sanitizer is involved, so a bookmark tool does not rewrite non-http schemes.
    #[test]
    fn preserves_non_http_scheme() {
        let html = render("ftp://example.com/pub", "files", &[], &[]);
        assert!(html.contains(r#"HREF="ftp://example.com/pub""#), "{html}");
    }

    #[test]
    fn escapes_extended_description() {
        let html = render("https://example.com/", "x", &[], &["a & b <c>"]);
        assert!(html.contains("<DD>a &amp; b &lt;c&gt;"), "{html}");
    }
}
