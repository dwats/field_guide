//! Name rules shared by guides, tags, entries, aliases, and vocabulary.

use crate::{Error, Result};

/// The key used for case-insensitive uniqueness and lookup: trimmed and Unicode-lowercased.
pub fn normalize(name: &str) -> String {
    name.trim().to_lowercase()
}

fn invalid(kind: &'static str, reason: &str) -> Error {
    Error::InvalidName {
        kind,
        reason: reason.to_owned(),
    }
}

fn single_line<'a>(kind: &'static str, raw: &'a str) -> Result<&'a str> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(invalid(kind, "must not be empty"));
    }
    if name.contains(['\n', '\r']) {
        return Err(invalid(kind, "must be a single line"));
    }
    Ok(name)
}

/// Entry titles and aliases: one line, and none of `[[`, `]]`, `|`, which would break wikilink syntax.
pub(crate) fn clean_entry_name(kind: &'static str, raw: &str) -> Result<String> {
    let name = single_line(kind, raw)?;
    if name.contains("[[") || name.contains("]]") || name.contains('|') {
        return Err(invalid(kind, "must not contain [[, ]], or |"));
    }
    Ok(name.to_owned())
}

/// Tag names: one line, no `/` because that is the path separator.
pub(crate) fn clean_tag_name(raw: &str) -> Result<String> {
    let name = single_line("tag name", raw)?;
    if name.contains('/') {
        return Err(invalid("tag name", "must not contain /"));
    }
    Ok(name.to_owned())
}

pub(crate) fn clean_guide_name(raw: &str) -> Result<String> {
    single_line("guide name", raw).map(str::to_owned)
}

/// Verb and qualifier labels: one line, trimmed.
pub(crate) fn clean_label(kind: &'static str, raw: &str) -> Result<String> {
    single_line(kind, raw).map(str::to_owned)
}
