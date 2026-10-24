//! Damage events (card 5.M3): the parser never panics and never
//! aborts on a malformed region — every skipped or degraded region of
//! the document is reported as a typed event with the 1-based line
//! number where it was detected (D-4: dirty-world diagnosis, the same
//! policy as the demux layer's damage ledger).
//!
//! "Damage" covers three taxa:
//!
//! * **malformed** — a block/setting that failed to parse
//!   ([`DamageKind::BadTimestamp`], [`DamageKind::BadSetting`], …);
//! * **unsupported-but-known** — legal WebVTT this Lumen v0 parser
//!   does not apply (`vertical:`, `size:`, REGION blocks) — recorded
//!   as [`DamageKind::UnsupportedSetting`]/[`DamageKind::UnsupportedBlock`]
//!   so callers can surface the degradation honestly instead of
//!   silently rendering wrong;
//! * **noted** — unknown setting names (future revisions) and numeric
//!   cue ids beyond `u64` ([`DamageKind::UnknownSetting`],
//!   [`DamageKind::IdOverflow`]).
//!
//! Setting/block names stored in events are truncated at a char
//! boundary to ≤16 bytes so hostile inputs cannot balloon the ledger.

/// One damage/note event, anchored to a 1-based source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageEvent {
    /// 1-based line number where the event was detected.
    pub line_no: u32,
    /// What happened.
    pub kind: DamageKind,
}

impl DamageEvent {
    /// A setting that failed to parse or fell outside its domain.
    pub(super) fn bad_setting(name: &str, line_no: u32) -> Self {
        Self {
            line_no,
            kind: DamageKind::BadSetting {
                name: truncate_name(name),
            },
        }
    }

    /// A well-formed but unrecognized setting name.
    pub(super) fn unknown_setting(name: &str, line_no: u32) -> Self {
        Self {
            line_no,
            kind: DamageKind::UnknownSetting {
                name: truncate_name(name),
            },
        }
    }

    /// A known WebVTT setting Lumen v0 does not apply.
    pub(super) fn unsupported_setting(name: &str, line_no: u32) -> Self {
        Self {
            line_no,
            kind: DamageKind::UnsupportedSetting {
                name: truncate_name(name),
            },
        }
    }

    /// A known WebVTT block type Lumen v0 skips.
    pub(super) fn unsupported_block(name: &str, line_no: u32) -> Self {
        Self {
            line_no,
            kind: DamageKind::UnsupportedBlock {
                name: truncate_name(name),
            },
        }
    }
}

/// Cap a captured name at 16 bytes on a char boundary.
fn truncate_name(name: &str) -> String {
    if name.len() <= 16 {
        name.to_owned()
    } else {
        let mut end = 16;
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        name[..end].to_owned()
    }
}

/// The event taxonomy. `#[non_exhaustive]`: later format cards may add
/// taxa (the Lead's 5.M1 unification).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DamageKind {
    /// A cue timing line's timestamps failed to parse.
    BadTimestamp,
    /// A cue's `end` timestamp precedes its `start` — typed-invalid in
    /// the model, so the cue is dropped.
    ReversedInterval {
        /// The rejected start (µs).
        start_us: i64,
        /// The rejected end (µs).
        end_us: i64,
    },
    /// The `-->` separator appeared deeper than the second line of a
    /// block (a cue id may only be the single first line).
    TimingLineNotSecond,
    /// A block that is neither header, NOTE, STYLE, REGION nor a cue
    /// (no `-->` anywhere).
    OrphanBlock,
    /// A setting whose value is malformed or out of domain.
    BadSetting {
        /// The setting's (truncated) name.
        name: String,
    },
    /// A well-formed but unrecognized setting name (forward compat).
    UnknownSetting {
        /// The setting's (truncated) name.
        name: String,
    },
    /// A known WebVTT setting Lumen v0 does not apply (`vertical`,
    /// `size`): vertical-writing and box-size semantics are not in the
    /// model yet — recorded, never silently ignored.
    UnsupportedSetting {
        /// The setting's (truncated) name.
        name: String,
    },
    /// A known WebVTT block Lumen v0 skips entirely (REGION: region
    /// anchoring is not in the model yet).
    UnsupportedBlock {
        /// The block's (truncated) leading word.
        name: String,
    },
    /// A numeric cue id that does not fit `u64` — an auto id was
    /// assigned instead.
    IdOverflow,
}

impl core::fmt::Display for DamageKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadTimestamp => write!(f, "cue timestamps failed to parse"),
            Self::ReversedInterval { start_us, end_us } => {
                write!(f, "cue end {end_us} µs precedes start {start_us} µs")
            }
            Self::TimingLineNotSecond => {
                write!(f, "`-->` separator deeper than the second block line")
            }
            Self::OrphanBlock => write!(f, "block is neither header, NOTE, STYLE, REGION nor cue"),
            Self::BadSetting { name } => write!(f, "setting `{name}` malformed or out of domain"),
            Self::UnknownSetting { name } => write!(f, "unknown setting `{name}` skipped"),
            Self::UnsupportedSetting { name } => {
                write!(f, "known-but-unsupported setting `{name}` not applied")
            }
            Self::UnsupportedBlock { name } => {
                write!(f, "known-but-unsupported `{name}` block skipped")
            }
            Self::IdOverflow => write!(f, "numeric cue id beyond u64; auto id assigned"),
        }
    }
}
