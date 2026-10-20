//! A minimal SemVer 2.0.0 parser for manifest `version` fields (doc 06
//! §2.2: "required, semver 2.0 string").
//!
//! Hand-rolled because the `semver` crate is not in this card's C5
//! dependency grant (serde/toml/thiserror + the two path deps only) and
//! the manifest surface needs just enough to validate and compare
//! majors. Grammar implemented (semver.org, §1–§9):
//!
//! ```text
//! version   = major "." minor "." patch pre? build?
//! pre       = "-" ( alnum | "-" )+   — dot-separated identifiers
//! build     = "+" ( alnum | "-" )+   — dot-separated identifiers
//! identifier = non-empty; numeric ⇒ no leading zeros
//! ```
//!
//! Precedence (semver.org §11) is exposed for the registry's
//! same-major comparisons [PROPOSAL — the v1 load rule only needs the
//! major; precedence arrives with dependency resolution].

use std::cmp::Ordering;
use std::fmt;

/// A validated SemVer 2.0.0 version (doc 06 §2.2 `version`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Semver {
    major: u64,
    minor: u64,
    patch: u64,
    /// Dot-separated pre-release identifiers, with the leading `-`.
    pre: Option<String>,
    /// Dot-separated build identifiers, with the leading `+`.
    build: Option<String>,
}

impl Semver {
    /// Parse and validate a version string (semver 2.0.0).
    ///
    /// # Errors
    /// [`SemverError`] naming the first rule violated — actionable, per
    /// the house diagnostic style (MP §8).
    pub fn parse(s: &str) -> Result<Semver, SemverError> {
        let bad = |why: SemverError| Err(why);
        // Split off build metadata first (allowed after pre-release).
        let (core_pre, build) = match s.split_once('+') {
            Some((core, b)) => match valid_identifiers(b, false) {
                Ok(()) => (core, Some(b)),
                Err(e) => return bad(e),
            },
            None => (s, None),
        };
        // Then the pre-release.
        let (core, pre) = match core_pre.split_once('-') {
            Some((c, p)) => match valid_identifiers(p, true) {
                Ok(()) => (c, Some(p)),
                Err(e) => return bad(e),
            },
            None => (core_pre, None),
        };
        // Core: exactly three numeric components, no leading zeros.
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return bad(SemverError::CoreShape(core.to_string()));
        }
        let mut nums = [0u64; 3];
        for (slot, part) in nums.iter_mut().zip(parts) {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return bad(SemverError::NumericComponent(part.to_string()));
            }
            if part.len() > 1 && part.starts_with('0') {
                return bad(SemverError::LeadingZero(part.to_string()));
            }
            *slot = match part.parse::<u64>() {
                Ok(n) => n,
                Err(e) => return bad(SemverError::Overflow(part.to_string(), e.to_string())),
            };
        }
        Ok(Semver {
            major: nums[0],
            minor: nums[1],
            patch: nums[2],
            pre: pre.map(|p| p.to_string()),
            build: build.map(|b| b.to_string()),
        })
    }

    /// The major component (doc 06 §2.2's own field is `api_version`;
    /// this is the plugin's `version` major).
    pub const fn major(&self) -> u64 {
        self.major
    }

    /// The minor component.
    pub const fn minor(&self) -> u64 {
        self.minor
    }

    /// The patch component.
    pub const fn patch(&self) -> u64 {
        self.patch
    }

    /// Precedence per semver.org §11 (build metadata ignored).
    ///
    /// A version with a pre-release has *lower* precedence than the
    /// same version without; identifiers compare numeric &lt; alphanumeric,
    /// field-by-field, shorter set loses when prefixes are equal.
    pub fn precedence(&self, other: &Semver) -> Ordering {
        let core = (
            self.major.cmp(&other.major),
            self.minor.cmp(&other.minor),
            self.patch.cmp(&other.patch),
        );
        let core = match core.0 {
            Ordering::Equal => match core.1 {
                Ordering::Equal => core.2,
                o => o,
            },
            o => o,
        };
        if core != Ordering::Equal {
            return core;
        }
        match (&self.pre, &other.pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => compare_pre(a, b),
        }
    }
}

impl fmt::Display for Semver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{build}")?;
        }
        Ok(())
    }
}

/// Validate dot-separated pre-release / build identifiers (semver.org
/// §9–§10: `[0-9A-Za-z-]+`, non-empty, numeric identifiers carry no
/// leading zeros in pre-release only).
fn valid_identifiers(s: &str, is_pre: bool) -> Result<(), SemverError> {
    if s.is_empty() {
        return Err(SemverError::EmptyIdentifier);
    }
    for ident in s.split('.') {
        if ident.is_empty() {
            return Err(SemverError::EmptyIdentifier);
        }
        if !ident
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(SemverError::IdentChars(ident.to_string()));
        }
        let numeric = ident.bytes().all(|b| b.is_ascii_digit());
        if is_pre && numeric && ident.len() > 1 && ident.starts_with('0') {
            return Err(SemverError::LeadingZero(ident.to_string()));
        }
    }
    Ok(())
}

/// Pre-release precedence (semver.org §11.4).
fn compare_pre(a: &str, b: &str) -> Ordering {
    let mut ai = a.split('.');
    let mut bi = b.split('.');
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less, // smaller set
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let xn = x.bytes().all(|b| b.is_ascii_digit());
                let yn = y.bytes().all(|b| b.is_ascii_digit());
                let ord = match (xn, yn) {
                    (true, true) => x
                        .parse::<u64>()
                        .unwrap_or(0)
                        .cmp(&y.parse::<u64>().unwrap_or(0)),
                    (true, false) => Ordering::Less, // numeric < alphanumeric
                    (false, true) => Ordering::Greater,
                    (false, false) => x.cmp(y),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

/// SemVer validation failures (doc 06 §2.2 `version`; MP §8 actionable
/// diagnostics).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SemverError {
    /// Not `major.minor.patch`.
    #[error(
        "semver core must be exactly `major.minor.patch` (three numeric components), found `{0}`"
    )]
    CoreShape(String),
    /// A core component is not numeric.
    #[error("semver component `{0}` is not a plain number")]
    NumericComponent(String),
    /// Numeric component or pre-release identifier with a leading zero.
    #[error("numeric component `{0}` has a leading zero (semver 2.0.0 forbids it)")]
    LeadingZero(String),
    /// A version component exceeds u64.
    #[error("semver component `{0}` overflows 64 bits: {1}")]
    Overflow(String, String),
    /// A pre-release / build identifier list has an empty element.
    #[error("pre-release/build identifiers must be non-empty")]
    EmptyIdentifier,
    /// An identifier contains characters outside `[0-9A-Za-z-]`.
    #[error("identifier `{0}` contains characters outside [0-9A-Za-z-]")]
    IdentChars(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_examples_parse() {
        for v in ["0.1.0", "1.0.0", "0.4.2"] {
            let sv = match Semver::parse(v) {
                Ok(sv) => sv,
                Err(e) => unreachable!("{v}: {e}"),
            };
            assert_eq!(sv.to_string(), v, "round trip {v}");
        }
    }

    #[test]
    fn full_grammar() {
        let sv = match Semver::parse("2.3.7-alpha.1.x+build.5.b") {
            Ok(sv) => sv,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!((sv.major(), sv.minor(), sv.patch()), (2, 3, 7));
        assert_eq!(sv.to_string(), "2.3.7-alpha.1.x+build.5.b");
    }

    #[test]
    fn rejects_bad_versions() {
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "1.2.x",
            "v1.2.3",
            "1.02.3",
            "01.2.3",
            "1.2.03",
            "1.2.3-",
            "1.2.3+",
            "1.2.3-a..b",
            "1.2.3-01",
            "1.2.3-alpha_1",
            "1.2.3+@",
            " 1.2.3",
            "1.2.3 ",
            "1.2.3-alpha+",
        ] {
            assert!(Semver::parse(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn precedence_rules() {
        // semver.org §11 worked example.
        let order = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in order.windows(2) {
            let (lo, hi) = (pair[0], pair[1]);
            let a = match Semver::parse(lo) {
                Ok(a) => a,
                Err(e) => unreachable!("{lo}: {e}"),
            };
            let b = match Semver::parse(hi) {
                Ok(b) => b,
                Err(e) => unreachable!("{hi}: {e}"),
            };
            assert_eq!(a.precedence(&b), Ordering::Less, "{lo} < {hi}");
            assert_eq!(b.precedence(&a), Ordering::Greater, "{hi} > {lo}");
        }
        // Build metadata is ignored for precedence.
        let a = match Semver::parse("1.0.0+one") {
            Ok(a) => a,
            Err(e) => unreachable!("{e}"),
        };
        let b = match Semver::parse("1.0.0+two") {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(a.precedence(&b), Ordering::Equal);
        assert_ne!(a, b, "equality still distinguishes builds");
    }

    #[test]
    fn zero_is_legal_but_only_unprefixed() {
        assert!(Semver::parse("0.0.0").is_ok());
        assert!(Semver::parse("0.0.0-0").is_ok());
        assert!(Semver::parse("0.0.0-00").is_err(), "leading zero in pre");
        assert!(Semver::parse("10.20.30").is_ok());
    }
}
