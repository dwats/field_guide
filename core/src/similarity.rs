//! String similarity for import mapping suggestions (`docs/design.md` section 8).
//!
//! The metric is a trait so it can be swapped without touching the mapping code. The default is a
//! normalized edit distance with no dependencies; a prefix-weighted metric such as Jaro-Winkler
//! can replace it later by implementing [`Similarity`] and passing it to
//! [`crate::Vault::propose_import_with`].

/// Scores how alike two already-normalized strings are, in `0.0..=1.0`, where `1.0` is identical.
pub trait Similarity {
    fn score(&self, a: &str, b: &str) -> f64;
}

/// One minus the Levenshtein distance divided by the longer length. Edits anywhere in the word
/// count the same; `mined with` against `mined by` scores 0.6.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NormalizedLevenshtein;

impl Similarity for NormalizedLevenshtein {
    fn score(&self, a: &str, b: &str) -> f64 {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let longest = a.len().max(b.len());
        if longest == 0 {
            return 1.0;
        }
        let mut previous: Vec<usize> = (0..=b.len()).collect();
        let mut current = vec![0; b.len() + 1];
        for (i, ca) in a.iter().enumerate() {
            current[0] = i + 1;
            for (j, cb) in b.iter().enumerate() {
                let cost = usize::from(ca != cb);
                current[j + 1] = (previous[j] + cost)
                    .min(previous[j + 1] + 1)
                    .min(current[j] + 1);
            }
            std::mem::swap(&mut previous, &mut current);
        }
        1.0 - previous[b.len()] as f64 / longest as f64
    }
}

/// The metric used when the caller does not pick one.
pub fn default_metric() -> impl Similarity {
    NormalizedLevenshtein
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_scores_one_and_disjoint_scores_zero() {
        let m = NormalizedLevenshtein;
        assert_eq!(m.score("drops", "drops"), 1.0);
        assert_eq!(m.score("", ""), 1.0);
        assert_eq!(m.score("abc", "xyz"), 0.0);
    }

    #[test]
    fn one_edit_in_five_scores_point_eight() {
        let m = NormalizedLevenshtein;
        assert!((m.score("drops", "drips") - 0.8).abs() < 1e-9);
        assert!((m.score("mined with", "mined by") - 0.6).abs() < 1e-9);
    }

    #[test]
    fn a_custom_metric_can_stand_in() {
        struct Prefix;
        impl Similarity for Prefix {
            fn score(&self, a: &str, b: &str) -> f64 {
                if a.starts_with(b) || b.starts_with(a) {
                    1.0
                } else {
                    0.0
                }
            }
        }
        assert_eq!(Prefix.score("mined with", "mined"), 1.0);
        assert_eq!(Prefix.score("mined with", "drops"), 0.0);
    }
}
