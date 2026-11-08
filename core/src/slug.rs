//! Slugs for archive file names: lowercase ASCII `[a-z0-9-]`, at most 64 characters, unique
//! within one directory. Titles that differ only by case or accents get distinct slugs through a
//! numeric suffix, so an archive opens the same way on case-insensitive and case-sensitive
//! filesystems and stays inside Windows path limits.

use std::collections::HashSet;

pub(crate) const MAX_LEN: usize = 64;

/// Transliterates to ASCII, lowercases, and collapses everything else into single dashes.
/// Uses `fallback` (slugified the same way) when nothing printable survives.
pub(crate) fn slugify(text: &str, fallback: &str) -> String {
    let mut slug = ascii_slug(text);
    if slug.is_empty() {
        slug = ascii_slug(fallback);
    }
    if slug.is_empty() {
        slug.push_str("item");
    }
    truncate(&mut slug, MAX_LEN);
    slug
}

fn ascii_slug(text: &str) -> String {
    let ascii = deunicode::deunicode_with_tofu(text, "");
    let mut out = String::with_capacity(ascii.len());
    for c in ascii.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Slugs are ASCII, so bytes are characters.
fn truncate(slug: &mut String, max: usize) {
    if slug.len() > max {
        slug.truncate(max);
    }
    while slug.ends_with('-') {
        slug.pop();
    }
}

/// Hands out slugs that are unique within one directory.
#[derive(Debug, Default)]
pub(crate) struct SlugSet {
    used: HashSet<String>,
}

impl SlugSet {
    pub(crate) fn claim(&mut self, text: &str, fallback: &str) -> String {
        let base = slugify(text, fallback);
        if self.used.insert(base.clone()) {
            return base;
        }
        for n in 2u32.. {
            let suffix = format!("-{n}");
            let mut candidate = base.clone();
            truncate(&mut candidate, MAX_LEN - suffix.len());
            candidate.push_str(&suffix);
            if self.used.insert(candidate.clone()) {
                return candidate;
            }
        }
        unreachable!("the suffix space is unbounded")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_become_lowercase_dashed_ascii() {
        assert_eq!(slugify("Stone Block", "x"), "stone-block");
        assert_eq!(slugify("  Oak  Log! ", "x"), "oak-log");
        assert_eq!(
            slugify("Half-Life 2: Episode Two", "x"),
            "half-life-2-episode-two"
        );
    }

    #[test]
    fn accents_and_other_scripts_transliterate() {
        assert_eq!(slugify("Café au Lait", "x"), "cafe-au-lait");
        assert_eq!(slugify("Pokémon", "x"), "pokemon");
        assert_eq!(slugify("Straße", "x"), "strasse");
        let japanese = slugify("日本語", "x");
        assert!(!japanese.is_empty());
        assert!(japanese.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
    }

    #[test]
    fn the_fallback_covers_titles_with_nothing_printable() {
        assert_eq!(slugify("!!!", "entry-018f"), "entry-018f");
        assert_eq!(slugify("", ""), "item");
    }

    #[test]
    fn long_titles_are_capped_even_with_a_suffix() {
        let long = "a".repeat(200);
        assert_eq!(slugify(&long, "x").len(), MAX_LEN);
        let mut set = SlugSet::default();
        set.claim(&long, "x");
        let second = set.claim(&long, "x");
        assert_eq!(second.len(), MAX_LEN);
        assert!(second.ends_with("-2"));
    }

    #[test]
    fn case_and_accent_variants_get_distinct_slugs() {
        let mut set = SlugSet::default();
        assert_eq!(set.claim("Café", "x"), "cafe");
        assert_eq!(set.claim("cafe", "x"), "cafe-2");
        assert_eq!(set.claim("CAFE", "x"), "cafe-3");
        assert_eq!(set.claim("Oak Log", "x"), "oak-log");
    }
}
