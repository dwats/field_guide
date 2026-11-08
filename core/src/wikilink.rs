//! `[[Title]]` and `[[Title|shown text]]` links in entry bodies.
//!
//! The text is the human-readable serialization; the `mention` table holds the resolved ids.
//! This scanner is plain text: it does not know about code spans, so a link written inside
//! backticks is still a link. Rendering and the scanner must agree, and both live in core.

use crate::names::normalize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink<'a> {
    /// The part before `|`, trimmed. Resolved against titles and aliases.
    pub target: &'a str,
    /// The part after `|`, verbatim, when present.
    pub text: Option<&'a str>,
    /// Byte offset of the opening `[[`.
    pub start: usize,
    /// Byte offset just past the closing `]]`.
    pub end: usize,
}

/// Every well-formed link in `body`, in order. A link must close on the same line and its target
/// must not be blank.
pub fn find_wikilinks(body: &str) -> Vec<WikiLink<'_>> {
    let mut links = Vec::new();
    let mut from = 0;
    while let Some(open) = body[from..].find("[[") {
        let start = from + open;
        let inner_start = start + 2;
        let Some(close) = body[inner_start..].find("]]") else {
            break;
        };
        let inner_end = inner_start + close;
        let inner = &body[inner_start..inner_end];
        if inner.contains('\n') || inner.contains("[[") {
            // Not a link; look for the next opener inside what we skipped.
            from = inner_start;
            continue;
        }
        let (target, text) = match inner.split_once('|') {
            Some((target, text)) => (target.trim(), Some(text)),
            None => (inner.trim(), None),
        };
        if !target.is_empty() {
            links.push(WikiLink {
                target,
                text,
                start,
                end: inner_end + 2,
            });
        }
        from = inner_end + 2;
    }
    links
}

/// Rewrites every link whose target names `old` (case-insensitively) to point at `new`, keeping
/// any `|text`. Returns the new body and how many links changed.
pub fn rewrite_targets(body: &str, old: &str, new: &str) -> (String, usize) {
    let old_key = normalize(old);
    let mut out = String::with_capacity(body.len());
    let mut copied_to = 0;
    let mut changed = 0;
    for link in find_wikilinks(body) {
        if normalize(link.target) != old_key {
            continue;
        }
        out.push_str(&body[copied_to..link.start]);
        out.push_str("[[");
        out.push_str(new);
        if let Some(text) = link.text {
            out.push('|');
            out.push_str(text);
        }
        out.push_str("]]");
        copied_to = link.end;
        changed += 1;
    }
    out.push_str(&body[copied_to..]);
    (out, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_targets_and_display_text() {
        let body = "see [[Oak Log]] and [[Oak Log|logs]] but not [[ ]] or [[a\nb]] or [[unclosed";
        let got: Vec<(&str, Option<&str>)> = find_wikilinks(body)
            .iter()
            .map(|l| (l.target, l.text))
            .collect();
        assert_eq!(got, [("Oak Log", None), ("Oak Log", Some("logs"))]);
    }

    #[test]
    fn a_nested_opener_restarts_the_scan() {
        let targets: Vec<&str> = find_wikilinks("[[a [[b]] c]]")
            .iter()
            .map(|l| l.target)
            .collect();
        assert_eq!(targets, ["b"]);
    }

    #[test]
    fn rewrites_only_matching_targets_and_keeps_text() {
        let (out, n) = rewrite_targets("[[old]] [[Old|shown]] [[Older]] [[other|]]", "Old", "New");
        assert_eq!(out, "[[New]] [[New|shown]] [[Older]] [[other|]]");
        assert_eq!(n, 2);
    }
}
