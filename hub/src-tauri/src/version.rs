//! Comparing two version strings.
//!
//! Ported from `HS-Offline-Tracker/src/About.svelte:29`, which compares piece by
//! piece for a reason worth repeating here: `"0.9.10" > "0.9.8"` is false as a
//! string comparison, so a lexical check reports the newer release as older and
//! the update never appears.

use std::cmp::Ordering;

/// Split a version into its numeric parts, ignoring a leading `v`.
///
/// A component that is not a number counts as zero. That is deliberate: these
/// versions come from git tags across ten repositories that nobody coordinates,
/// and a tag like `1.2.0-rc1` should sort near `1.2.0` rather than throw.
fn parts(version: &str) -> Vec<u64> {
    let trimmed = version.trim();
    let trimmed = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    trimmed
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u64>().unwrap_or(0)
        })
        .collect()
}

pub fn compare(left: &str, right: &str) -> Ordering {
    let (a, b) = (parts(left), parts(right));
    for index in 0..a.len().max(b.len()) {
        let x = a.get(index).copied().unwrap_or(0);
        let y = b.get(index).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

/// Is `there` a newer version than `here`?
pub fn is_newer(there: &str, here: &str) -> bool {
    compare(there, here) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_digit_components_beat_single_digit_ones() {
        // The case the string comparison gets wrong, and the reason this
        // function exists at all.
        assert!(is_newer("0.9.10", "0.9.8"));
        assert!(!is_newer("0.9.8", "0.9.10"));
    }

    #[test]
    fn a_leading_v_is_ignored_on_either_side() {
        assert_eq!(compare("v1.3.16", "1.3.16"), Ordering::Equal);
        assert!(is_newer("v1.3.17", "1.3.16"));
        assert!(is_newer("1.3.17", "v1.3.16"));
    }

    #[test]
    fn missing_components_count_as_zero() {
        assert_eq!(compare("1.0", "1.0.0"), Ordering::Equal);
        assert!(is_newer("1.0.1", "1.0"));
        assert!(!is_newer("1.0", "1.0.1"));
    }

    #[test]
    fn real_catalog_versions() {
        assert!(is_newer("2.15.4", "2.15.3"));
        assert!(is_newer("1.4.1", "1.4.0"));
        assert!(!is_newer("1.0.2", "1.0.2"));
        assert!(!is_newer("2.7.9", "2.7.10"));
    }

    #[test]
    fn a_prerelease_suffix_does_not_panic_and_sorts_with_its_number() {
        assert_eq!(compare("1.2.0", "1.2.0-rc1"), Ordering::Equal);
        assert!(is_newer("1.2.1", "1.2.0-rc1"));
    }

    #[test]
    fn nonsense_is_not_newer_than_anything() {
        assert!(!is_newer("", "1.0.0"));
        assert!(!is_newer("not-a-version", "1.0.0"));
    }
}
