//! Frame stepping — the one-frame seek policy (issue #32, doc 05
//! §2.4 `playback.step-frame`; VLC `frame-next` + the design-new
//! `frame-previous`).
//!
//! # What this module is
//!
//! The **pure planning half** of a frame step: given the current
//! position, the media's frame period and a direction, produce the
//! [`SeekTarget`] the session should apply. Execution rides the
//! existing seek flow of [`crate::seek`] (a frame step IS a seek of
//! one frame's magnitude — doc 05 §2.4 says the command is class A,
//! generated, and resolves against the same seven-step flow as any
//! other seek).
//!
//! # Semantics
//!
//! - **Auto-pause first**: VLC's frame step pauses playback before
//!   advancing (the catalog row says "auto-pause then step one
//!   frame"). [`FrameStepPlan`] reports whether the pause input is
//!   needed so a playing session steps to a *frozen* next frame.
//! - **Next** steps `+1` frame period from the current position;
//!   **Previous** steps `-1`. Both clamp to `[0, duration]` inside
//!   [`crate::seek::resolve_target`]-backed resolution (the
//!   [`SeekTarget::Relative`] target clamps on apply) — stepping
//!   past the last frame parks at the duration, stepping before the
//!   first parks at 0. Clamping is truth, not failure.
//! - **Frame period source**: the demuxer's video track frame rate
//!   is the honest source. The v0 [`media_types::Track`] carries no
//!   frame-rate field, so the period arrives as an `Option` the
//!   host fills from its probe/demux data; `None` falls back to
//!   [`FALLBACK_FRAME_PERIOD_NS`] (24 fps — the cinema default the
//!   UI demo transport also assumes). **[PROPOSAL: doc 05 §2.4
//!   leaves the fallback rate unspecified; 24 fps is the picked
//!   default, revisit when `Track` gains a frame-rate field.]**
//!
//! # What is deliberately v0 here
//!
//! The step lands on the container's keyframe-safe position ≤ the
//! target (the [`plugin_api::Demuxer::reanchor`] contract), so a
//! *decoded* frame boundary is not guaranteed — the pre-roll decode
//! of the seek flow presents the frame the decoder reaches. A
//! decode-exact frame step is a renderer/decoder-coupled feature and
//! stays out of the pure seam.

use crate::seek::SeekTarget;

/// Which way a frame step moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStepDirection {
    /// Advance one frame (VLC `e`, design `.`).
    Next,
    /// Rewind one frame (design `,` — VLC 3.0.23 has no
    /// frame-previous action; doc 05 §2.4 models it).
    Previous,
}

impl FrameStepDirection {
    /// Signed frame multiple: `+1` next, `-1` previous.
    fn sign(self) -> i64 {
        match self {
            FrameStepDirection::Next => 1,
            FrameStepDirection::Previous => -1,
        }
    }
}

/// Fallback frame period when the media's true rate is unknown:
/// 24 fps (cinema default; matches the UI demo transport's
/// `1000 / 24` ms step). **[PROPOSAL]** — see module docs.
pub const FALLBACK_FRAME_PERIOD_NS: i64 = 1_000_000_000 / 24;

/// One planned frame step: what the session must do to show the
/// adjacent frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStepPlan {
    /// `true` when playback is currently running and the pause
    /// input must be applied before the seek (auto-pause).
    pub needs_pause: bool,
    /// The seek to apply — always relative (one signed frame
    /// period), resolved and clamped by the existing seek flow.
    pub target: SeekTarget,
}

/// Plan a frame step (the pure seam of `playback.step-frame`).
///
/// `frame_period_ns` is the media's real frame period when the host
/// knows it (demuxer/probe-derived); `None` uses the documented
/// 24 fps fallback. A non-positive period is a caller bug and also
/// falls back — never a panic, never a divide.
#[must_use]
pub fn plan_frame_step(
    direction: FrameStepDirection,
    frame_period_ns: Option<i64>,
    is_playing: bool,
) -> FrameStepPlan {
    let period = match frame_period_ns {
        Some(ns) if ns > 0 => ns,
        _ => FALLBACK_FRAME_PERIOD_NS,
    };
    FrameStepPlan {
        needs_pause: is_playing,
        target: SeekTarget::Relative {
            delta_ns: direction.sign() * period,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_step_is_plus_one_period_relative() {
        let plan = plan_frame_step(FrameStepDirection::Next, Some(33_366_700), true);
        assert!(plan.needs_pause);
        assert_eq!(
            plan.target,
            SeekTarget::Relative {
                delta_ns: 33_366_700
            }
        );
    }

    #[test]
    fn previous_step_is_minus_one_period_relative() {
        let plan = plan_frame_step(FrameStepDirection::Previous, Some(33_366_700), false);
        assert!(!plan.needs_pause);
        assert_eq!(
            plan.target,
            SeekTarget::Relative {
                delta_ns: -33_366_700
            }
        );
    }

    #[test]
    fn unknown_period_falls_back_to_24fps() {
        let plan = plan_frame_step(FrameStepDirection::Next, None, false);
        assert_eq!(
            plan.target,
            SeekTarget::Relative {
                delta_ns: FALLBACK_FRAME_PERIOD_NS
            }
        );
        // 41_666_666 ns ≈ 41.67 ms — the 24 fps period.
        assert_eq!(FALLBACK_FRAME_PERIOD_NS, 41_666_666);
    }

    #[test]
    fn non_positive_period_is_replaced_not_trusted() {
        // A zero/negative period would freeze the seek delta at 0;
        // the fallback keeps the step meaningful.
        for bad in [0, -41_666_667] {
            let plan = plan_frame_step(FrameStepDirection::Next, Some(bad), false);
            assert_eq!(
                plan.target,
                SeekTarget::Relative {
                    delta_ns: FALLBACK_FRAME_PERIOD_NS
                }
            );
        }
    }

    #[test]
    fn paused_step_needs_no_pause_input() {
        let plan = plan_frame_step(FrameStepDirection::Next, Some(20_000_000), false);
        assert!(!plan.needs_pause);
    }
}
