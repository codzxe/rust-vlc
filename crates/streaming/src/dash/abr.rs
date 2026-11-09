//! DASH **buffer-based representation selection** (card 6.C2): the
//! 6.B2b/6.D1 hysteresis pattern re-aimed at MPD representation
//! ladders — the watermark bands, EWMA throughput, dwell counters
//! and stall-forced down-steps of the HLS `AbrController`, plus the
//! buffer-level axis DASH v0 wires from the start.
//!
//! # Policy (v0, honestly labeled)
//!
//! * [`select_representation_index`] — the open-time pick: highest
//!   `@bandwidth` at or under the cap. Rungs with a missing/junk
//!   `@bandwidth` (the 6.C1a warn-and-`None` tolerance) are
//!   **skipped**, never guessed; a ladder with nothing selectable
//!   refuses typed (`streaming.dash.no-representation`).
//! * [`DashAbr::decide`] — boundary decisions only (between two
//!   segment fetches), on the same seam as 6.D1's `BufferModel`:
//!   the host measures fetches ([`DashAbr::observe_throughput`]),
//!   forwards buffer signals ([`DashAbr::on_buffer_signal`]) and
//!   buffer levels ([`DashAbr::note_buffer_level`]); the controller
//!   never reads a clock.
//! * **Hysteresis, no thrash** — one band per axis: a down-step
//!   needs the EWMA below `down_margin_percent` of the current
//!   rung, OR a 6.D1 stall prediction, OR the buffer under the low
//!   watermark; an up-step needs EWMA headroom over the next rung
//!   AND dwell served AND the buffer above the high watermark AND
//!   the rung under the cap. Between the margins it holds.
//! * `[PROPOSAL]` — the algorithm class (watermark/hysteresis vs a
//!   proper rate-based model like BBA/BOLA) stays a Lead decision;
//!   this module pins the hls-proven shape and its knobs.

use std::collections::VecDeque;

use playlists::AdaptationSet;

use crate::buffer::BufferSignal;

/// Default bandwidth cap for selection (mirrors the hls default).
pub const DEFAULT_MAX_BANDWIDTH_BPS: u64 = 25_000_000;

/// Default up-switch margin: EWMA must reach 125% of the next rung.
pub const DEFAULT_UP_MARGIN_PERCENT: u32 = 25;

/// Default down-switch margin: EWMA below 80% of the current rung.
pub const DEFAULT_DOWN_MARGIN_PERCENT: u32 = 80;

/// Default dwell before an up-switch may be considered.
pub const DEFAULT_MIN_DWELL_SEGMENTS: u32 = 2;

/// Default low buffer watermark (seconds): under it, down-steps are
/// forced (the drain side of the 6.D1 fill/drain band).
pub const DEFAULT_LOW_WATERMARK_S: f64 = 6.0;

/// Default high buffer watermark (seconds): above it (plus the
/// throughput band), up-steps are allowed (the fill side).
pub const DEFAULT_HIGH_WATERMARK_S: f64 = 20.0;

/// Bound on the remembered switch history (the anti-thrash ledger).
pub const SWITCH_HISTORY_CAP: usize = 16;

/// Typed refusals of DASH selection/scheduling (house style:
/// `streaming.dash.*` dotted codes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashAbrError {
    /// No representation is selectable (all over the cap, or all
    /// missing `@bandwidth`) (`streaming.dash.no-representation`).
    NoRepresentation,
    /// The requested representation index is outside the ladder
    /// (`streaming.dash.bad-representation`).
    BadRepresentation,
}

impl std::fmt::Display for DashAbrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DashAbrError::NoRepresentation => write!(
                f,
                "streaming.dash.no-representation: no representation selectable under the policy"
            ),
            DashAbrError::BadRepresentation => {
                write!(
                    f,
                    "streaming.dash.bad-representation: index outside the ladder"
                )
            }
        }
    }
}

impl std::error::Error for DashAbrError {}

/// Selection + hysteresis policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DashAbrConfig {
    /// Highest `@bandwidth` selection will consider.
    pub max_bandwidth_bps: u64,
    /// Up-switch margin in percent (see [`DEFAULT_UP_MARGIN_PERCENT`]).
    pub up_margin_percent: u32,
    /// Down-switch margin in percent (see
    /// [`DEFAULT_DOWN_MARGIN_PERCENT`]).
    pub down_margin_percent: u32,
    /// Segments that must be served on the current rung before an
    /// up-switch (down-steps on stall/drain bypass it).
    pub min_dwell_segments: u32,
    /// Low buffer watermark (seconds) — under it, down-steps force.
    pub low_watermark_s: f64,
    /// High buffer watermark (seconds) — above it, up-steps unlock.
    pub high_watermark_s: f64,
}

impl Default for DashAbrConfig {
    fn default() -> Self {
        DashAbrConfig {
            max_bandwidth_bps: DEFAULT_MAX_BANDWIDTH_BPS,
            up_margin_percent: DEFAULT_UP_MARGIN_PERCENT,
            down_margin_percent: DEFAULT_DOWN_MARGIN_PERCENT,
            min_dwell_segments: DEFAULT_MIN_DWELL_SEGMENTS,
            low_watermark_s: DEFAULT_LOW_WATERMARK_S,
            high_watermark_s: DEFAULT_HIGH_WATERMARK_S,
        }
    }
}

/// The open-time pick: highest `@bandwidth` at or under the cap;
/// `None`-bandwidth rungs are skipped (6.C1a's warn-tolerance
/// becomes an explicit skip at selection time — never a guess).
/// Ties keep document order. `Err(NoRepresentation)` = nothing
/// selectable.
pub fn select_representation_index(
    set: &AdaptationSet,
    max_bandwidth_bps: u64,
) -> Result<usize, DashAbrError> {
    set.representations
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.bandwidth.map(|bw| (i, bw)))
        .filter(|(_, bw)| *bw <= max_bandwidth_bps)
        .max_by_key(|(_, bw)| *bw)
        .map(|(i, _)| i)
        .ok_or(DashAbrError::NoRepresentation)
}

/// The buffer-based decision core: a pure policy object, no I/O and
/// no clock — the host feeds observations in, the session asks
/// [`decide`](DashAbr::decide) at each segment boundary.
#[derive(Debug, Clone)]
pub struct DashAbr {
    config: DashAbrConfig,
    /// Current rung index (into the caller's ladder slice).
    current: usize,
    /// EWMA of measured throughput (bytes per millisecond of wall
    /// time) — the 3/4-old + 1/4-new shift-only merge of 6.D1's rate
    /// EWMA (the hls controller's exact form).
    ewma_bytes_per_ms: Option<u128>,
    /// Segments served on the current rung (the dwell counter).
    segments_on_current: u32,
    /// Total boundaries seen (the history's time axis).
    segments_seen: u64,
    /// Last reported buffer level (seconds).
    buffer_s: Option<f64>,
    /// A 6.D1 `StallPredicted` is outstanding: force down-steps.
    stalled: bool,
    /// Completed switches.
    switch_count: u64,
    /// Bounded switch ledger: `(segments_seen, target_rung)`.
    history: VecDeque<(u64, usize)>,
}

impl DashAbr {
    /// A controller starting on rung `start` (the selection
    /// policy's answer at open).
    #[must_use]
    pub fn new(start: usize, config: DashAbrConfig) -> DashAbr {
        DashAbr {
            config,
            current: start,
            ewma_bytes_per_ms: None,
            segments_on_current: 0,
            segments_seen: 0,
            buffer_s: None,
            stalled: false,
            switch_count: 0,
            history: VecDeque::new(),
        }
    }

    /// The host observed a fetch: fold `bytes` over `wall_ns` into
    /// the EWMA. A zero span is skipped (no honest rate in it).
    pub fn observe_throughput(&mut self, bytes: u64, wall_ns: u64) {
        if wall_ns == 0 {
            return;
        }
        let rate = u128::from(bytes) * 1_000_000 / u128::from(wall_ns);
        self.ewma_bytes_per_ms = Some(match self.ewma_bytes_per_ms {
            Some(prev) => prev - (prev >> 2) + (rate >> 2),
            None => rate,
        });
    }

    /// The host forwarded a 6.D1 buffer signal: `StallPredicted`
    /// arms the forced down-step; `StallCleared`/`Filled` disarm it.
    pub fn on_buffer_signal(&mut self, signal: BufferSignal) {
        match signal {
            BufferSignal::StallPredicted { .. } => self.stalled = true,
            BufferSignal::StallCleared | BufferSignal::Filled => self.stalled = false,
            BufferSignal::Steady | BufferSignal::Drained => {}
        }
    }

    /// The host reported the current buffer level (seconds).
    pub fn note_buffer_level(&mut self, seconds: f64) {
        if seconds.is_finite() && seconds >= 0.0 {
            self.buffer_s = Some(seconds);
        }
    }

    /// The EWMA throughput in bits per second, once sampled.
    #[must_use]
    pub fn throughput_bps(&self) -> Option<u64> {
        self.ewma_bytes_per_ms
            .map(|r| (r * 8_000).min(u128::from(u64::MAX)) as u64)
    }

    /// The current rung index.
    #[must_use]
    pub fn current(&self) -> usize {
        self.current
    }

    /// Counters for tests, logging, the event-hub surface.
    #[must_use]
    pub fn snapshot(&self) -> DashAbrSnapshot {
        DashAbrSnapshot {
            current: self.current,
            switch_count: self.switch_count,
            throughput_bps: self.throughput_bps(),
            stalled: self.stalled,
            buffer_s: self.buffer_s,
            segments_on_current: self.segments_on_current,
        }
    }

    /// Account one segment served on the current rung.
    pub fn segment_served(&mut self) {
        self.segments_on_current += 1;
        self.segments_seen += 1;
    }

    /// Decide the rung for the NEXT segment boundary.
    /// `ladder_bandwidths` is the caller's rung slice (one entry per
    /// representation, in the same order `select_representation_
    /// index`/the session uses). `Some(target)` = switch now (the
    /// controller has committed); `None` = hold.
    #[must_use]
    pub fn decide(&mut self, ladder_bandwidths: &[u64]) -> Option<usize> {
        if ladder_bandwidths.is_empty() {
            return None;
        }
        let throughput = self.throughput_bps()?;
        let mut ladder: Vec<usize> = (0..ladder_bandwidths.len()).collect();
        ladder.sort_by_key(|&i| ladder_bandwidths[i]);
        let pos = ladder.iter().position(|&i| i == self.current)?;

        // ---- Down: stall prediction, starvation, or a drained
        // buffer (emergencies bypass the dwell).
        let current_bw = ladder_bandwidths[self.current];
        let starved = u128::from(throughput) * 100
            < u128::from(current_bw) * u128::from(self.config.down_margin_percent);
        let drained = self
            .buffer_s
            .is_some_and(|b| b < self.config.low_watermark_s);
        if pos > 0 && (self.stalled || starved || drained) {
            return self.commit(ladder[pos - 1]);
        }

        // ---- Up: headroom over the next rung, dwell served, the
        // buffer above the high watermark, the rung under the cap.
        if pos + 1 < ladder.len() && !self.stalled {
            let next = ladder[pos + 1];
            let next_bw = ladder_bandwidths[next];
            let headroom = u128::from(throughput) * 100
                >= u128::from(next_bw) * u128::from(100 + self.config.up_margin_percent);
            let filled = self
                .buffer_s
                .is_none_or(|b| b >= self.config.high_watermark_s);
            if next_bw <= self.config.max_bandwidth_bps
                && headroom
                && filled
                && self.segments_on_current >= self.config.min_dwell_segments
            {
                return self.commit(next);
            }
        }
        None
    }

    /// Commit a switch (bounded history, reset dwell, count it).
    fn commit(&mut self, target: usize) -> Option<usize> {
        self.current = target;
        self.segments_on_current = 0;
        self.switch_count += 1;
        self.history.push_back((self.segments_seen, target));
        while self.history.len() > SWITCH_HISTORY_CAP {
            self.history.pop_front();
        }
        Some(target)
    }
}

/// The controller's queryable counters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DashAbrSnapshot {
    /// Current rung index.
    pub current: usize,
    /// Completed switches.
    pub switch_count: u64,
    /// EWMA throughput (bits per second), once sampled.
    pub throughput_bps: Option<u64>,
    /// A stall prediction is forcing down-steps.
    pub stalled: bool,
    /// Last reported buffer level (seconds), if any.
    pub buffer_s: Option<f64>,
    /// Segments served on the current rung since the last switch.
    pub segments_on_current: u32,
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;
    use playlists::parse_mpd_bytes;

    fn set_from(mpd_text: &str) -> AdaptationSet {
        let parsed = match parse_mpd_bytes(mpd_text.as_bytes()) {
            Ok(p) => p,
            Err(e) => unreachable!("parse: {e}"),
        };
        parsed.mpd.periods[0].adaptation_sets[0].clone()
    }

    fn ladder_mpd() -> String {
        let mut s = String::from(
            "<?xml version=\"1.0\"?><MPD type=\"static\" mediaPresentationDuration=\"PT24S\">\
             <Period><AdaptationSet contentType=\"video\">",
        );
        for (id, bw) in [("hi", 800_000u64), ("mid", 400_000), ("lo", 150_000)] {
            s.push_str(&format!("<Representation id=\"{id}\" bandwidth=\"{bw}\"/>"));
        }
        s.push_str("</AdaptationSet></Period></MPD>");
        s
    }

    #[test]
    fn selection_is_highest_under_cap_skipping_none_rungs() {
        let mut text = String::from(
            "<?xml version=\"1.0\"?><MPD type=\"static\"><Period><AdaptationSet contentType=\"video\">",
        );
        // A None-bandwidth rung (attr absent → warn + skip at
        // selection), then mid, then hi.
        text.push_str("<Representation id=\"broken\"/>");
        text.push_str("<Representation id=\"mid\" bandwidth=\"400000\"/>");
        text.push_str("<Representation id=\"hi\" bandwidth=\"800000\"/>");
        text.push_str("</AdaptationSet></Period></MPD>");
        let set = set_from(&text);
        assert_eq!(select_representation_index(&set, 25_000_000), Ok(2));
        assert_eq!(select_representation_index(&set, 500_000), Ok(1));
        // Only the broken rung is under the cap: nothing selectable.
        assert_eq!(
            select_representation_index(&set, 1),
            Err(DashAbrError::NoRepresentation)
        );
    }

    #[test]
    fn hysteresis_steps_down_on_starvation_and_back_up() {
        let set = set_from(&ladder_mpd());
        let start = match select_representation_index(&set, 25_000_000) {
            Ok(i) => i,
            Err(e) => unreachable!("select: {e}"),
        };
        assert_eq!(start, 0, "hi rung");
        let mut abr = DashAbr::new(start, DashAbrConfig::default());
        let ladder: Vec<u64> = set
            .representations
            .iter()
            .map(|r| r.bandwidth.unwrap_or(0))
            .collect();

        // Healthy hi throughput (1.6 Mbps), buffer fat.
        abr.observe_throughput(1000, 5_000_000);
        abr.note_buffer_level(30.0);
        abr.segment_served();
        abr.segment_served();
        assert_eq!(abr.decide(&ladder), None, "hi holds — top rung");

        // Crash: eight 0.08 Mbps samples sink the EWMA under 80%
        // of 800 kbps (80 bytes/ms).
        for _ in 0..8 {
            abr.observe_throughput(100, 10_000_000);
        }
        let target = match abr.decide(&ladder) {
            Some(t) => t,
            None => unreachable!("must step down"),
        };
        assert_eq!(set.representations[target].bandwidth, Some(400_000), "mid");

        // Starve to lo.
        for _ in 0..8 {
            abr.observe_throughput(30, 10_000_000);
            abr.segment_served();
        }
        let target = match abr.decide(&ladder) {
            Some(t) => t,
            None => unreachable!("must step down again"),
        };
        assert_eq!(set.representations[target].bandwidth, Some(150_000), "lo");

        // Recover: fat buffer + 2 Mbps for enough boundaries.
        abr.note_buffer_level(30.0);
        for _ in 0..10 {
            abr.observe_throughput(250, 1_000_000);
            abr.segment_served();
            let _ = abr.decide(&ladder);
        }
        assert!(
            abr.snapshot().current <= 1,
            "recovered off lo (doc order hi,mid,lo), current={}",
            abr.snapshot().current
        );
        assert!(abr.snapshot().switch_count >= 2);
    }

    #[test]
    fn low_buffer_forces_down_high_buffer_gates_up() {
        let set = set_from(&ladder_mpd());
        let ladder: Vec<u64> = set
            .representations
            .iter()
            .map(|r| r.bandwidth.unwrap_or(0))
            .collect();
        // Start mid (index 1).
        let mut abr = DashAbr::new(1, DashAbrConfig::default());
        // Plenty of throughput for hi, but the buffer is drained:
        // the low watermark forces DOWN, not up.
        abr.observe_throughput(500, 1_000_000);
        abr.note_buffer_level(1.0);
        abr.segment_served();
        abr.segment_served();
        // The ladder is bandwidth-sorted (lo, mid, hi) = document
        // indices (2, 1, 0); from mid the forced down-step is the
        // 150k rung = document index 2.
        assert_eq!(abr.decide(&ladder), Some(2), "drained buffer steps DOWN");
        assert_eq!(set.representations[abr.current()].bandwidth, Some(150_000));

        // Fat throughput + healthy buffer + dwell → back up.
        let mut abr = DashAbr::new(1, DashAbrConfig::default());
        abr.observe_throughput(500, 1_000_000);
        abr.note_buffer_level(30.0);
        abr.segment_served();
        abr.segment_served();
        assert_eq!(abr.decide(&ladder), Some(0), "filled buffer allows UP");
        assert_eq!(set.representations[abr.current()].bandwidth, Some(800_000));
    }

    #[test]
    fn stall_prediction_bypasses_everything() {
        let set = set_from(&ladder_mpd());
        let ladder: Vec<u64> = set
            .representations
            .iter()
            .map(|r| r.bandwidth.unwrap_or(0))
            .collect();
        let mut abr = DashAbr::new(0, DashAbrConfig::default());
        abr.observe_throughput(1000, 1_000_000); // 8 Mbps, healthy
        abr.note_buffer_level(30.0);
        abr.on_buffer_signal(BufferSignal::StallPredicted {
            eta_ns: 500_000_000,
        });
        assert_eq!(
            abr.decide(&ladder),
            Some(1),
            "stall forces down despite health"
        );
        abr.on_buffer_signal(BufferSignal::StallCleared);
        assert_eq!(abr.decide(&ladder), None, "cleared, holds");
    }
}
