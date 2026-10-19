//! "Did you mean" suggestions (doc 07 §3.5 rule 4: near-miss keys get a
//! Levenshtein-based hint over the known key set).

/// Classic Levenshtein edit distance (character-level), bounded by nothing —
/// the candidate sets here are small schema key sets.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            let up = match prev.get(j + 1) {
                Some(v) => *v + 1,
                None => usize::MAX / 4,
            };
            let left = match curr.get(j) {
                Some(v) => *v + 1,
                None => usize::MAX / 4,
            };
            let diag = match prev.get(j) {
                Some(v) => v.saturating_add(cost),
                None => usize::MAX / 4,
            };
            if let Some(slot) = curr.get_mut(j + 1) {
                *slot = up.min(left).min(diag);
            }
        }
        core::mem::swap(&mut prev, &mut curr);
    }
    match prev.last() {
        Some(v) => *v,
        None => 0,
    }
}

/// The closest candidate within the suggestion threshold, if any.
///
/// Threshold: edit distance <= 2, or <= one third of the typo's length for
/// longer keys (doc 07 §3.5 gives `cache_demux_mb` vs `cache_demux_mib`,
/// distance 1) `[PROPOSAL: exact threshold]`.
pub fn did_you_mean<'a>(typo: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let max_dist = (typo.len() / 3).clamp(2, 4);
    let mut best: Option<(&str, usize)> = None;
    for cand in candidates {
        let d = levenshtein(typo, cand);
        if d == 0 {
            continue; // exact matches are not suggestions
        }
        if d <= max_dist {
            match best {
                Some((_, bd)) if bd <= d => {}
                _ => best = Some((*cand, d)),
            }
        }
    }
    best.map(|(cand, _)| cand)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_zero_for_identical() {
        assert_eq!(levenshtein("cache_demux_mib", "cache_demux_mib"), 0);
    }

    #[test]
    fn distance_one_for_single_substitution() {
        assert_eq!(levenshtein("cache_demux_mb", "cache_demux_mib"), 1);
    }

    #[test]
    fn distance_counts_insert_delete_substitute() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
    }

    #[test]
    fn doc_07_s35_example_suggests_the_right_key() {
        let known = [
            "cache_demux_mib",
            "cache_network_mib",
            "queue_packets_audio",
            "queue_packets_video",
        ];
        assert_eq!(
            did_you_mean("cache_demux_mb", &known),
            Some("cache_demux_mib")
        );
    }

    #[test]
    fn no_suggestion_for_unrelated_keys() {
        assert_eq!(did_you_mean("zebra", &["queue_packets_audio"]), None);
    }
}
