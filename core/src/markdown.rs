//! Entry bodies rendered to HTML. One renderer for the app preview and any exported HTML, so
//! wikilink and media resolution is tested once (`docs/design.md` section 1).
//!
//! Supported: CommonMark, GFM tables, strikethrough, task lists, `[[wikilinks]]`, and media
//! references of the form `media/<sha256>.<ext>`. A single newline is a line break wherever
//! CommonMark would soften it to a space (paragraphs, quotes, list items), as in Obsidian and
//! GitHub comments. Raw HTML in a
//! body is shown as text, never passed through, so a body from someone else's archive cannot
//! inject markup.
//!
//! The output is plain, prefix-free HTML. Wikilinks become `<a class="wikilink" data-entry-id=...>`
//! when they resolve and `<a class="wikilink wikilink-unresolved" data-target=...>` when they do
//! not; the UI decides what a click does. Media keeps its `media/...` path in `src`, and the host
//! rewrites it to a URL it can serve. Headings get an id from their text, listed in
//! `Rendered::headings`, so a table of contents and exported HTML share one anchor.

use std::collections::HashSet;
use std::fmt::Write as _;

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

use crate::{EntryId, GuideId, Result, Vault, wikilink};

/// Rendered body plus what the renderer learned about its links.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rendered {
    pub html: String,
    /// Distinct link targets, as written, that matched no title or alias in the guide.
    pub unresolved_links: Vec<String>,
    /// Distinct entry ids the body links to.
    pub linked_entry_ids: Vec<EntryId>,
    /// The body's headings in order, each with the id set on its `<hN>`.
    pub headings: Vec<Heading>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    /// 1 to 6.
    pub level: u8,
    /// Plain text, links and code reduced to their text.
    pub text: String,
    pub id: String,
}

impl Vault {
    /// Renders a stored entry's body.
    pub fn render_entry(&self, id: &EntryId) -> Result<Rendered> {
        let entry = self.get_entry(id)?;
        self.render_markdown(&entry.guide_id, &entry.body_md)
    }

    /// Renders any markdown as if it were an entry body in `guide_id`. Used for live preview of
    /// text that has not been saved yet.
    pub fn render_markdown(&self, guide_id: &GuideId, body: &str) -> Result<Rendered> {
        let mut resolve =
            |target: &str| self.lookup_entry(guide_id, target).map(|n| n.map(|n| n.id));
        render(body, &mut resolve)
    }
}

/// Renders with a caller-supplied resolver from link text to entry id.
pub fn render(
    body: &str,
    resolve: &mut dyn FnMut(&str) -> Result<Option<EntryId>>,
) -> Result<Rendered> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut out = Rendered::default();
    let mut events: Vec<Event<'_>> = Vec::new();
    let mut text_run = String::new();
    let mut image: Option<(String, String)> = None; // (dest, alt so far)

    let flush_text = |run: &mut String,
                      events: &mut Vec<Event<'_>>,
                      out: &mut Rendered,
                      resolve: &mut dyn FnMut(&str) -> Result<Option<EntryId>>|
     -> Result<()> {
        if run.is_empty() {
            return Ok(());
        }
        let text = std::mem::take(run);
        push_text_with_wikilinks(&text, events, out, resolve)?;
        Ok(())
    };

    for event in Parser::new_ext(body, options) {
        if let Some((_, alt)) = image.as_mut() {
            match event {
                Event::End(TagEnd::Image) => {
                    let (dest, alt) = image.take().expect("image in progress");
                    events.push(Event::Html(CowStr::from(media_html(&dest, &alt))));
                }
                Event::Text(t) | Event::Code(t) => alt.push_str(&t),
                Event::SoftBreak | Event::HardBreak => alt.push(' '),
                _ => {}
            }
            continue;
        }
        match event {
            Event::Text(t) => text_run.push_str(&t),
            Event::Html(t) | Event::InlineHtml(t) => text_run.push_str(&t),
            Event::Start(Tag::Image { dest_url, .. }) => {
                flush_text(&mut text_run, &mut events, &mut out, resolve)?;
                image = Some((dest_url.into_string(), String::new()));
            }
            // A newline the author typed is a line break they meant.
            Event::SoftBreak => {
                flush_text(&mut text_run, &mut events, &mut out, resolve)?;
                events.push(Event::HardBreak);
            }
            other => {
                flush_text(&mut text_run, &mut events, &mut out, resolve)?;
                events.push(other);
            }
        }
    }
    flush_text(&mut text_run, &mut events, &mut out, resolve)?;
    assign_heading_ids(&mut events, &mut out.headings);

    pulldown_cmark::html::push_html(&mut out.html, events.into_iter());
    Ok(out)
}

/// Gives every heading an id from its text (`Where found` becomes `where-found`; a repeat gets
/// `-2`, `-3`, ...) and records it. Wikilinks inside a heading count by their shown text.
fn assign_heading_ids(events: &mut [Event<'_>], headings: &mut Vec<Heading>) {
    let mut found: Vec<(usize, u8, String)> = Vec::new();
    let mut open: Option<(usize, u8, String)> = None;
    for (i, event) in events.iter().enumerate() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                open = Some((i, *level as u8, String::new()))
            }
            Event::End(TagEnd::Heading(_)) => found.extend(open.take()),
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, _, text)) = open.as_mut() {
                    text.push_str(t);
                }
            }
            Event::Html(h) | Event::InlineHtml(h) => {
                if let Some((_, _, text)) = open.as_mut() {
                    text.push_str(&text_of_html(h));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, text)) = open.as_mut() {
                    text.push(' ');
                }
            }
            _ => {}
        }
    }
    let mut taken = HashSet::new();
    for (start, level, raw) in found {
        let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        let base = slug(&text);
        let mut id = base.clone();
        let mut n = 1;
        while !taken.insert(id.clone()) {
            n += 1;
            id = format!("{base}-{n}");
        }
        if let Event::Start(Tag::Heading { id: slot, .. }) = &mut events[start] {
            *slot = Some(CowStr::from(id.clone()));
        }
        headings.push(Heading { level, text, id });
    }
}

/// Lowercase letters and digits with single dashes between runs; `section` when nothing is left.
fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "section".to_owned()
    } else {
        out
    }
}

/// The text of a fragment this module wrote: tags dropped, the entities `escape` writes undone.
fn text_of_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn push_text_with_wikilinks<'a>(
    text: &str,
    events: &mut Vec<Event<'a>>,
    out: &mut Rendered,
    resolve: &mut dyn FnMut(&str) -> Result<Option<EntryId>>,
) -> Result<()> {
    let mut copied_to = 0;
    for link in wikilink::find_wikilinks(text) {
        if link.start > copied_to {
            events.push(Event::Text(CowStr::from(
                text[copied_to..link.start].to_owned(),
            )));
        }
        let shown = link.text.filter(|t| !t.is_empty()).unwrap_or(link.target);
        let mut html = String::new();
        match resolve(link.target)? {
            Some(id) => {
                write!(
                    html,
                    "<a class=\"wikilink\" href=\"#\" data-entry-id=\"{}\">{}</a>",
                    escape(id.as_str()),
                    escape(shown)
                )
                .expect("write to string");
                if !out.linked_entry_ids.contains(&id) {
                    out.linked_entry_ids.push(id);
                }
            }
            None => {
                write!(
                    html,
                    "<a class=\"wikilink wikilink-unresolved\" href=\"#\" data-target=\"{}\">{}</a>",
                    escape(link.target),
                    escape(shown)
                )
                .expect("write to string");
                if !out.unresolved_links.iter().any(|t| t == link.target) {
                    out.unresolved_links.push(link.target.to_owned());
                }
            }
        }
        events.push(Event::Html(CowStr::from(html)));
        copied_to = link.end;
    }
    if copied_to < text.len() {
        events.push(Event::Text(CowStr::from(text[copied_to..].to_owned())));
    }
    Ok(())
}

/// `<img>` for images, `<video controls>` for `.webm`. Any other destination is an ordinary image
/// tag; the host decides whether it can serve it.
fn media_html(dest: &str, alt: &str) -> String {
    let is_video = dest
        .rsplit('.')
        .next()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("webm"));
    if is_video {
        format!(
            "<video controls src=\"{}\" title=\"{}\"></video>",
            escape(dest),
            escape(alt)
        )
    } else {
        format!("<img src=\"{}\" alt=\"{}\">", escape(dest), escape(alt))
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_with(body: &str, known: &[(&str, &str)]) -> Rendered {
        let mut resolve = |target: &str| {
            Ok(known
                .iter()
                .find(|(title, _)| title.eq_ignore_ascii_case(target.trim()))
                .map(|(_, id)| EntryId((*id).to_owned())))
        };
        render(body, &mut resolve).unwrap()
    }

    #[test]
    fn resolved_and_unresolved_wikilinks_render_differently() {
        let r = render_with(
            "See [[Oak Log]] and [[oak log|logs]] and [[Missing]].",
            &[("Oak Log", "n1")],
        );
        assert_eq!(
            r.html,
            "<p>See <a class=\"wikilink\" href=\"#\" data-entry-id=\"n1\">Oak Log</a> and \
             <a class=\"wikilink\" href=\"#\" data-entry-id=\"n1\">logs</a> and \
             <a class=\"wikilink wikilink-unresolved\" href=\"#\" data-target=\"Missing\">Missing</a>.</p>\n"
        );
        assert_eq!(r.linked_entry_ids, [EntryId("n1".to_owned())]);
        assert_eq!(r.unresolved_links, ["Missing"]);
    }

    #[test]
    fn raw_html_is_shown_as_text() {
        let r = render_with("a <b>bold</b> move\n\n<script>alert(1)</script>\n", &[]);
        assert!(r.html.contains("&lt;b&gt;bold&lt;/b&gt;"), "{}", r.html);
        assert!(r.html.contains("&lt;script&gt;"), "{}", r.html);
        assert!(!r.html.contains("<script>"), "{}", r.html);
    }

    #[test]
    fn media_references_become_images_and_videos() {
        let r = render_with(
            "![A stone](media/abc.png)\n\n![clip](media/def.webm)\n",
            &[],
        );
        assert!(
            r.html
                .contains("<img src=\"media/abc.png\" alt=\"A stone\">"),
            "{}",
            r.html
        );
        assert!(
            r.html
                .contains("<video controls src=\"media/def.webm\" title=\"clip\"></video>"),
            "{}",
            r.html
        );
    }

    #[test]
    fn gfm_extensions_are_on() {
        let r = render_with(
            "| a | b |\n|---|---|\n| 1 | 2 |\n\n~~gone~~\n\n- [x] done\n",
            &[],
        );
        assert!(r.html.contains("<table>"), "{}", r.html);
        assert!(r.html.contains("<del>gone</del>"), "{}", r.html);
        assert!(r.html.contains("type=\"checkbox\""), "{}", r.html);
    }

    #[test]
    fn headings_get_anchor_ids_and_are_listed() {
        let r = render_with(
            "# Where found\n\ntext\n\n## Where  found\n\n### [[Oak Log]] & co\n\n#### !!!\n",
            &[("Oak Log", "n1")],
        );
        let listed: Vec<(u8, &str, &str)> = r
            .headings
            .iter()
            .map(|h| (h.level, h.text.as_str(), h.id.as_str()))
            .collect();
        assert_eq!(
            listed,
            [
                (1, "Where found", "where-found"),
                (2, "Where found", "where-found-2"),
                (3, "Oak Log & co", "oak-log-co"),
                (4, "!!!", "section"),
            ]
        );
        assert!(r.html.contains("<h1 id=\"where-found\">"), "{}", r.html);
        assert!(r.html.contains("<h2 id=\"where-found-2\">"), "{}", r.html);
        assert!(r.html.contains("<h3 id=\"oak-log-co\">"), "{}", r.html);
        assert!(r.html.contains("<h4 id=\"section\">"), "{}", r.html);
    }

    #[test]
    fn a_newline_inside_a_paragraph_or_quote_is_a_line_break() {
        let r = render_with("one\ntwo\n\n> first\n> second\n", &[]);
        assert_eq!(
            r.html,
            "<p>one<br />\ntwo</p>\n<blockquote>\n<p>first<br />\nsecond</p>\n</blockquote>\n"
        );
    }

    #[test]
    fn a_newline_inside_a_list_item_is_a_line_break_and_a_hard_break_stays_one() {
        let r = render_with("- one\n  two\n\nthree  \nfour\n", &[]);
        assert!(r.html.contains("one<br />\ntwo"), "{}", r.html);
        assert!(r.html.contains("three<br />\nfour"), "{}", r.html);
    }

    #[test]
    fn heading_text_from_a_link_has_its_escaping_undone() {
        let r = render_with("## [[A & B|see \"A\"]]\n", &[("A & B", "n1")]);
        assert_eq!(r.headings[0].text, "see \"A\"");
        assert_eq!(r.headings[0].id, "see-a");
    }

    #[test]
    fn link_text_in_a_code_span_is_not_a_link() {
        let r = render_with("`[[Oak Log]]`", &[("Oak Log", "n1")]);
        assert_eq!(r.html, "<p><code>[[Oak Log]]</code></p>\n");
        assert!(r.linked_entry_ids.is_empty());
    }

    #[test]
    fn link_text_is_escaped() {
        let r = render_with("[[Missing|a <b> & c]]", &[]);
        assert!(r.html.contains(">a &lt;b&gt; &amp; c</a>"), "{}", r.html);
    }
}
