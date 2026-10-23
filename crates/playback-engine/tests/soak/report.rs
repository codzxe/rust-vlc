//! The soak report (card 3.E4): the markdown artifact the run emits —
//! the full RSS/live-bytes series, the least-squares slope numbers,
//! the loop counts, and the leak-triage list. Every number in the
//! report is measured; every per-hour figure derived from a shorter
//! window is labeled an EXTRAPOLATION.

use std::fmt::Write as _;

use super::memory::{PhaseAccumulator, Sample};
use super::scenario::CycleCounts;

/// The report's measured facts (built by the soak driver from its
/// instruments — nothing here is invented).
pub struct SoakReport {
    /// The configured duration (minutes).
    pub minutes: f64,
    /// The configured sample cadence (ms).
    pub sample_ms: u64,
    /// Build profile at run time (`"debug"`/`"release"` — read from
    /// `PROFILE`).
    pub profile: String,
    /// The fixtures the rotation drove (labels).
    pub fixtures: Vec<String>,
    /// The full sample series (warmup marked).
    pub samples: Vec<Sample>,
    /// The measured-window live-bytes slope (B/s), if the window
    /// defined a line.
    pub live_slope_b_per_s: Option<f64>,
    /// The measured-window VmRSS slope (kB/s), if measurable.
    pub rss_slope_kb_per_s: Option<f64>,
    /// The generous CI ceiling this run asserted (MB/h).
    pub ceiling_mb_per_h: f64,
    /// The loop counters.
    pub counts: CycleCounts,
    /// The per-phase live-bytes attribution (the triage).
    pub phases: PhaseAccumulator,
    /// Engine lifetime stats at the end (frames, decoder builds).
    pub frames_delivered: u64,
    /// Decoder cache hits (reuses) at the end.
    pub decoders_reused: u64,
    /// Decoder builds at the end.
    pub decoders_built: u64,
    /// Engine polls driven.
    pub polls: u64,
    /// Engine events observed across the soak.
    pub events_seen: u64,
    /// Command receipts answered across the soak.
    pub receipts_seen: u64,
    /// Track-selection terminals observed across the soak.
    pub switch_terminals: u64,
    /// Decoded payload bytes seen at the sinks (cumulative).
    pub sink_bytes: u64,
    /// The reserved sample capacity (the sampler-is-not-a-leak note).
    pub reserved_samples: usize,
    /// Wall seconds the driver actually looped (measured window
    /// start → end).
    pub measured_window_s: f64,
    /// Cycles completed inside the measured window.
    pub measured_cycles: u64,
}

impl SoakReport {
    /// Render the full markdown report (the committed artifact's
    /// body — everything below the run-context header the driver
    /// writes above it).
    #[must_use = "the artifact"]
    pub fn markdown(&self) -> String {
        let mut out = String::with_capacity(16 * 1024);
        let total = self.samples.len();
        let measured: Vec<&Sample> = self.samples.iter().filter(|s| !s.warmup).collect();
        let warmup_count = total - measured.len();

        let _ = writeln!(
            out,
            "## Method\n\n\
             - Loop: `open → play → seek → track-switch → (play to end) → close`, repeated on one \
             `Engine` for the whole run (reopen = `close → open` on the same engine — the \
             accretion-visible invariant).\n\
             - Command path: every op is a catalog `WireRequest` routed through the soak's \
             `lumen_soak_dispatch` (the ARCH-5 single-entry shape; a faithful in-test mirror of \
             `apps/cli`'s `lumen_dispatch` playing subset — the app crate cannot be a dev-dependency \
             of the engine crate it builds on) onto the same `SessionHandle` seam the CLI host \
             drives.\n\
             - Scenario determinism: frozen virtual clock (doc 03 §8) + op script as a pure \
             function of the cycle index. Memory measurement: REAL wall clock — samples on a fixed \
             {} ms cadence from inside every engine poll.\n\
             - Instruments: `VmRSS` from `/proc/self/status` (kernel truth, arenas included) and \
             `live_bytes` from the process-global counting allocator \
             (`testkit::alloc_count`, the workspace's only allocator instrumentation — no \
             per-category allocator exists; \"by category\" is realized as per-loop-phase \
             attribution below).\n\
             - Warmup: the first {} of {} samples are marked warmup (pool sizing, allocator \
             steady state) and EXCLUDED from the slope fit; the series below keeps them.\n\
             - Fixtures (the committed rotation, multi-track included): {}.\n\
             - Profile: {} build.",
            self.sample_ms,
            warmup_count,
            total,
            self.fixtures.join(", "),
            self.profile,
        );

        // ---- the slope numbers ----
        let _ = writeln!(out, "\n## Slope (least squares, measured window)");
        let _ = writeln!(
            out,
            "\nMeasured window: {:.1} s of wall clock, {} samples, {} completed cycles.",
            self.measured_window_s,
            measured.len(),
            self.measured_cycles
        );
        match self.live_slope_b_per_s {
            Some(slope) => {
                let mb_per_h = slope * 3600.0 / 1e6;
                let verdict = if mb_per_h < 0.0 {
                    "negative (memory returning)"
                } else if mb_per_h < 5.0 {
                    "within the 5 MB/h target [PROPOSAL]"
                } else if mb_per_h < self.ceiling_mb_per_h {
                    "ABOVE the 5 MB/h target, below the CI ceiling — investigate"
                } else {
                    "ABOVE the CI ceiling — gross leak"
                };
                let _ = writeln!(
                    out,
                    "\n- **live_bytes slope: {slope:.1} B/s = {mb_per_h:.3} MB/h \
                     (EXTRAPOLATION: the per-hour figure extends the measured window's line)** — \
                     {verdict}."
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "\n- live_bytes slope: UNDETERMINED (fewer than two measured samples or \
                     zero time spread — the honest refusal)."
                );
            }
        }
        match self.rss_slope_kb_per_s {
            Some(slope) => {
                let mb_per_h = slope * 3600.0 / 1024.0 / 1024.0;
                let _ = writeln!(
                    out,
                    "- VmRSS slope: {slope:.3} kB/s = {mb_per_h:.3} MB/h (EXTRAPOLATION, same \
                     rule; includes allocator arena/fragmentation effects the live-bytes line \
                     does not)."
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "- VmRSS slope: UNAVAILABLE (no /proc or too few samples)."
                );
            }
        }
        let _ = writeln!(
            out,
            "- CI assertion: measured-window live_bytes slope < {:.0} MB/h (the generous \
             gross-leak ceiling; the 5 MB/h figure is the target [PROPOSAL], reported, not \
             asserted).",
            self.ceiling_mb_per_h
        );

        // ---- the loop counts ----
        let _ = writeln!(out, "\n## Loop counts");
        let mean_cycle_ms = if self.counts.cycles > 0 {
            self.counts.wall_s * 1000.0 / self.counts.cycles as f64
        } else {
            0.0
        };
        let _ = writeln!(
            out,
            "\n| fact | count |\n|---|---|\n\
             | completed cycles (open→…→close) | {} |\n\
             | cycles in the measured window | {} |\n\
             | opens accepted | {} |\n\
             | opens settling in Error (the dirty lane, expected) | {} |\n\
             | seeks completed (`Seeked`) | {} |\n\
             | seeks failed (`Seeked`'s D4 sibling `SeekFailed`) | {} |\n\
             | seeks skipped (state) | {} |\n\
             | track switches completed (hops, there-and-back = 2) | {} |\n\
             | switch hops skipped (state) | {} |\n\
             | cycles reaching `Ended` | {} |\n\
             | closes from an active state (composed stop→close) | {} |\n\
             | closes from the Error lane | {} |\n\
             | engine polls driven | {} |\n\
             | engine events observed / receipts answered | {} / {} |\n\
             | track-selection terminals observed | {} |\n\
             | frames delivered (engine) / accepted (sink) | {} / {} |\n\
             | decoded payload bytes seen at the sinks | {} |\n\
             | decoder builds / cache reuses | {} / {} |\n\
             | mean cycle wall time | {mean_cycle_ms:.1} ms |",
            self.counts.cycles,
            self.measured_cycles,
            self.counts.opens_ok,
            self.counts.error_opens,
            self.counts.seeks_ok,
            self.counts.seeks_failed,
            self.counts.seeks_skipped,
            self.counts.switches_ok,
            self.counts.switches_skipped,
            self.counts.ended_cycles,
            self.counts.active_closes,
            self.counts.error_closes,
            self.polls,
            self.events_seen,
            self.receipts_seen,
            self.switch_terminals,
            self.frames_delivered,
            self.counts.decoded_frames,
            self.sink_bytes,
            self.decoders_built,
            self.decoders_reused,
        );

        // ---- the triage list ----
        let _ = writeln!(
            out,
            "\n## Leak triage\n\n\
             Top loop-op suspects by mean net live-byte growth per cycle (the per-category \
             attribution the card asks for; negative values are net frees — expected at close). \
             Peak = the high-water mark of the running per-phase sum (transient footprint, freed \
             within the window)."
        );
        let _ = writeln!(
            out,
            "\n| rank | loop op | mean net Δ live bytes / cycle | running-sum peak (B) |"
        );
        let _ = writeln!(out, "|---|---|---|---|");
        for (rank, phase) in self.phases.ranked_by_growth().into_iter().enumerate() {
            let _ = writeln!(
                out,
                "| {} | {} | {:+.1} | {} |",
                rank + 1,
                phase.label(),
                self.phases.mean_per_cycle(phase),
                self.phases.peak(phase)
            );
        }
        let worst = self.phases.ranked_by_growth().first().copied();
        match worst {
            Some(phase)
                if self.phases.mean_per_cycle(phase) > 0.0
                    && self.live_slope_b_per_s.is_some_and(|slope| slope > 0.0) =>
            {
                let _ = writeln!(
                    out,
                    "\nHandoff note if the slope stands: the **{}** op is the top growth \
                     suspect — check the per-`{}` engine paths first.",
                    phase.label(),
                    phase.label()
                );
            }
            _ => {
                let _ = writeln!(
                    out,
                    "\nNo loop op shows persistent net growth above noise; the top movers are \
                     transient (allocated and freed within the cycle — see the peaks)."
                );
            }
        }
        let _ = writeln!(
            out,
            "\nEngine counters: {} decoder builds over {} cycles (≈{:.1} per cycle — every \
             reopen rebuilds its session's decoders, the per-session cache by design) with {} \
             cache reuses (the in-session switch lane); {} frames delivered.",
            self.decoders_built,
            self.counts.cycles,
            if self.counts.cycles > 0 {
                self.decoders_built as f64 / self.counts.cycles as f64
            } else {
                0.0
            },
            self.decoders_reused,
            self.frames_delivered
        );
        let _ = writeln!(
            out,
            "\nInstrument self-check: the sample series was preallocated at {} slots ({} used) \
             — the sampler is not a live-bytes growth term.",
            self.reserved_samples, total
        );

        // ---- the series ----
        let _ = writeln!(
            out,
            "\n## Series (every sample, {} ms cadence, wall clock)\n",
            self.sample_ms
        );
        let _ = writeln!(out, "| t (s) | window | VmRSS (kB) | live_bytes | cycles |",);
        let _ = writeln!(out, "|---|---|---|---|---|");
        for sample in &self.samples {
            let _ = writeln!(
                out,
                "| {:.1} | {} | {} | {} | {} |",
                sample.elapsed_s,
                if sample.warmup { "warmup" } else { "measured" },
                sample
                    .rss_kb
                    .map(|kb| kb.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                sample.live_bytes,
                sample.cycles
            );
        }
        let _ = writeln!(
            out,
            "\n(end of series — {} samples; generated by `cargo test -p playback-engine \
             --test soak` at SOAK_MINUTES={})",
            total, self.minutes
        );
        out
    }
}
