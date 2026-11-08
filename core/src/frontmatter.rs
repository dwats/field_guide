//! Note front matter for archives: the fixed key set from `docs/design.md` section 7, written as
//! plain YAML and read back by core without a YAML library. A value is written plain when YAML
//! reads it back unchanged and JSON-quoted otherwise, so any YAML tool sees the same values that
//! core does, and core's parser only has to understand its own output.

use crate::{Error, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FrontMatter {
    pub id: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub stub: bool,
    pub gallery: Vec<String>,
    pub created: String,
    pub updated: String,
}

const FENCE: &str = "---";

/// The note file: front matter, closing fence, then the body verbatim.
pub(crate) fn write(front: &FrontMatter, body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 256);
    out.push_str(FENCE);
    out.push('\n');
    out.push_str(&format!("id: {}\n", scalar(&front.id)));
    out.push_str(&format!("title: {}\n", scalar(&front.title)));
    out.push_str(&format!("aliases: {}\n", list(&front.aliases)));
    out.push_str(&format!("tags: {}\n", list(&front.tags)));
    out.push_str(&format!("stub: {}\n", front.stub));
    out.push_str(&format!("gallery: {}\n", list(&front.gallery)));
    out.push_str(&format!("created: {}\n", scalar(&front.created)));
    out.push_str(&format!("updated: {}\n", scalar(&front.updated)));
    out.push_str(FENCE);
    out.push('\n');
    out.push_str(body);
    out
}

/// Splits a note file into its front matter and body. Unknown keys are ignored so a file from a
/// newer build still reads; `format_version` in the manifest gates real incompatibilities.
pub(crate) fn parse(text: &str) -> Result<(FrontMatter, String)> {
    let rest = text
        .strip_prefix("---\r\n")
        .or_else(|| text.strip_prefix("---\n"))
        .ok_or_else(|| archive("note file does not start with a front matter fence"))?;
    let mut front = FrontMatter::default();
    let mut offset = 0;
    loop {
        let line_end = rest[offset..].find('\n').map(|i| offset + i);
        let (line, next) = match line_end {
            Some(end) => (&rest[offset..end], end + 1),
            None => (&rest[offset..], rest.len()),
        };
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line == FENCE {
            if front.id.is_empty() || front.title.is_empty() {
                return Err(archive("front matter needs an id and a title"));
            }
            return Ok((front, rest[next..].to_owned()));
        }
        if line_end.is_none() {
            return Err(archive("front matter has no closing fence"));
        }
        if !line.trim().is_empty() {
            let (key, value) = line
                .split_once(':')
                .ok_or_else(|| archive(&format!("front matter line without a key: {line}")))?;
            let value = value.trim();
            match key.trim() {
                "id" => front.id = unquote(value)?,
                "title" => front.title = unquote(value)?,
                "aliases" => front.aliases = parse_list(value)?,
                "tags" => front.tags = parse_list(value)?,
                "gallery" => front.gallery = parse_list(value)?,
                "created" => front.created = unquote(value)?,
                "updated" => front.updated = unquote(value)?,
                "stub" => {
                    front.stub = match value {
                        "true" => true,
                        "false" => false,
                        other => {
                            return Err(archive(&format!(
                                "stub must be true or false, not {other}"
                            )));
                        }
                    }
                }
                _ => {}
            }
        }
        offset = next;
    }
}

fn archive(message: &str) -> Error {
    Error::Archive(message.to_owned())
}

fn scalar(value: &str) -> String {
    if needs_quotes(value, false) {
        json_quote(value)
    } else {
        value.to_owned()
    }
}

fn list(items: &[String]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|item| {
            if needs_quotes(item, true) {
                json_quote(item)
            } else {
                item.clone()
            }
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

fn json_quote(value: &str) -> String {
    serde_json::to_string(value).expect("a string always serializes")
}

/// True when YAML would read the plain form back as something other than this exact text.
fn needs_quotes(value: &str, in_list: bool) -> bool {
    if value.is_empty() || value != value.trim() {
        return true;
    }
    let first = value.chars().next().expect("non-empty");
    if "-?:,[]{}#&*!|>'\"%@`".contains(first) {
        return true;
    }
    if value.contains(": ") || value.contains(" #") || value.ends_with(':') {
        return true;
    }
    if value.chars().any(char::is_control) {
        return true;
    }
    if in_list && value.contains([',', '[', ']']) {
        return true;
    }
    let lower = value.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "true" | "false" | "null" | "~" | "yes" | "no" | "on" | "off" | "y" | "n"
    ) {
        return true;
    }
    looks_numeric(value)
}

/// Anything YAML might read as a number, including dates like `2026-11-07`. Timestamps with a
/// time part contain letters, so they stay plain.
fn looks_numeric(value: &str) -> bool {
    let first = value.chars().next().expect("non-empty");
    (first.is_ascii_digit() || first == '+' || first == '-' || first == '.')
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() || "+-._xXoObBeE".contains(c))
}

fn unquote(value: &str) -> Result<String> {
    if value.starts_with('"') {
        return serde_json::from_str(value)
            .map_err(|e| archive(&format!("bad quoted value {value}: {e}")));
    }
    if let Some(inner) = value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        return Ok(inner.replace("''", "'"));
    }
    Ok(value.to_owned())
}

fn parse_list(value: &str) -> Result<Vec<String>> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or_else(|| archive(&format!("expected a [list], found {value}")))?;
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in inner.chars() {
        match quote {
            Some(q) => {
                current.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' && q == '"' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' if current.trim().is_empty() => {
                    quote = Some(c);
                    current.push(c);
                }
                ',' => {
                    items.push(std::mem::take(&mut current));
                }
                _ => current.push(c),
            },
        }
    }
    if quote.is_some() {
        return Err(archive(&format!("unterminated quote in list {value}")));
    }
    items.push(current);
    items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(unquote)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FrontMatter {
        FrontMatter {
            id: "018f-abc".to_owned(),
            title: "Half-Life 2: Episode \"Two\"".to_owned(),
            aliases: vec![
                "  leading space".to_owned(),
                "true".to_owned(),
                "1984".to_owned(),
                "a, b".to_owned(),
                "#hash".to_owned(),
                "Café".to_owned(),
                "it's".to_owned(),
            ],
            tags: vec!["Blocks/Natural".to_owned(), "Items/Drops".to_owned()],
            stub: true,
            gallery: vec!["media/ab12.png".to_owned()],
            created: "2026-11-07T19:02:11.123Z".to_owned(),
            updated: "2026-11-07T19:40:03.000Z".to_owned(),
        }
    }

    #[test]
    fn awkward_values_and_the_body_round_trip_exactly() {
        let front = sample();
        let body = "---\nnot a fence here\n\n# Heading\n\ntrailing spaces   \r\nCRLF line\n";
        let text = write(&front, body);
        let (back, back_body) = parse(&text).unwrap();
        assert_eq!(back, front);
        assert_eq!(back_body, body);
        assert!(text.contains("title: \"Half-Life 2: Episode \\\"Two\\\"\"\n"));
        assert!(text.contains("tags: [Blocks/Natural, Items/Drops]\n"));
        assert!(text.contains("created: 2026-11-07T19:02:11.123Z\n"));
        assert!(text.contains("\"1984\""), "numeric-looking text is quoted");
        assert!(text.contains("\"true\""), "YAML keywords are quoted");
    }

    #[test]
    fn empty_lists_and_an_empty_body() {
        let front = FrontMatter {
            id: "a".to_owned(),
            title: "b".to_owned(),
            ..FrontMatter::default()
        };
        let text = write(&front, "");
        assert!(text.ends_with("---\n"));
        assert!(text.contains("aliases: []\n"));
        let (back, body) = parse(&text).unwrap();
        assert_eq!(back, front);
        assert_eq!(body, "");
    }

    #[test]
    fn other_yaml_writers_quoting_is_accepted() {
        let text = "---\r\nid: 'it''s'\r\ntitle: \"quoted\"\r\ncolor: red\r\n\r\ntags: ['A/B', \"C, D\"]\r\n---\r\nbody\r\n";
        let (front, body) = parse(text).unwrap();
        assert_eq!(front.id, "it's");
        assert_eq!(front.title, "quoted");
        assert_eq!(front.tags, ["A/B", "C, D"]);
        assert_eq!(body, "body\r\n");
    }

    #[test]
    fn malformed_files_are_refused() {
        for bad in [
            "no fence at all",
            "---\nid: x\ntitle: y\n",
            "---\nid: x\ntitle: \"unterminated\n---\n",
            "---\nid: x\n---\n",
            "---\nid: x\ntitle: y\nstub: maybe\n---\n",
            "---\nid: x\ntitle: y\ntags: not a list\n---\n",
            "---\njust words\n---\n",
        ] {
            assert!(
                matches!(parse(bad), Err(Error::Archive(_))),
                "{bad:?} parsed"
            );
        }
    }
}
