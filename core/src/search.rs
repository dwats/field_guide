//! Full-text search helpers. The index is the `entries_fts` table, kept current by triggers.

/// Turns free text into an FTS5 expression: each whitespace-separated token becomes a quoted
/// prefix query and the tokens are ANDed. Tokens without a letter or digit are dropped, since the
/// tokenizer would reduce them to nothing. Returns `None` when nothing is left to search for.
pub(crate) fn match_expression(text: &str) -> Option<String> {
    let tokens: Vec<String> = text
        .split_whitespace()
        .filter(|token| token.chars().any(char::is_alphanumeric))
        .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}
