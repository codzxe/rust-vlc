//! Capability descriptors (doc 01 §2.4, card 1.D1-R): what one codec
//! implementation offers — profiles and levels as opaque strings,
//! video dimension limits, and the ADR-001 hardware posture.
//!
//! Descriptors are *claims* made by implementations and matched by the
//! registry; this crate defines the shape and pure predicates only.
//! Selection policy (priority, enable/disable, user preference) is the
//! registry's (MP §6) — deliberately absent here.

use media_types::error::ErrorContext;
use media_types::ids::CodecDescriptor;
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The error minting point for this module: capability values are codec
/// configuration data, so rejections are
/// [`Unsupported`](ErrorKind::Unsupported) at the
/// [`Decode`](PipelineStage::Decode) stage.
fn unsupported(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new(code),
    )
}

/// An opaque profile identifier ("main", "high", "profile 0", …) —
/// minted by the codec implementation, compared by equality, rendered
/// for the user (card 1.D1-R: "profiles/levels as opaque strings").
///
/// Same validation posture as
/// [`CodecDescriptor`](media_types::CodecDescriptor): non-empty, no
/// control characters. The vocabulary is open — codec families disagree
/// about what a profile *is*, and the contract must not care.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProfileDescriptor(String);

impl ProfileDescriptor {
    /// Wrap a profile identifier.
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] when empty or containing control
    /// characters.
    pub fn new(profile: &str) -> Result<ProfileDescriptor, EngineError> {
        validate_opaque(profile, "codec.profile.invalid").map(ProfileDescriptor)
    }

    /// The identifier string (already user-renderable).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProfileDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An opaque level identifier ("4.2", "level 31", …). Same rules as
/// [`ProfileDescriptor`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LevelDescriptor(String);

impl LevelDescriptor {
    /// Wrap a level identifier.
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] when empty or containing control
    /// characters.
    pub fn new(level: &str) -> Result<LevelDescriptor, EngineError> {
        validate_opaque(level, "codec.level.invalid").map(LevelDescriptor)
    }

    /// The identifier string (already user-renderable).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LevelDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Shared validation for the opaque descriptor strings.
fn validate_opaque(value: &str, code: &'static str) -> Result<String, EngineError> {
    if value.is_empty() || value.chars().any(|c| c.is_ascii_control()) {
        return Err(unsupported(code));
    }
    Ok(value.to_string())
}

/// Declared video dimension limits of one codec implementation
/// (card 1.D1-R: "min/max dims").
///
/// Per axis: `1..=MAX_DIMENSION` and `min <= max`. The cap is a
/// vocabulary bound, not a performance promise `[PROPOSAL]` — 65 535 px
/// covers every codec family's documented maximum (containers store
/// dimensions as 16-bit fields) while keeping absurd claims out of the
/// registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DimensionLimits {
    min_width: u32,
    min_height: u32,
    max_width: u32,
    max_height: u32,
}

impl DimensionLimits {
    /// Largest per-axis dimension a capability may declare.
    pub const MAX_DIMENSION: u32 = 65_535;

    /// Validate and assemble.
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] (code `codec.dimensions.invalid`)
    /// when any axis is zero or above [`MAX_DIMENSION`](Self::MAX_DIMENSION),
    /// or a minimum exceeds its maximum.
    pub fn new(
        min_width: u32,
        min_height: u32,
        max_width: u32,
        max_height: u32,
    ) -> Result<DimensionLimits, EngineError> {
        let axis_ok = |min: u32, max: u32| {
            (1..=Self::MAX_DIMENSION).contains(&min)
                && (1..=Self::MAX_DIMENSION).contains(&max)
                && min <= max
        };
        if axis_ok(min_width, max_width) && axis_ok(min_height, max_height) {
            Ok(DimensionLimits {
                min_width,
                min_height,
                max_width,
                max_height,
            })
        } else {
            Err(unsupported("codec.dimensions.invalid"))
        }
    }

    /// Smallest declared width.
    pub fn min_width(&self) -> u32 {
        self.min_width
    }

    /// Smallest declared height.
    pub fn min_height(&self) -> u32 {
        self.min_height
    }

    /// Largest declared width.
    pub fn max_width(&self) -> u32 {
        self.max_width
    }

    /// Largest declared height.
    pub fn max_height(&self) -> u32 {
        self.max_height
    }

    /// Pure predicate: do `(width, height)` fall inside the declared
    /// limits?
    pub fn accepts(&self, width: u32, height: u32) -> bool {
        self.min_width <= width
            && width <= self.max_width
            && self.min_height <= height
            && height <= self.max_height
    }
}

/// The ADR-001 hardware posture declared by one codec implementation
/// (card 1.D1-R: "ADR-001 hw-preference").
///
/// This is a *claim by the implementation about itself*; the product
/// level user policy lives in `product.toml`
/// (`features.hw_decode_policy`: software-only / prefer-hardware /
/// hardware-only, `product-config::HwDecodePolicy`) and the registry
/// combines the two — the capability says what exists, the policy
/// chooses. Variant names track ADR-001's sections:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HwPreference {
    /// Pure-Rust software implementation — the MP §2 default and
    /// ADR-001 §4's posture for every family except H.264/HEVC.
    Software,
    /// Hardware-backed implementation that ADR-001 §1 schedules ahead
    /// of software for its family (H.264/HEVC via the audited E2
    /// OS-ABI exception). Truthful active-decoder reporting is
    /// mandatory while it runs.
    HardwareFirst,
    /// Hardware-backed implementation for a software-first family
    /// (ADR-001 §4): usable when the user policy explicitly demands
    /// hardware, never the silent default.
    HardwareOptional,
}

/// What one codec implementation offers (doc 01 §2.4 "capability
/// descriptors") — the data the registry matches a
/// [`Track`](media_types::Track) against.
///
/// Absence is not rejection: an empty `profiles`/`levels` list means
/// "no profile granularity declared", and `dimensions: None` means "no
/// video dimension constraint declared" (audio/subtitle codecs). How
/// much to distrust a sloppy claim is registry policy, not vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecCapability {
    /// The codec this capability describes — equality-matches
    /// [`Track::codec`](media_types::Track::codec).
    pub codec: CodecDescriptor,
    /// Profiles the implementation can serve; opaque strings.
    pub profiles: Vec<ProfileDescriptor>,
    /// Levels the implementation can serve; opaque strings.
    pub levels: Vec<LevelDescriptor>,
    /// Video dimension limits; `None` for non-video codecs or when
    /// unconstrained.
    pub dimensions: Option<DimensionLimits>,
    /// ADR-001 hardware posture of the implementation.
    pub hardware: HwPreference,
}

impl CodecCapability {
    /// A minimal software capability for `codec` — no declared profile
    /// or level granularity, no dimension constraint.
    pub fn new(codec: CodecDescriptor) -> CodecCapability {
        CodecCapability {
            codec,
            profiles: Vec::new(),
            levels: Vec::new(),
            dimensions: None,
            hardware: HwPreference::Software,
        }
    }

    /// Pure predicate: is `profile` among the declared profiles? An
    /// empty list declares no granularity and accepts anything.
    pub fn supports_profile(&self, profile: &ProfileDescriptor) -> bool {
        self.profiles.is_empty() || self.profiles.contains(profile)
    }

    /// Pure predicate: is `level` among the declared levels? An empty
    /// list accepts anything.
    pub fn supports_level(&self, level: &LevelDescriptor) -> bool {
        self.levels.is_empty() || self.levels.contains(level)
    }

    /// Pure predicate: does a `(width, height)` track fall inside the
    /// declared dimension limits? `None` accepts anything (video
    /// dimension questions are only asked of video codecs anyway).
    pub fn accepts_dimensions(&self, width: u32, height: u32) -> bool {
        match &self.dimensions {
            Some(limits) => limits.accepts(width, height),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_descriptors_reject_empty_and_control_bytes() {
        for bad in ["", "\n", "high\x00"] {
            match ProfileDescriptor::new(bad) {
                Err(e) => {
                    assert!(matches!(e.kind, ErrorKind::Unsupported { .. }), "{bad:?}");
                    assert_eq!(e.context.code, "codec.profile.invalid");
                }
                Ok(_) => unreachable!("{bad:?} must be rejected"),
            }
            assert!(LevelDescriptor::new(bad).is_err(), "{bad:?}");
        }
        let p = ProfileDescriptor::new("high").unwrap_or_else(|e| unreachable!("{e}"));
        let l = LevelDescriptor::new("4.2").unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(p.as_str(), "high");
        assert_eq!(l.to_string(), "4.2");
        assert_eq!(
            p,
            ProfileDescriptor::new("high").unwrap_or_else(|e| unreachable!("{e}"))
        );
    }

    #[test]
    fn dimension_limits_validate_per_axis() {
        let ok = DimensionLimits::new(16, 16, 1920, 1088).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(ok.min_width(), 16);
        assert_eq!(ok.max_height(), 1088);
        assert!(ok.accepts(1920, 1088));
        assert!(!ok.accepts(1921, 1088));
        assert!(!ok.accepts(8, 1088), "below minimum");
        // Zero, over-cap and inverted axes are all rejected.
        for bad in [
            (0, 16, 1920, 1088),
            (16, 0, 1920, 1088),
            (16, 16, 0, 1088),
            (16, 16, DimensionLimits::MAX_DIMENSION + 1, 1088),
            (1921, 16, 1920, 1088),
            (16, 1089, 1920, 1088),
        ] {
            match DimensionLimits::new(bad.0, bad.1, bad.2, bad.3) {
                Err(e) => assert_eq!(e.context.code, "codec.dimensions.invalid"),
                Ok(_) => unreachable!("{bad:?} must be rejected"),
            }
        }
        // The documented cap itself is legal.
        let cap = DimensionLimits::new(
            1,
            1,
            DimensionLimits::MAX_DIMENSION,
            DimensionLimits::MAX_DIMENSION,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(cap.accepts(
            DimensionLimits::MAX_DIMENSION,
            DimensionLimits::MAX_DIMENSION
        ));
    }

    #[test]
    fn capability_predicates_treat_absence_as_unconstrained() {
        let codec = CodecDescriptor::new("av1").unwrap_or_else(|e| unreachable!("{e}"));
        let mut cap = CodecCapability::new(codec.clone());
        // Fresh capability: nothing declared, everything unconstrained.
        assert_eq!(cap.codec, codec);
        assert_eq!(cap.hardware, HwPreference::Software);
        let any_profile = ProfileDescriptor::new("main").unwrap_or_else(|e| unreachable!("{e}"));
        assert!(cap.supports_profile(&any_profile));
        assert!(cap.accepts_dimensions(7680, 4320));

        // Declared granularity narrows the predicates (data, not policy).
        cap.profiles = vec![
            ProfileDescriptor::new("main").unwrap_or_else(|e| unreachable!("{e}")),
            ProfileDescriptor::new("high").unwrap_or_else(|e| unreachable!("{e}")),
        ];
        cap.levels = vec![LevelDescriptor::new("4.2").unwrap_or_else(|e| unreachable!("{e}"))];
        cap.dimensions =
            Some(DimensionLimits::new(16, 16, 3840, 2160).unwrap_or_else(|e| unreachable!("{e}")));
        cap.hardware = HwPreference::HardwareFirst;
        assert!(cap.supports_profile(&any_profile));
        assert!(!cap.supports_profile(
            &ProfileDescriptor::new("professional").unwrap_or_else(|e| unreachable!("{e}"))
        ));
        assert!(
            !cap.supports_level(
                &LevelDescriptor::new("5.1").unwrap_or_else(|e| unreachable!("{e}"))
            )
        );
        assert!(cap.accepts_dimensions(3840, 2160));
        assert!(!cap.accepts_dimensions(4096, 2160));
        assert_eq!(cap.hardware, HwPreference::HardwareFirst);
    }
}
