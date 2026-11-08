//! Note bodies rendered to HTML. One renderer for the app preview and any exported HTML, so
//! wikilink and media resolution is tested once (`docs/design.md` section 1).
//!
//! Supported: CommonMark, GFM tables, strikethrough, task lists, `[[wikilinks]]`, and media
//! references of the form `media/<sha256>.<ext>`. Raw HTML in a body is shown as text, never
//! passed through, so a body from someone else's archive cannot inject markup.
//!
//! The output is plain, prefix-free HTML. Wikilinks become `<a class="wikilink" data-note-id=...>`
//! when they resolve and `<a class="wikilink wikilink-unresolved" data-target=...>` when they do
//! not; the UI decides what a click does. Media keeps its `media/...` path in `src`, and the host
//! rewrites it to a URL it can serve.

use std::fmt::Write as _;

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

use crate::{GuideId, NoteId, Result, Vault, wikilink};

/// Rendered body plus what the renderer learned about its links.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rendered {
    pub html: String,
    /// Distinct link targets, as written, that matched no title or alias in the guide.
    pub unresolved_links: Vec<String>,
    /// Distinct note ids the body links to.
    pub linked_note_ids: Vec<NoteId>,
}

impl Vault {
    /// Renders a stored note's body.
    pub fn render_note(&self, id: &NoteId) -> Result<Rendered> {
        let note = self.get_note(id)?;
        self.render_markdown(&note.guide_id, &note.body_md)
    }

    /// Renders any markdown as if it were a note body in `guide_id`. Used for live preview of
    /// text that has not been saved yet.
    pub fn render_markdown(&self, guide_id: &GuideId, body: &str) -> Result<Rendered> {
        let mut resolve =
            |target: &str| self.lookup_note(guide_id, target).map(|n| n.map(|n| n.id));
        render(body, &mut resolve)
    }
}

/// Renders with a caller-supplied resolver from link text to note id.
pub fn render(
    body: &str,
    resolve: &mut dyn FnMut(&str) -> Result<Option<NoteId>>,
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
                      resolve: &mut dyn FnMut(&str) -> Result<Option<NoteId>>|
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
            other => {
                flush_text(&mut text_run, &mut events, &mut out, resolve)?;
                events.push(other);
            }
        }
    }
    flush_text(&mut text_run, &mut events, &mut out, resolve)?;

    pulldown_cmark::html::push_html(&mut out.html, events.into_iter());
    Ok(out)
}

fn push_text_with_wikilinks<'a>(
    text: &str,
    events: &mut Vec<Event<'a>>,
    out: &mut Rendered,
    resolve: &mut dyn FnMut(&str) -> Result<Option<NoteId>>,
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
                    "<a class=\"wikilink\" href=\"#\" data-note-id=\"{}\">{}</a>",
                    escape(id.as_str()),
                    escape(shown)
                )
                .expect("write to string");
                if !out.linked_note_ids.contains(&id) {
                    out.linked_note_ids.push(id);
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
                .map(|(_, id)| NoteId((*id).to_owned())))
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
            "<p>See <a class=\"wikilink\" href=\"#\" data-note-id=\"n1\">Oak Log</a> and \
             <a class=\"wikilink\" href=\"#\" data-note-id=\"n1\">logs</a> and \
             <a class=\"wikilink wikilink-unresolved\" href=\"#\" data-target=\"Missing\">Missing</a>.</p>\n"
        );
        assert_eq!(r.linked_note_ids, [NoteId("n1".to_owned())]);
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
    fn link_text_in_a_code_span_is_not_a_link() {
        let r = render_with("`[[Oak Log]]`", &[("Oak Log", "n1")]);
        assert_eq!(r.html, "<p><code>[[Oak Log]]</code></p>\n");
        assert!(r.linked_note_ids.is_empty());
    }

    #[test]
    fn link_text_is_escaped() {
        let r = render_with("[[Missing|a <b> & c]]", &[]);
        assert!(r.html.contains(">a &lt;b&gt; &amp; c</a>"), "{}", r.html);
    }
}
