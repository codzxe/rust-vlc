//! Track switching mid-play (card 3.A3; design: worklog W1-17): the
//! selector vocabulary, the seven-step switch flow, decoder-instance
//! recycling and the class-S subtitle seam.
//!
//! # Generation semantics — a track switch is a PIPELINE GENERATION
//!
//! `tracks.set-video-track` / `tracks.set-audio-track` (class A) mint
//! a pipeline generation exactly like a seek: the switched-away
//! track's resident packets and any decoder-buffered frames are stale
//! work, and doc 03 §5's boundary ("after the session publishes
//! generation G and the flush of each queue has returned, no packet or
//! frame with stamp ≠ G can be popped, decoded-into-output, presented
//! or mixed") is the property the AC's no-stale-frames assertions
//! lean on. Doc 03 §5 names `SwitchTrack` in the mint list; doc 05
//! §4.2 lists both set-*-track commands as gen-bumping. It is **not**
//! an epoch event: the source, demuxer, session identity and
//! wall-clock continuity all survive a track switch (epoch invariant
//! S4 scopes what dies — none of it does).
//!
//! It is lighter than a seek in *work*, not in *identity*: the
//! demuxer never moves (no re-anchor), only the switched kind's limb
//! is rebuilt, and the unchanged kind keeps playing through the mint
//! (sound because the per-limb generation floor — see
//! [`crate::pipeline`] — keeps the unchanged kind's G-stamped
//! residents presentable while its queue's boundary only ratchets at
//! its own flushes).
//!
//! # The seven steps, as implemented
//!
//! 1. **Accept** (owner: `session::apply_command`, before the table):
//!    payload guard, then selector resolution ([`resolve_selector`])
//!    against the live track list — an unresolvable *track selector*
//!    is a request-validity question and rejects **before** the mint
//!    (no generation bump, no state-changed, `Rejected` receipt with
//!    [`crate::state::RejectionReason::TrackNotFound`]). This is the
//!    deliberate asymmetry with seek, where an unresolvable target
//!    still applies its row and fails via `SeekFailed`: doc 05 §2.5's
//!    `not-found` is a rejection-class error (the menu checkmark must
//!    not wait on a dead id), and not bumping for a no-op keeps rapid
//!    UI cycling cheap. State legality for A/V flows through the
//!    table's `SwitchTrack` rows.
//! 2. **Bump** (the row's `BumpPipeline`): mint G+1, publish the
//!    token. The wire `gen` moves at accept; the demux driver stamps
//!    every packet from the next tick on with G+1 — including the
//!    unchanged kind's.
//! 3. **Limb swap** ([`pipeline_switch`], the `SwitchPipeline`
//!    effect's interpreter): for the switched kind only,
//!    prepare-before-commit — obtain the new decoder first (cache hit
//!    ⇒ `configure` as reset; miss ⇒ the provider, whose failure is
//!    **fatal** per the recorded wire() contract "a selection that
//!    fails after promising is fatal, not a skip" — the old limb is
//!    untouched until the new decoder exists), then retire the old
//!    limb (flush + close its queue, drain its decoder to `Drained`
//!    through a generation-checking sink, park the decoder *instance*
//!    in the per-session cache), then install the new limb (fresh
//!    queue published at G+1). The audio kind also freezes the clock
//!    at the old position now (the audio master's samples stop). A
//!    retired limb is not "newly drained" — no `end-of-track` fires
//!    for it.
//! 4. **No demuxer movement**: the demuxer stands where it stands;
//!    the new track's interleaved packets flow into the new limb from
//!    the current position on the next tick. A track whose declared
//!    span ended before the current position simply delivers nothing
//!    — the switch completes via the provably-never path at EOF and
//!    the limb stays silent (accept-and-complete-honestly; doc 05
//!    §2.5's frozen error list has no shape for refusing it).
//! 5. **Re-prime** (the tick path): completion is observed like the
//!    seek pre-roll — the new limb delivered ≥ 1 frame under the
//!    switch's generation, or can provably never (EOF, nothing
//!    resident, decoder `Drained`). While the new limb has not yet
//!    delivered, it is **exempt from the underrun check and from
//!    priming** — a switch is not a starvation; the input can keep
//!    up, we changed which limb we listen to (without the carve-out
//!    every audio switch from `Playing` would flash
//!    playing→buffering→playing).
//! 6. **Clock re-anchor** ([`complete_switch`], audio kind only):
//!    re-anchor on the new track's **first delivered frame's own
//!    pts** — anchor on truth, not on the freeze position. A video
//!    switch never touches the clock (audio keeps playing; the
//!    scheduler snaps on the new track's first frame — video-engine
//!    policy, out of 3.A3 scope).
//! 7. **Report** (the tick path): the terminal
//!    [`EngineEventKind::TrackSelectionChanged`](crate::event::EngineEventKind)
//!    under the **switch's own generation** (it may legitimately
//!    diverge from the live generation when a follow-up mint raced
//!    the completion; exactly one terminal per completed generation
//!    is doc 05 §4.3 invariant 4 — the terminal joins the outbox's
//!    drain exemption). The new track's first frame is delivered to
//!    the sink **before** the terminal (the frame's delivery IS the
//!    completion fact).
//!
//! # Supersession
//!
//! Per-kind single in-flight slots on the session: a second switch of
//! the same kind accepted before completion replaces the slot — the
//! older switch is silent (no terminal). Different kinds are
//! independent. A seek arriving mid-switch subsumes the switch's
//! re-prime (both watch "first frame under current gen"); a switch
//! arriving mid-seek re-arms the seek's observation window for the
//! swapped limb — the seek completes on the new track's first frame,
//! which is what will actually present.
//!
//! # The class-S subtitle seam — and its honest stubs
//!
//! `tracks.set-subtitle-track` mints **nothing** (doc 05 §2.5:
//! "Subtitle switching is a selection over already-parsed streams; no
//! A/V pipeline invalidation, hence class S"). It is
//! session-attribute work: resolve the selector, swap the (optional)
//! subtitle limb at the **current** generation — queue-close +
//! decoder-cache recycle + fresh queue, no mint — and answer
//! `Completed` in the receipt. **What is stub today, marked
//! honestly (D-5):**
//!
//! 1. **No demuxer emits subtitle tracks**: the AVI demuxer skips
//!    `txts`/other stream types with damage "unsupported-stream-type"
//!    (`demux/src/avi/header.rs`); there is no MP4/MKV demuxer. Real
//!    subtitle tracks arrive with the Phase-5 subtitle card.
//! 2. **No subtitle decoder exists** (`codecs` carries MJPEG + PCM).
//! 3. Therefore, on every container the current demuxer can parse, a
//!    subtitle `Id` selector resolves to the typed `TrackNotFound`
//!    refusal (truthful menus), and `None`/`Auto` succeed trivially
//!    as a deselect. The engine-level path (wire → queue → decode →
//!    `Frame::Subtitle` → swap without restart) is exercised on
//!    synthetic tracks by `tests/track_switch.rs`; real-format
//!    subtitle coverage belongs to the Phase-5 card and its fixture.
//!
//! # Secondary audio
//!
//! `tracks.set-secondary-audio` is **out of scope** for 3.A3 (it
//! needs a second concurrent audio limb + the audio-engine mix; doc
//! 05 marks it `[PROPOSAL]` — a follow-up card; the wire
//! `TrackChangeKind::SecondaryAudio` arm already exists).

use commands::envelope::RequestId;
use media_clock::{NANO_BASE, PlaybackClock};
use media_types::time::Timestamp;
use media_types::{EngineError, MediaKind, Track, TrackId};

use crate::pipeline::Pipeline;

/// A media kind with a queue lane (audit F-1, error-surface
/// 2026-10-23 §3): the three kinds the pipeline wires.
///
/// This is the vocabulary [`QueuePlan::for_kind`](crate::pipeline::QueuePlan::for_kind)
/// indexes. It replaces the cross-module "wired kinds" convention the
/// audit flagged as the crate's one REAL RISK (a `MediaKind`-keyed
/// `Option` lookup whose caller carried a production `expect`): a
/// lane-less kind is now **unrepresentable** at the lane-building
/// seams — [`Pipeline::wire`](crate::pipeline::Pipeline::wire)
/// iterates `WiredKind` values and
/// [`SessionCommandKind::selection_kind`](crate::session::SessionCommandKind::selection_kind)
/// answers `Option<WiredKind>` — so the invariant lives in the type
/// system, not in a runtime abort reachable by a future `MediaKind`
/// variant or a new `build_limb` caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WiredKind {
    /// The audio lane.
    Audio,
    /// The video lane.
    Video,
    /// The subtitle lane (the optional kind — class S).
    Subtitle,
}

impl WiredKind {
    /// The full-kind view of this lane: the track-list vocabulary
    /// ([`default_track`], track-kind comparisons, event payloads)
    /// stays [`MediaKind`]; this is the widening direction, total by
    /// construction.
    #[must_use]
    pub fn as_media_kind(self) -> MediaKind {
        match self {
            WiredKind::Audio => MediaKind::Audio,
            WiredKind::Video => MediaKind::Video,
            WiredKind::Subtitle => MediaKind::Subtitle,
        }
    }
}

/// The typed refusal of the [`MediaKind`] → [`WiredKind`]
/// conversion: the offered kind has no queue lane (design A3 — data
/// is not wired; the exhaustive truth, never a silent audio
/// fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnwiredKind {
    /// The lane-less kind that was offered.
    pub kind: MediaKind,
}

impl std::fmt::Display for UnwiredKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.kind {
            MediaKind::Audio => "audio",
            MediaKind::Video => "video",
            MediaKind::Subtitle => "subtitle",
            MediaKind::Data => "data",
        };
        write!(
            f,
            "the {name} kind has no queue lane (design A3: lane-less kinds are never wired)"
        )
    }
}

impl std::error::Error for UnwiredKind {}

impl TryFrom<MediaKind> for WiredKind {
    type Error = UnwiredKind;

    /// The single `MediaKind` → `WiredKind` conversion point (audit
    /// F-1): the one place a full kind meets the lane vocabulary.
    ///
    /// # Errors
    /// [`UnwiredKind`] for a lane-less kind (`MediaKind::Data` — and
    /// any future lane-less variant, which this match will refuse to
    /// compile without) — the typed truth where the retired
    /// convention aborted at runtime.
    fn try_from(kind: MediaKind) -> Result<WiredKind, UnwiredKind> {
        match kind {
            MediaKind::Audio => Ok(WiredKind::Audio),
            MediaKind::Video => Ok(WiredKind::Video),
            MediaKind::Subtitle => Ok(WiredKind::Subtitle),
            MediaKind::Data => Err(UnwiredKind { kind }),
        }
    }
}

/// One track-selection request, engine-side (doc 05 §2.5's
/// `TrackSelector` — the wire type converts at the session boundary,
/// the `SeekTarget` precedent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackSelector {
    /// Select the track with this id.
    Id {
        /// Track handle.
        id: TrackId,
    },
    /// Select no track of the kind (deselect).
    None,
    /// Apply the product's track-selection rules. At switch time this
    /// re-resolves to the container-default track of the kind (the
    /// `default_on` flag, else first-of-kind in stream order); the
    /// full product.toml rules engine is a `[PROPOSAL]` deferred to
    /// the config card (4.E3-era).
    Auto,
}

impl TrackSelector {
    /// The wire form (doc 05 §2.2 `TrackSelector`).
    #[must_use]
    pub fn as_wire(self) -> commands::catalog::types::TrackSelector {
        match self {
            TrackSelector::Id { id } => commands::catalog::types::TrackSelector::Id {
                id: commands::catalog::types::TrackId(id.handle()),
            },
            TrackSelector::None => commands::catalog::types::TrackSelector::None,
            TrackSelector::Auto => commands::catalog::types::TrackSelector::Auto,
        }
    }

    /// The engine form of a wire selector (the session-boundary
    /// conversion; an id is a bare handle, never interpreted).
    #[must_use]
    pub fn from_wire(selector: commands::catalog::types::TrackSelector) -> TrackSelector {
        match selector {
            commands::catalog::types::TrackSelector::Id { id } => TrackSelector::Id {
                id: TrackId::new(id.0),
            },
            commands::catalog::types::TrackSelector::None => TrackSelector::None,
            commands::catalog::types::TrackSelector::Auto => TrackSelector::Auto,
        }
    }
}

/// Why a selector was refused (the typed, request-validity half of
/// step 1). Maps to the wire `not-found` rejection family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackSelectionError {
    /// The id is not a live track of the requested kind (unknown id,
    /// or a track of another kind — doc 05 V5: reference validity is
    /// checked at execution).
    NotFound {
        /// The requested handle.
        id: TrackId,
    },
}

impl std::fmt::Display for TrackSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackSelectionError::NotFound { id } => {
                write!(f, "no live {}-kind track", id.handle())
            }
        }
    }
}

/// A resolved selection: the track to switch to, or a deselect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedSelection {
    /// Switch to this track.
    Track(TrackId),
    /// Deselect the kind (retire its limb; the kind goes silent).
    Deselect,
}

/// The container-default track of `kind` — the `default_on` flag,
/// else the first of the kind in stream order. This is both the
/// open-time `Auto` answer (wire's initial selection) and the
/// switch-time re-resolution.
#[must_use]
pub(crate) fn default_track(tracks: &[Track], kind: MediaKind) -> Option<&Track> {
    tracks
        .iter()
        .find(|track| track.kind == kind && track.flags.default_on)
        .or_else(|| tracks.iter().find(|track| track.kind == kind))
}

/// Step 1's semantic half: resolve `selector` against the live track
/// list for `kind`.
///
/// # Errors
/// [`TrackSelectionError::NotFound`] for an `Id` that is not a live
/// track of the requested kind — the typed pre-table rejection.
pub(crate) fn resolve_selector(
    selector: TrackSelector,
    tracks: &[Track],
    kind: MediaKind,
) -> Result<ResolvedSelection, TrackSelectionError> {
    match selector {
        TrackSelector::Id { id } => match tracks.iter().find(|track| track.id == id) {
            Some(track) if track.kind == kind => Ok(ResolvedSelection::Track(id)),
            _ => Err(TrackSelectionError::NotFound { id }),
        },
        TrackSelector::None => Ok(ResolvedSelection::Deselect),
        TrackSelector::Auto => Ok(match default_track(tracks, kind) {
            Some(track) => ResolvedSelection::Track(track.id),
            None => ResolvedSelection::Deselect,
        }),
    }
}

/// The switch a session is currently executing (steps 5–7): which
/// kind, which track won, and the generation that owns the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InFlightTrackSwitch {
    /// The switched kind (audio or video; subtitle is class S and
    /// never in flight).
    pub kind: MediaKind,
    /// The now-selected track of that kind (`None` = deselected).
    pub selected: Option<TrackId>,
    /// The generation the switch runs under (the mint at accept).
    pub generation: media_types::GenerationId,
    /// The request that caused the switch, when attributable (doc 05
    /// §3.1 `origin`).
    pub origin: Option<RequestId>,
}

/// What [`pipeline_switch`] produced for the session's observability
/// totals and the in-flight bookkeeping.
#[derive(Debug, Default)]
pub(crate) struct SwitchOutcome {
    /// Frames dropped as stale while draining the retired limb's
    /// decoder (doc 03 §5 checking point 5).
    pub stale_frames_flushed: u32,
    /// The new decoder came from the per-session cache (D-1: the
    /// multi-MB MJPEG `DecodeContext` survives the switch).
    pub decoder_reused: bool,
    /// The provider built a fresh decoder for the new track.
    pub decoder_built: bool,
}

/// Step 3 against the live pipeline: the limb swap for `kind` under
/// `generation` (the mint the row's `BumpPipeline` just published).
/// Pure with respect to the session — the caller records/emits from
/// the outcome. See [`Pipeline::switch_limbs`] for the
/// prepare-before-commit order.
///
/// # Errors
/// The provider's typed error (a decoder build failure for the
/// newly-selected track is fatal per the wire() contract — the old
/// limb is untouched, the caller enters `Error`); a cached decoder's
/// configure refusal likewise.
pub(crate) fn pipeline_switch(
    pipeline: &mut Pipeline,
    generation: media_types::GenerationId,
    kind: WiredKind,
    resolved: ResolvedSelection,
    provider: &dyn crate::pipeline::DecoderProvider,
    plan: &crate::pipeline::QueuePlan,
    flush_budget: u32,
) -> Result<SwitchOutcome, EngineError> {
    let selected = match resolved {
        ResolvedSelection::Track(id) => Some(id),
        ResolvedSelection::Deselect => None,
    };
    pipeline.switch_limbs(generation, kind, selected, provider, plan, flush_budget)
}

/// Steps 6 at completion (audio kind only): re-anchor the clock on
/// the new track's first delivered frame's own pts — the anchor
/// truth, never the freeze position — and move the clock's generation
/// to the switch's.
///
/// # Errors
/// The clock's typed error (an unrepresentable position — the clock
/// is unchanged; the caller surfaces the failure, never a fake
/// success).
pub(crate) fn complete_switch(
    clock: &media_clock::MediaClock,
    inflight: &InFlightTrackSwitch,
    anchor_ns: Option<i64>,
) -> Result<(), EngineError> {
    if let Some(anchor_ns) = anchor_ns {
        clock.reanchor(Timestamp::new(anchor_ns, NANO_BASE))?;
    }
    clock.set_generation(inflight.generation);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::CodecDescriptor;

    fn track(id: u32, kind: MediaKind, default_on: bool) -> Track {
        Track {
            id: TrackId::new(id),
            stream: media_types::StreamId::new(id),
            kind,
            language: None,
            codec: CodecDescriptor::new("fake").unwrap_or_else(|e| unreachable!("{e}")),
            title: None,
            flags: media_types::TrackFlags {
                default_on,
                ..media_types::TrackFlags::NONE
            },
            timing: media_types::TrackTiming {
                span: media_types::TimeSpan::UNKNOWN,
                base: media_types::TimeBase::new(1, 1_000_000)
                    .unwrap_or_else(|e| unreachable!("{e}")),
                gaps: Vec::new(),
            },
        }
    }

    fn list() -> Vec<Track> {
        vec![
            track(1, MediaKind::Video, false),
            track(2, MediaKind::Audio, true),
            track(3, MediaKind::Audio, false),
            track(4, MediaKind::Subtitle, false),
        ]
    }

    #[test]
    fn id_resolves_only_within_the_requested_kind() {
        let tracks = list();
        assert_eq!(
            resolve_selector(
                TrackSelector::Id {
                    id: TrackId::new(3)
                },
                &tracks,
                MediaKind::Audio
            ),
            Ok(ResolvedSelection::Track(TrackId::new(3)))
        );
        // A video id asked of the audio selection: typed not-found
        // (kind mismatch is a validity question, doc 05 V5).
        assert_eq!(
            resolve_selector(
                TrackSelector::Id {
                    id: TrackId::new(1)
                },
                &tracks,
                MediaKind::Audio
            ),
            Err(TrackSelectionError::NotFound {
                id: TrackId::new(1)
            })
        );
        // Unknown id likewise.
        assert_eq!(
            resolve_selector(
                TrackSelector::Id {
                    id: TrackId::new(99)
                },
                &tracks,
                MediaKind::Video
            ),
            Err(TrackSelectionError::NotFound {
                id: TrackId::new(99)
            })
        );
    }

    #[test]
    fn none_deselects_and_auto_takes_the_default_then_the_first() {
        let tracks = list();
        assert_eq!(
            resolve_selector(TrackSelector::None, &tracks, MediaKind::Audio),
            Ok(ResolvedSelection::Deselect)
        );
        // Auto: the default_on audio track (id 2).
        assert_eq!(
            resolve_selector(TrackSelector::Auto, &tracks, MediaKind::Audio),
            Ok(ResolvedSelection::Track(TrackId::new(2)))
        );
        // No default flag on video: first-of-kind in stream order.
        assert_eq!(
            resolve_selector(TrackSelector::Auto, &tracks, MediaKind::Video),
            Ok(ResolvedSelection::Track(TrackId::new(1)))
        );
        // No data track exists: Auto deselects honestly.
        assert_eq!(
            resolve_selector(TrackSelector::Auto, &tracks, MediaKind::Data),
            Ok(ResolvedSelection::Deselect)
        );
    }

    #[test]
    fn auto_prefers_the_flag_even_when_it_is_not_first() {
        let tracks = vec![
            track(7, MediaKind::Audio, false),
            track(8, MediaKind::Audio, true),
        ];
        assert_eq!(
            resolve_selector(TrackSelector::Auto, &tracks, MediaKind::Audio),
            Ok(ResolvedSelection::Track(TrackId::new(8)))
        );
    }

    #[test]
    fn selector_wire_round_trip() {
        let wire = commands::catalog::types::TrackSelector::Id {
            id: commands::catalog::types::TrackId(5),
        };
        assert_eq!(
            TrackSelector::from_wire(wire).as_wire(),
            wire,
            "engine and wire selector vocabularies round-trip"
        );
        assert_eq!(
            TrackSelector::from_wire(commands::catalog::types::TrackSelector::Auto),
            TrackSelector::Auto
        );
        assert_eq!(
            TrackSelector::from_wire(commands::catalog::types::TrackSelector::None),
            TrackSelector::None
        );
    }

    /// Audit F-1: the three wired kinds widen losslessly to their
    /// full-kind view (the track-list vocabulary stays `MediaKind`).
    #[test]
    fn wired_kinds_widen_to_their_media_kinds() {
        assert_eq!(WiredKind::Audio.as_media_kind(), MediaKind::Audio);
        assert_eq!(WiredKind::Video.as_media_kind(), MediaKind::Video);
        assert_eq!(WiredKind::Subtitle.as_media_kind(), MediaKind::Subtitle);
    }

    /// Audit F-1: the single `MediaKind` → `WiredKind` conversion
    /// answers the typed `UnwiredKind` refusal for the lane-less kind
    /// — the data-has-no-lane truth is now a typed error at the
    /// conversion seam, not a runtime abort in `build_limb`.
    #[test]
    fn data_is_the_typed_unwired_kind() {
        for kind in [MediaKind::Audio, MediaKind::Video, MediaKind::Subtitle] {
            let wired = match WiredKind::try_from(kind) {
                Ok(wired) => wired,
                Err(unwired) => unreachable!("{kind:?} is wired, got {unwired}"),
            };
            assert_eq!(wired.as_media_kind(), kind);
        }
        let err = match WiredKind::try_from(MediaKind::Data) {
            Ok(_) => unreachable!("data is lane-less by design A3"),
            Err(err) => err,
        };
        assert_eq!(err, UnwiredKind { kind: MediaKind::Data });
        assert_eq!(
            err.to_string(),
            "the data kind has no queue lane (design A3: lane-less kinds are never wired)"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(err);
        assert!(!boxed.to_string().is_empty());
    }
}
