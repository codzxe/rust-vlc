//! The font stack seam (issue #17, gap G-5): font SOURCE resolution for
//! text subtitles — a provider/stack abstraction with an explicit
//! fallback chain, plus the glyph-metrics/raster seam the render paths
//! consume (`font::pipeline`).
//!
//! ## Why a seam, not a stack (the honest v1)
//!
//! Today the ASS render path rasterizes placeholder block glyphs
//! (`ass::render`'s monospace-widest-char metrics — the G-5 gap the
//! phase-5 records audit registered). A REAL font stack needs pure-Rust
//! font parsing, shaping and rasterization crates; admitting them is a
//! Lead decision (AGENTS §3.2: language, source, transitive graph,
//! license, maintenance, provenance — the full audit record lives at
//! `docs/dependencies/records/font-stack-0.2.10.md`, marked
//! **[PROPOSAL — Lead approval pending]**). Everything in this module
//! is therefore the CONTRACT the future implementation plugs into:
//!
//! * [`FontProvider`] — one source of faces. The bundled default face
//!   and the (sibling-card) MKV-attachment wiring each implement it;
//!   `MemoryFontProvider` is the in-crate provider a demuxer adapter
//!   builds from attached font files [PROPOSAL: the concrete
//!   attachments hookup lands with the MKV card — the seam is the
//!   deliverable here].
//! * [`FontStack`] — ordered providers + the documented fallback chain
//!   ([`FontStack::resolve`] returns the face AND the audit trail of
//!   every provider consulted, so callers can record honest
//!   degradation instead of silent fallbacks).
//! * `font::pipeline::GlyphPipeline` — the shaping/raster seam.
//!   `font::pipeline::MonospaceCellPipeline` is the no-new-deps
//!   implementation (today's placeholder glyphs, now decoration-aware
//!   for the ASS override-tag subset); the audited crate stack
//!   implements the same trait behind the [PROPOSAL] admission.
//!
//! ## Resolution policy (libass/VSFilter-compatible subset)
//!
//! 1. Providers are consulted in ADDED order — the demuxer's attached
//!    fonts are added first, so ATTACHED beats the bundled default
//!    (the VSFilter embedded-fonts-first convention).
//! 2. Inside a provider, family matching is ASCII-case-insensitive
//!    (the `Arial`/`arial` fold every font system performs); exact
//!    style (bold/italic) wins over a style-synthesized face.
//! 3. A family hit with a style miss still returns the face — the
//!    resolution carries [`GlyphSynthesis`] flags (`FauxBold`/
//!    `FauxItalic`) the pipeline honors by synthetic emboldening/
//!    obliquing, the same fallback real stacks perform.
//!
//! ## No bundled face yet (recorded, not hidden)
//!
//! The only font file in-repo is `ui/src/assets/fonts/
//! nunito-latin-var.woff2` (OFL 1.1) — a WOFF2 container, which needs
//! Brotli decompression before any parser can read it (a second new
//! dependency, out of scope). The engine default face (raw TTF/OTF,
//! permissive license) is a Lead admission decision recorded in the
//! audit doc; until it lands [`FontStack::empty`] is the stack the
//! render paths use, and every `\fn`-driven lookup misses honestly.
//!
//! ## No panics
//!
//! Typed refusals only (MP §8): empty font data, unknown/compressed
//! container magic, zero-size metrics. No `unsafe` (workspace deny).

pub mod pipeline;

pub use pipeline::{GlyphDecorations, GlyphMask, GlyphMetrics, GlyphPipeline, MetricsError,
                   MonospaceCellPipeline, MAX_FONT_SIZE_PX};

/// The container format sniffed from a font file's leading bytes.
///
/// Sniffing is magic-prefix only — full validation belongs to the
/// parsing crate behind the [PROPOSAL] admission; this is enough to
/// route/refuse data honestly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFormat {
    /// TrueType sfnt (`00 01 00 00`).
    TrueType,
    /// OpenType with CFF outlines (`OTTO`).
    OpenTypeCff,
    /// TrueType/OpenType collection (`ttcf`).
    Collection,
}

/// Typed failures of font-data intake (MP §8: data, never a panic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontFaceError {
    /// `FontFace::from_bytes` got zero bytes.
    EmptyData,
    /// The leading magic matches no sfnt flavour this seam accepts.
    UnknownContainer {
        /// The first up-to-4 bytes, for the damage ledger.
        magic: [u8; 4],
    },
    /// A compressed web container (WOFF/WOFF2): reading it needs a
    /// decompression dependency that is NOT admitted — typed refusal,
    /// not a silent miss (see the module docs' bundled-face note).
    CompressedContainer {
        /// The sniffed magic, for the damage ledger.
        magic: [u8; 4],
    },
}

impl std::fmt::Display for FontFaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontFaceError::EmptyData => f.write_str("font face has zero bytes of data"),
            FontFaceError::UnknownContainer { magic } => write!(
                f,
                "font container magic {:02x?} is not TrueType/OTTO/ttcf",
                magic
            ),
            FontFaceError::CompressedContainer { magic } => write!(
                f,
                "font container magic {:02x?} is WOFF/WOFF2 — decompression is out of scope (not admitted)",
                magic
            ),
        }
    }
}

impl std::error::Error for FontFaceError {}

/// One typeface request from the render path (an ASS `\fn` + the
/// style's bold/italic flags, a WebVTT cue's future hook, …).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontRequest {
    /// The requested family name (matched ASCII-case-insensitively).
    pub family: String,
    /// Bold weight requested.
    pub bold: bool,
    /// Italic slant requested.
    pub italic: bool,
}

impl FontRequest {
    /// Construct a request.
    #[must_use]
    pub fn new(family: &str, bold: bool, italic: bool) -> Self {
        Self {
            family: family.to_owned(),
            bold,
            italic,
        }
    }

    /// ASCII-case-insensitive family comparison (the fold every font
    /// system applies; non-ASCII bytes compare exactly).
    fn family_eq(&self, other: &str) -> bool {
        self.family.eq_ignore_ascii_case(other)
    }
}

/// One available font face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFace {
    /// The family name as the provider knows it.
    family: String,
    /// The face's own bold flag.
    bold: bool,
    /// The face's own italic flag.
    italic: bool,
    /// The raw font bytes (container per [`Self::format`]).
    data: Vec<u8>,
    /// The sniffed container format.
    format: FontFormat,
}

impl FontFace {
    /// Intake a face from raw bytes, sniffing the container.
    ///
    /// # Errors
    /// [`FontFaceError`] — empty data, an unknown magic, or a
    /// compressed WOFF/WOFF2 container (typed refusal, module docs).
    pub fn from_bytes(
        family: &str,
        bold: bool,
        italic: bool,
        data: Vec<u8>,
    ) -> Result<Self, FontFaceError> {
        if data.is_empty() {
            return Err(FontFaceError::EmptyData);
        }
        let format = sniff_format(&data)?;
        Ok(Self {
            family: family.to_owned(),
            bold,
            italic,
            data,
            format,
        })
    }

    /// The family name.
    #[must_use]
    pub fn family(&self) -> &str {
        &self.family
    }

    /// The face's bold flag.
    #[must_use]
    pub const fn bold(&self) -> bool {
        self.bold
    }

    /// The face's italic flag.
    #[must_use]
    pub const fn italic(&self) -> bool {
        self.italic
    }

    /// The raw font bytes.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// The sniffed container format.
    #[must_use]
    pub const fn format(&self) -> FontFormat {
        self.format
    }
}

/// Sniff the sfnt-family container magic (4 bytes minimum — shorter
/// data is zero-padded into the reported magic and refused: no real
/// face header fits in under 4 bytes).
fn sniff_format(data: &[u8]) -> Result<FontFormat, FontFaceError> {
    let mut magic = [0u8; 4];
    if data.len() >= 4 {
        magic.copy_from_slice(&data[..4]);
    } else {
        let n = data.len();
        magic[..n].copy_from_slice(data);
        return Err(FontFaceError::UnknownContainer { magic });
    }
    match &magic {
        [0x00, 0x01, 0x00, 0x00] => Ok(FontFormat::TrueType),
        b"OTTO" => Ok(FontFormat::OpenTypeCff),
        b"ttcf" => Ok(FontFormat::Collection),
        b"wOFF" | b"wOF2" => Err(FontFaceError::CompressedContainer { magic }),
        _ => Err(FontFaceError::UnknownContainer { magic }),
    }
}

/// One source of faces. Implemented by the bundled default and by the
/// demuxer's attachment adapter; `MemoryFontProvider` is the concrete
/// in-crate implementation for both.
///
/// The lookup contract: return the best face this provider has for the
/// request, or `None`. Family matching is the provider's duty (the
/// case fold is a [`FontRequest`] helper); style synthesis is the
/// STACK's duty (it compares the request against the returned face).
pub trait FontProvider {
    /// Stable provider name for resolution chains and damage ledgers.
    fn name(&self) -> &'static str;
    /// Best face for `request`, or `None`.
    fn lookup(&self, request: &FontRequest) -> Option<FontFace>;
}

/// A face set held in memory — the provider the bundled default and
/// the (future) MKV-attachment adapter both use
/// ([PROPOSAL]: the attachment hookup lands with the demux card).
///
/// A provider is family-scoped by default; [`Self::catch_all`] marks
/// it as answering ANY request with its first face (the bundled
/// default's last-resort role — the resolution trail still records
/// `family_matched: false` so callers can log the fallback).
#[derive(Debug, Clone, Default)]
pub struct MemoryFontProvider {
    name: &'static str,
    faces: Vec<FontFace>,
    catch_all: bool,
}

impl MemoryFontProvider {
    /// Construct an empty provider with a stable name.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            faces: Vec::new(),
            catch_all: false,
        }
    }

    /// Mark this provider as a catch-all: when no family matches, its
    /// first face answers (the bundled default's role).
    #[must_use]
    pub const fn catch_all(mut self) -> Self {
        self.catch_all = true;
        self
    }

    /// Add a face.
    #[must_use]
    pub fn with_face(mut self, face: FontFace) -> Self {
        self.faces.push(face);
        self
    }

    /// The number of faces held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// Whether the provider holds no faces.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
}

impl FontProvider for MemoryFontProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn lookup(&self, request: &FontRequest) -> Option<FontFace> {
        // Family first (ASCII fold), then exact style, then any style
        // of the family (the stack records the synthesis gap). An
        // exact match at ANY position beats a family-only hit seen
        // earlier (the loop keeps going until one appears).
        let mut family_fallback = None;
        for f in &self.faces {
            if !request.family_eq(&f.family) {
                continue;
            }
            if f.bold == request.bold && f.italic == request.italic {
                return Some(f.clone());
            }
            if family_fallback.is_none() {
                family_fallback = Some(f.clone());
            }
        }
        family_fallback.or_else(|| {
            if self.catch_all {
                self.faces.first().cloned()
            } else {
                None
            }
        })
    }
}

/// What the pipeline must synthesize because the matched face lacks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphSynthesis {
    /// The face is not bold; strokes must be synthetically emboldened.
    FauxBold,
    /// The face is not italic; glyphs must be synthetically obliqued.
    FauxItalic,
}

/// One provider consultation, as recorded in a [`FontResolution`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderStep {
    /// This provider had no face for the request.
    Missed {
        /// The provider's stable name.
        provider: &'static str,
    },
    /// This provider supplied the resolution's face.
    Supplied {
        /// The provider's stable name.
        provider: &'static str,
        /// Whether the FAMILY matched the request (a miss means the
        /// provider's default face was returned — family fallback).
        family_matched: bool,
    },
}

/// The outcome of [`FontStack::resolve`]: the face (if any), the
/// synthesis the pipeline must apply, and the full consultation trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontResolution {
    /// The winning face, or `None` when every provider missed (the
    /// caller falls back to placeholder metrics — honestly recorded).
    pub face: Option<FontFace>,
    /// Style synthesis the pipeline must apply to `face`.
    pub synthesis: Vec<GlyphSynthesis>,
    /// Every provider consulted, in order.
    pub chain: Vec<ProviderStep>,
}

impl FontResolution {
    /// Whether the request's family was actually found (vs. a default
    /// or nothing at all).
    #[must_use]
    pub fn family_matched(&self) -> bool {
        self.chain
            .iter()
            .any(|s| matches!(s, ProviderStep::Supplied { family_matched: true, .. }))
    }
}

/// The ordered provider set: font source resolution with an audit
/// trail. Providers are consulted in ADDED order (attached fonts
/// first — the VSFilter convention); the bundled default is added
/// last via [`FontStack::with_bundled`].
#[derive(Debug, Clone, Default)]
pub struct FontStack {
    providers: Vec<MemoryFontProvider>,
}

impl FontStack {
    /// The empty stack — every lookup misses (today's honest state:
    /// no face is bundled yet, module docs).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// A stack with exactly one provider (the common construction).
    #[must_use]
    pub fn with_provider(provider: MemoryFontProvider) -> Self {
        Self::empty().add_provider(provider)
    }

    /// Add a provider (consulted BEFORE the providers already added).
    /// Demuxer attachment adapters call this with their face set.
    #[must_use]
    pub fn add_provider(mut self, provider: MemoryFontProvider) -> Self {
        self.providers.insert(0, provider);
        self
    }

    /// Add the engine's bundled default face as the LAST-resort
    /// provider (named `"bundled"`).
    ///
    /// [PROPOSAL — Lead approval pending]: no bundled face exists
    /// yet; the audit record documents the admission decision.
    #[must_use]
    pub fn with_bundled(mut self, face: FontFace) -> Self {
        let bundled = MemoryFontProvider::new("bundled")
            .catch_all()
            .with_face(face);
        self.providers.push(bundled);
        self
    }

    /// Resolve a request through the provider chain.
    ///
    /// The trail is complete: every provider appears in `chain` (a
    /// `Missed` step for each miss), the winner is the FIRST hit, and
    /// `synthesis` lists the style emulation the pipeline must apply.
    #[must_use]
    pub fn resolve(&self, request: &FontRequest) -> FontResolution {
        let mut chain = Vec::with_capacity(self.providers.len());
        for provider in &self.providers {
            match provider.lookup(request) {
                None => chain.push(ProviderStep::Missed {
                    provider: provider.name(),
                }),
                Some(face) => {
                    let family_matched = request.family_eq(&face.family);
                    let mut synthesis = Vec::new();
                    if request.bold && !face.bold {
                        synthesis.push(GlyphSynthesis::FauxBold);
                    }
                    if request.italic && !face.italic {
                        synthesis.push(GlyphSynthesis::FauxItalic);
                    }
                    chain.push(ProviderStep::Supplied {
                        provider: provider.name(),
                        family_matched,
                    });
                    return FontResolution {
                        face: Some(face),
                        synthesis,
                        chain,
                    };
                }
            }
        }
        FontResolution {
            face: None,
            synthesis: Vec::new(),
            chain,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FontFace, FontFaceError, FontFormat, FontProvider, FontRequest, FontStack,
        GlyphSynthesis, MemoryFontProvider, ProviderStep,
    };
    use testkit::Must;

    /// Minimal TrueType-magic payload (sniffing only — the parse
    /// crate validates the rest, module docs).
    fn ttf_bytes() -> Vec<u8> {
        let mut v = vec![0x00, 0x01, 0x00, 0x00];
        v.extend_from_slice(b"rest-of-the-face");
        v
    }

    fn face(family: &str, bold: bool, italic: bool) -> FontFace {
        FontFace::from_bytes(family, bold, italic, ttf_bytes()).must()
    }

    #[test]
    fn container_magic_sniffing_table() {
        assert_eq!(
            FontFace::from_bytes("x", false, false, ttf_bytes())
                .must()
                .format(),
            FontFormat::TrueType
        );
        let otto = FontFace::from_bytes("x", false, false, b"OTTOcff...".to_vec()).must();
        assert_eq!(otto.format(), FontFormat::OpenTypeCff);
        let ttcf = FontFace::from_bytes("x", false, false, b"ttcf00..".to_vec()).must();
        assert_eq!(ttcf.format(), FontFormat::Collection);
        // Unknown magic + empty are typed refusals.
        assert_eq!(
            FontFace::from_bytes("x", false, false, b"nope".to_vec()),
            Err(FontFaceError::UnknownContainer { magic: *b"nope" })
        );
        assert_eq!(
            FontFace::from_bytes("x", false, false, Vec::new()),
            Err(FontFaceError::EmptyData)
        );
        // WOFF/WOFF2 are compressed containers: typed refusal (the
        // decompression dependency is not admitted — module docs).
        assert_eq!(
            FontFace::from_bytes("x", false, false, b"wOF2zz".to_vec()),
            Err(FontFaceError::CompressedContainer { magic: *b"wOF2" })
        );
        assert_eq!(
            FontFace::from_bytes("x", false, false, b"wOFFzz".to_vec()),
            Err(FontFaceError::CompressedContainer { magic: *b"wOFF" })
        );
        // A 3-byte file cannot carry any magic — unknown (zero-padded
        // magic in the ledger), not a panic.
        assert_eq!(
            FontFace::from_bytes("x", false, false, vec![0x00, 0x01, 0x00]),
            Err(FontFaceError::UnknownContainer {
                magic: [0x00, 0x01, 0x00, 0x00]
            })
        );
    }

    #[test]
    fn family_matching_folds_ascii_case() {
        let provider = MemoryFontProvider::new("test").with_face(face("Arial", false, false));
        for asked in ["Arial", "arial", "ARIAL", "aRiAl"] {
            let hit = provider.lookup(&FontRequest::new(asked, false, false));
            assert!(hit.is_some(), "family {asked} must match");
        }
        // Non-ASCII folds do NOT apply (byte-exact comparison).
        assert!(provider
            .lookup(&FontRequest::new("Ârial", false, false))
            .is_none());
        assert!(provider
            .lookup(&FontRequest::new("DejaVu Sans", false, false))
            .is_none());
    }

    #[test]
    fn provider_prefers_exact_style_then_family() {
        let provider = MemoryFontProvider::new("test")
            .with_face(face("F", false, false))
            .with_face(face("F", true, false));
        // Exact style match.
        let hit = provider.lookup(&FontRequest::new("F", true, false)).must();
        assert!(hit.bold() && !hit.italic());
        // Style miss inside a family hit: the regular face returns
        // (the STACK records the synthesis).
        let hit = provider.lookup(&FontRequest::new("F", true, true)).must();
        assert!(!hit.bold() && !hit.italic());
    }

    #[test]
    fn empty_stack_misses_with_empty_chain() {
        let stack = FontStack::empty();
        let r = stack.resolve(&FontRequest::new("Arial", false, false));
        assert!(r.face.is_none());
        assert!(r.chain.is_empty());
        assert!(!r.family_matched());
    }

    #[test]
    fn resolution_walks_providers_in_added_order() {
        // Attached (added first) shadows the bundled default.
        let attached =
            MemoryFontProvider::new("attachments").with_face(face("Arial", false, false));
        let bundled =
            MemoryFontProvider::new("bundled").with_face(face("Lumen Default", false, false));
        let stack = FontStack::with_provider(bundled).add_provider(attached);
        let r = stack.resolve(&FontRequest::new("Arial", false, false));
        assert_eq!(r.face.as_ref().must().family(), "Arial");
        assert!(r.family_matched());
        assert_eq!(
            r.chain,
            vec![ProviderStep::Supplied {
                provider: "attachments",
                family_matched: true
            }]
        );
        // A family only the default has: attached misses (recorded),
        // bundled supplies, family still matched.
        let r = stack.resolve(&FontRequest::new("Lumen Default", false, false));
        assert_eq!(r.face.as_ref().must().family(), "Lumen Default");
        assert_eq!(
            r.chain,
            vec![
                ProviderStep::Missed {
                    provider: "attachments"
                },
                ProviderStep::Supplied {
                    provider: "bundled",
                    family_matched: true
                }
            ]
        );
        // Nothing anywhere: full miss trail, face None.
        let r = stack.resolve(&FontRequest::new("Comic Sans", false, false));
        assert!(r.face.is_none());
        assert_eq!(
            r.chain,
            vec![
                ProviderStep::Missed {
                    provider: "attachments"
                },
                ProviderStep::Missed {
                    provider: "bundled"
                }
            ]
        );
    }

    #[test]
    fn synthesis_flags_report_the_style_gap() {
        let regular_only = MemoryFontProvider::new("test").with_face(face("F", false, false));
        let stack = FontStack::with_provider(regular_only);
        let r = stack.resolve(&FontRequest::new("F", true, true));
        assert_eq!(
            r.synthesis,
            vec![GlyphSynthesis::FauxBold, GlyphSynthesis::FauxItalic]
        );
        // Bold face present, italic requested: only the italic gap.
        let bold_face = MemoryFontProvider::new("test").with_face(face("F", true, false));
        let r = FontStack::with_provider(bold_face).resolve(&FontRequest::new("F", true, true));
        assert_eq!(r.synthesis, vec![GlyphSynthesis::FauxItalic]);
        // Exact: no synthesis.
        let r = stack.resolve(&FontRequest::new("F", false, false));
        assert!(r.synthesis.is_empty());
    }

    #[test]
    fn with_bundled_places_the_default_last() {
        let stack = FontStack::empty().with_bundled(face("Lumen Default", false, false));
        let r = stack.resolve(&FontRequest::new("Anything", true, false));
        // Family fallback: the bundled default answers, family did
        // NOT match, and bold must be synthesized.
        assert_eq!(r.face.as_ref().must().family(), "Lumen Default");
        assert!(!r.family_matched());
        assert_eq!(r.synthesis, vec![GlyphSynthesis::FauxBold]);
        assert_eq!(
            r.chain,
            vec![ProviderStep::Supplied {
                provider: "bundled",
                family_matched: false
            }]
        );
    }

    #[test]
    fn provider_len_and_emptiness() {
        let empty = MemoryFontProvider::new("t");
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        let one = empty.clone().with_face(face("F", false, false));
        assert!(!one.is_empty());
        assert_eq!(one.len(), 1);
        assert_eq!(empty.lookup(&FontRequest::new("F", false, false)), None);
    }
}
