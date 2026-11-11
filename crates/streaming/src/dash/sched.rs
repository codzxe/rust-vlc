//! DASH **segment scheduling + live availability-time sync** (card
//! 6.C2): the segment walk over 6.C1b [`RepresentationPlan`]s under
//! 6.C2 [`DashAbr`] control, and the wall-clock arithmetic that
//! makes *dynamic* MPDs schedulable — ISO 8601
//! `availabilityStartTime` parsing (the fact 6.C1a deliberately
//! recorded verbatim), segment availability times (AST + Period
//! start + presentation time offset), and live-edge computation.
//!
//! # The v0 shape (honest)
//!
//! * The scheduler is a **pure state machine**: no I/O and no clock
//!   of its own — the host calls [`DashScheduler::next_fetch`] with
//!   the wall time it observed and performs the fetch itself
//!   (the same posture as 6.D1's `BufferModel` and the hls
//!   sessions). The scripted-bandwidth tests below drive it
//!   directly; a mockhttp socket e2e lane is future work (recorded
//!   in the feature matrix, not claimed here).
//! * Rungs of one AdaptationSet are **aligned** (the
//!   `@segmentAlignment` assumption): segment index `i` means the
//!   same media time on every rung; a switch happens only between
//!   fetches.
//! * **Live join** (dynamic MPDs): the first fetch targets the
//!   newest segment whose availability time is at least
//!   [`SAFE_BEHIND_LIVE_EDGE_S`] in the past (the safe distance
//!   behind the live edge); earlier segments are skipped, not
//!   fetched-and-stalled. `timeShiftBufferDepth` trimming is the
//!   reload-loop card's concern (recorded, not guessed).
//! * Static MPDs schedule from segment 0, no wall clock needed.

use playlists::{AdaptationSet, Mpd};

use super::abr::{DashAbr, DashAbrConfig, DashAbrError, select_representation_index};
use super::resolve::{DashResolveError, RepresentationPlan, plan_representation};

/// Default safe distance behind the live edge for dynamic joins
/// (seconds) — small enough to be live, large enough that the
/// segment is complete on every mirror.
pub const SAFE_BEHIND_LIVE_EDGE_S: f64 = 3.0;

/// Typed scheduling refusals (house style: `streaming.dash.*`).
#[derive(Debug, Clone, PartialEq)]
pub enum DashSchedError {
    /// Nothing selectable in the ladder (see
    /// [`DashAbrError::NoRepresentation`]).
    NoRepresentation,
    /// A rung's plan refused ([`DashResolveError`] verbatim).
    Resolve(DashResolveError),
    /// The ladder has no rungs at all (`streaming.dash.bad-ladder`).
    BadLadder,
    /// A dynamic manifest's `availabilityStartTime` is absent or
    /// unparseable — live scheduling is impossible
    /// (`streaming.dash.bad-availability-time`).
    BadAvailabilityTime,
}

impl std::fmt::Display for DashSchedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DashSchedError::NoRepresentation => write!(
                f,
                "streaming.dash.no-representation: no representation selectable under the policy"
            ),
            DashSchedError::Resolve(inner) => write!(f, "streaming.dash.resolve: {inner}"),
            DashSchedError::BadLadder => {
                write!(f, "streaming.dash.bad-ladder: empty representation ladder")
            }
            DashSchedError::BadAvailabilityTime => write!(
                f,
                "streaming.dash.bad-availability-time: dynamic manifest without a usable availabilityStartTime"
            ),
        }
    }
}

impl std::error::Error for DashSchedError {}

impl From<DashAbrError> for DashSchedError {
    fn from(value: DashAbrError) -> Self {
        match value {
            DashAbrError::NoRepresentation => DashSchedError::NoRepresentation,
            DashAbrError::BadRepresentation => DashSchedError::BadLadder,
        }
    }
}

// ------------------------------------------------- ISO 8601 / availability

/// Parse an ISO 8601 / RFC 3339 date-time into epoch seconds:
/// `YYYY-MM-DDThh:mm:ss[.frac][Z|±hh:MM]` (the `availabilityStartTime`
/// profile; a space separator is tolerated). **Whole-second
/// resolution**: a fractional part is validated then dropped
/// (sub-second live edges are not a v0 concern). `None` on any
/// other shape — never a guess. Leap second `:60` clamps to `:59`
/// (the POSIX-day convention).
#[must_use]
pub fn parse_iso8601_epoch_seconds(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let num = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = text.get(range)?;
        if part.bytes().all(|b| b.is_ascii_digit()) {
            part.parse().ok()
        } else {
            None
        }
    };
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    if bytes[10] != b'T' && bytes[10] != b' ' {
        return None;
    }
    if bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let year = num(0..4)?;
    let month = num(5..7)?;
    let day = num(8..10)?;
    let hour = num(11..13)?;
    let minute = num(14..16)?;
    let second = num(17..19)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let second = second.min(59); // leap second clamp
    let offset_s = split_fraction_and_offset(&text[19..])?;
    // Days from civil (Howard Hinnant's algorithm) — no deps.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    // The offset is the zone's displacement from UTC: local − offset = UTC.
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second - offset_s)
}

/// Split the `".frac"` and `Z|±hh:mm` tail into (validated,
/// offset seconds). The fraction is validated and dropped
/// (whole-second resolution, see the parser docs).
fn split_fraction_and_offset(rest: &str) -> Option<i64> {
    let mut rest = rest;
    if let Some(tail) = rest.strip_prefix('.') {
        let digits_end = tail
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(tail.len());
        if digits_end == 0 {
            return None;
        }
        rest = &tail[digits_end..];
    }
    if rest.is_empty() || rest == "Z" || rest == "z" {
        // No offset: DASH ASTs are UTC in practice — treat as Z
        // (recorded decision; the alternative is refusing every
        // common manifest that omits it).
        return Some(0);
    }
    let (sign, tail) = match rest.as_bytes().first() {
        Some(b'+') => (1i64, &rest[1..]),
        Some(b'-') => (-1i64, &rest[1..]),
        _ => return None,
    };
    let bytes = tail.as_bytes();
    if bytes.len() == 5 && bytes[2] == b':' {
        let hh: i64 = tail[0..2].parse().ok()?;
        let mm: i64 = tail[3..5].parse().ok()?;
        if hh > 23 || mm > 59 {
            return None;
        }
        Some(sign * (hh * 3_600 + mm * 60))
    } else {
        None
    }
}

/// The availability time (epoch seconds) of the segment whose
/// timeline starts at `segment_time_ticks` with duration
/// `duration_ticks`: a segment is available when its **last** chunk
/// exists — `AST + PeriodStart + (t + d − PTO) / timescale`
/// (ISO/IEC 23009-1 §5.3.9.5's arithmetic on the wall-clock axis).
#[must_use]
pub fn segment_availability_epoch(
    ast_epoch: i64,
    period_start_s: f64,
    segment_time_ticks: u64,
    duration_ticks: u64,
    pto_ticks: u64,
    timescale: u64,
) -> f64 {
    let presented = segment_time_ticks.saturating_add(duration_ticks);
    let rel_s = ticks_to_seconds(presented.saturating_sub(pto_ticks), timescale);
    // `as f64` is lossless for wall-clock seconds (< 2^53).
    ast_epoch as f64 + period_start_s + rel_s
}

/// The live edge in **presentation seconds** (the media time "now"
/// maps to): `now − (AST + PeriodStart) + PTO/timescale` — the
/// presentation-time offset moves the wall-clock origin.
#[must_use]
pub fn live_edge_presentation_seconds(
    now_epoch: i64,
    ast_epoch: i64,
    period_start_s: f64,
    pto_ticks: u64,
    timescale: u64,
) -> f64 {
    (now_epoch - ast_epoch) as f64 - period_start_s + ticks_to_seconds(pto_ticks, timescale)
}

/// Ticks → seconds (timescale 0 treated as 1 — the parser's
/// default; a 0 here can only come from a defaulted field).
fn ticks_to_seconds(ticks: u64, timescale: u64) -> f64 {
    let ts = if timescale == 0 { 1 } else { timescale };
    ticks as f64 / ts as f64
}

// ------------------------------------------------------------- scheduler

/// One scheduled fetch: the rung, its resolved URL, the segment's
/// number and duration.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchStep {
    /// Rung index (the caller's ladder order).
    pub representation: usize,
    /// The resolved absolute media URL.
    pub url: String,
    /// The segment's `@number`.
    pub number: u64,
    /// The segment's duration (seconds).
    pub duration_s: f64,
}

/// The answer to "what next": fetch, wait for live availability, or
/// the plan is drained.
#[derive(Debug, Clone, PartialEq)]
pub enum NextStep {
    /// Fetch this step now.
    Fetch(FetchStep),
    /// Dynamic: the next segment is not yet available — call again
    /// no sooner than this many seconds from now.
    Wait {
        /// Seconds until the next segment becomes available.
        available_in_s: f64,
    },
    /// Every segment of the plan has been served.
    Done,
}

/// The segment scheduler over one AdaptationSet's resolved ladder.
#[derive(Debug, Clone)]
pub struct DashScheduler {
    /// Resolved plans, one per representation (ladder order).
    plans: Vec<RepresentationPlan>,
    /// The ABR policy (boundary decisions).
    abr: DashAbr,
    /// Next segment index (rungs are aligned — `@segmentAlignment`).
    next: usize,
    /// `MPD@type == dynamic`.
    dynamic: bool,
    /// availabilityStartTime as epoch seconds (dynamic manifests).
    ast_epoch: Option<i64>,
    /// Period@start (seconds; the MPD-relative presentation origin).
    period_start_s: f64,
    /// Safe distance behind the live edge for the live join.
    safe_behind_live_edge_s: f64,
    /// Whether the live-join skip has happened yet.
    joined: bool,
}

impl DashScheduler {
    /// Build the scheduler: resolve every rung of `set` in period
    /// `period` against `manifest_url`, select the opening rung by
    /// the bandwidth cap, and start the ABR controller.
    ///
    /// # Errors
    /// [`DashSchedError::Resolve`] (a rung's plan refused — the
    /// honest line: a ladder with one broken addressing shape is
    /// refused, not silently truncated), [`DashSchedError::
    /// NoRepresentation`] (nothing selectable).
    pub fn new(
        mpd: &Mpd,
        period: &playlists::Period,
        set: &AdaptationSet,
        manifest_url: &str,
        config: DashAbrConfig,
    ) -> Result<DashScheduler, DashSchedError> {
        if set.representations.is_empty() {
            return Err(DashSchedError::BadLadder);
        }
        let mut plans = Vec::with_capacity(set.representations.len());
        for rep in &set.representations {
            plans.push(
                plan_representation(mpd, period, set, rep, manifest_url)
                    .map_err(DashSchedError::Resolve)?,
            );
        }
        let start = select_representation_index(set, config.max_bandwidth_bps)?;
        let ast_epoch = mpd
            .availability_start_time
            .as_deref()
            .and_then(parse_iso8601_epoch_seconds);
        Ok(DashScheduler {
            plans,
            abr: DashAbr::new(start, config),
            next: 0,
            dynamic: matches!(mpd.mpd_type, playlists::MpdType::Dynamic),
            ast_epoch,
            period_start_s: period.start.unwrap_or(0.0),
            safe_behind_live_edge_s: SAFE_BEHIND_LIVE_EDGE_S,
            joined: false,
        })
    }

    /// The ABR controller's snapshot (selection metrics).
    #[must_use]
    pub fn abr_snapshot(&self) -> super::abr::DashAbrSnapshot {
        self.abr.snapshot()
    }

    /// The host observed a fetch (or any bandwidth sample).
    pub fn observe_throughput(&mut self, bytes: u64, wall_ns: u64) {
        self.abr.observe_throughput(bytes, wall_ns);
    }

    /// The host reported the buffer level (seconds).
    pub fn note_buffer_level(&mut self, seconds: f64) {
        self.abr.note_buffer_level(seconds);
    }

    /// The host forwarded a 6.D1 buffer signal.
    pub fn on_buffer_signal(&mut self, signal: crate::buffer::BufferSignal) {
        self.abr.on_buffer_signal(signal);
    }

    /// The next fetch under ABR + availability. `now_epoch` is the
    /// host's observed wall clock (only consulted for dynamic
    /// manifests; `None` means "treat everything as available" —
    /// the static and test posture).
    ///
    /// # Errors
    /// [`DashSchedError`] — ladder troubles only; scheduling itself
    /// never fails (an exhausted plan is [`NextStep::Done`], not an
    /// error).
    pub fn next_fetch(&mut self, now_epoch: Option<i64>) -> Result<NextStep, DashSchedError> {
        // Boundary decision (between fetches — never mid-segment);
        // `decide` commits internally, `current` moves at once.
        if self.next > 0 {
            let ladder: Vec<u64> = self
                .plans
                .iter()
                .map(|p| p.bandwidth.unwrap_or(0))
                .collect();
            let _ = self.abr.decide(&ladder);
        }
        let rung = match self.plans.get(self.abr.current()) {
            Some(r) => r,
            None => return Err(DashSchedError::BadLadder),
        };
        let total = rung.segments.len();
        if self.next >= total {
            // The current rung is exhausted; aligned ladders mean
            // they all are. (Defensive: a shorter rung ends the
            // walk rather than panicking — recorded v0 shape.)
            return Ok(NextStep::Done);
        }
        // Live join: on the first fetch of a dynamic manifest, skip
        // to the newest segment already safely available — advance
        // while the NEXT segment is also ready. When nothing is
        // ready yet the walk stays un-joined, so the availability
        // gate below answers Wait and a later call re-joins at the
        // newer edge (a dynamic plan is never "drained" by
        // skipping — availability is a clock fact, not a plan fact).
        if self.dynamic && !self.joined {
            let now = match now_epoch {
                Some(now) => now,
                None => {
                    return Ok(NextStep::Wait {
                        available_in_s: 0.0,
                    });
                }
            };
            let ast = match self.ast_epoch {
                Some(ast) => ast,
                None => return Err(DashSchedError::BadAvailabilityTime),
            };
            while self.next + 1 < total {
                let rung = &self.plans[self.abr.current()];
                let upcoming = &rung.segments[self.next + 1];
                let avail = segment_availability_epoch(
                    ast,
                    self.period_start_s,
                    upcoming.time,
                    upcoming.duration,
                    rung.presentation_time_offset,
                    rung.timescale,
                );
                if now as f64 >= avail + self.safe_behind_live_edge_s {
                    self.next += 1;
                } else {
                    break;
                }
            }
            // The join completes only once the positioned segment is
            // itself ready; until then the gate below answers Wait
            // and a later call re-runs the skip at the newer edge.
            let rung = &self.plans[self.abr.current()];
            let seg = &rung.segments[self.next];
            let avail = segment_availability_epoch(
                ast,
                self.period_start_s,
                seg.time,
                seg.duration,
                rung.presentation_time_offset,
                rung.timescale,
            );
            if now as f64 >= avail + self.safe_behind_live_edge_s {
                self.joined = true;
            }
        }
        // Availability gating (dynamic): the next segment must be
        // complete before it is fetched.
        if self.dynamic
            && let (Some(now), Some(ast)) = (now_epoch, self.ast_epoch)
        {
            let rung = &self.plans[self.abr.current()];
            let seg = &rung.segments[self.next];
            let avail = segment_availability_epoch(
                ast,
                self.period_start_s,
                seg.time,
                seg.duration,
                rung.presentation_time_offset,
                rung.timescale,
            );
            let ready_at = avail + self.safe_behind_live_edge_s;
            if (now as f64) < ready_at {
                return Ok(NextStep::Wait {
                    available_in_s: ready_at - now as f64,
                });
            }
        }
        let rung = &self.plans[self.abr.current()];
        let seg = &rung.segments[self.next];
        Ok(NextStep::Fetch(FetchStep {
            representation: self.abr.current(),
            url: seg.url.clone(),
            number: seg.number,
            duration_s: ticks_to_seconds(seg.duration, rung.timescale),
        }))
    }

    /// Account one served fetch (the dwell counter's tick; call
    /// after the host completed the fetch of the step it got).
    pub fn segment_served(&mut self) {
        self.abr.segment_served();
        self.next += 1;
    }

    /// The number of segments served so far.
    #[must_use]
    pub fn segments_served(&self) -> usize {
        self.next
    }
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;
    use playlists::parse_mpd_bytes;

    fn parse(text: &str) -> Mpd {
        match parse_mpd_bytes(text.as_bytes()) {
            Ok(p) => p.mpd,
            Err(e) => unreachable!("parse: {e}"),
        }
    }

    // -- ISO 8601 vectors (RFC 3339 profile). --

    #[test]
    fn iso8601_epoch_vectors() {
        assert_eq!(parse_iso8601_epoch_seconds("1970-01-01T00:00:00Z"), Some(0));
        // 2026-01-01 is 56 years + 14 leap days after the epoch:
        // 56*365 + 14 = 20454 days.
        assert_eq!(
            parse_iso8601_epoch_seconds("2026-01-01T00:00:00Z"),
            Some(20_454 * 86_400)
        );
        assert_eq!(
            parse_iso8601_epoch_seconds("1970-01-01T01:00:00+01:00"),
            Some(0)
        );
        assert_eq!(
            parse_iso8601_epoch_seconds("1970-01-02T00:00:00-01:00"),
            Some(90_000)
        );
        assert_eq!(
            parse_iso8601_epoch_seconds("1970-01-01T00:00:00.500Z"),
            Some(0)
        ); // whole-second resolution
        assert_eq!(parse_iso8601_epoch_seconds("1970-01-01 00:00:00Z"), Some(0));
        assert_eq!(parse_iso8601_epoch_seconds("not a date"), None);
        assert_eq!(parse_iso8601_epoch_seconds("1970-13-01T00:00:00Z"), None);
        assert_eq!(parse_iso8601_epoch_seconds("1970-01-01T25:00:00Z"), None);
        assert_eq!(
            parse_iso8601_epoch_seconds("1970-01-01T00:00:60Z"),
            Some(59)
        );
        assert_eq!(parse_iso8601_epoch_seconds(""), None);
    }

    // -- Availability + live edge math. --

    #[test]
    fn availability_is_segment_end_on_the_wall_clock() {
        // AST = epoch; period at 0; t=0 d=10 @ timescale 1 → the
        // segment is available when its 10s are complete.
        assert_eq!(segment_availability_epoch(0, 0.0, 0, 10, 0, 1), 10.0);
        // PTO shifts the wall origin backward.
        assert_eq!(segment_availability_epoch(0, 0.0, 0, 10, 10, 1), 0.0);
        // Timescale 90000, 1s segments.
        assert_eq!(
            segment_availability_epoch(100, 5.0, 90_000, 90_000, 0, 90_000),
            107.0
        );
    }

    #[test]
    fn live_edge_computation() {
        assert_eq!(live_edge_presentation_seconds(100, 0, 0.0, 0, 1), 100.0);
        // PTO moves the presentation origin: 30s of offset.
        assert_eq!(live_edge_presentation_seconds(100, 0, 0.0, 30, 1), 130.0);
        // Period starting at 10s.
        assert_eq!(live_edge_presentation_seconds(100, 0, 10.0, 0, 1), 90.0);
        // AST 1000, now 1005.
        assert_eq!(live_edge_presentation_seconds(1_005, 1_000, 0.0, 0, 1), 5.0);
    }

    // -- The scheduler over synthetic manifests. --

    fn static_ladder_mpd() -> Mpd {
        parse(
            "<?xml version=\"1.0\"?><MPD type=\"static\" mediaPresentationDuration=\"PT16S\">\
             <Period start=\"PT0S\"><AdaptationSet contentType=\"video\" segmentAlignment=\"true\">\
             <SegmentTemplate timescale=\"1\" duration=\"2\" startNumber=\"1\" \
             media=\"$RepresentationID$-$Number$.m4s\" initialization=\"$RepresentationID$-init.m4s\"/>\
             <Representation id=\"hi\" bandwidth=\"800000\"/>\
             <Representation id=\"mid\" bandwidth=\"400000\"/>\
             <Representation id=\"lo\" bandwidth=\"150000\"/>\
             </AdaptationSet></Period></MPD>",
        )
    }

    fn scheduler(mpd: &Mpd) -> DashScheduler {
        match DashScheduler::new(
            mpd,
            &mpd.periods[0],
            &mpd.periods[0].adaptation_sets[0],
            "http://unit.test/vod/manifest.mpd", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            DashAbrConfig::default(),
        ) {
            Ok(s) => s,
            Err(e) => unreachable!("scheduler: {e}"),
        }
    }

    #[test]
    fn static_walk_serves_every_segment_in_order() {
        let mpd = static_ladder_mpd();
        let mut sched = scheduler(&mpd);
        let mut urls = Vec::new();
        loop {
            let step = match sched.next_fetch(None) {
                Ok(NextStep::Fetch(step)) => step,
                Ok(NextStep::Done) => break,
                other => unreachable!("static never waits: {other:?}"),
            };
            assert_eq!(
                step.url,
                format!("http://unit.test/vod/hi-{}.m4s", urls.len() + 1) // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            );
            urls.push(step.url);
            sched.observe_throughput(8_000_000, 8_000_000); // healthy
            sched.note_buffer_level(30.0);
            sched.segment_served();
        }
        assert_eq!(urls.len(), 8, "16s / 2s segments");
        assert_eq!(sched.segments_served(), 8);
    }

    #[test]
    fn scripted_crash_steps_the_ladder_down() {
        let mpd = static_ladder_mpd();
        let mut sched = scheduler(&mpd);
        // Two healthy hi segments (1.6 Mbps), then the crash.
        for _ in 0..2 {
            let step = match sched.next_fetch(None) {
                Ok(NextStep::Fetch(s)) => s,
                other => unreachable!("{other:?}"),
            };
            assert_eq!(step.representation, 0, "hi rung (document index 0)");
            sched.observe_throughput(1000, 5_000_000);
            sched.note_buffer_level(30.0);
            sched.segment_served();
        }
        // Crash the link: eight 0.08 Mbps samples per boundary sink
        // the EWMA below 80% of 800 kbps.
        let mut seen = Vec::new();
        loop {
            let step = match sched.next_fetch(None) {
                Ok(NextStep::Fetch(s)) => s,
                Ok(NextStep::Done) => break,
                other => unreachable!("{other:?}"),
            };
            seen.push((step.representation, step.number));
            for _ in 0..8 {
                sched.observe_throughput(100, 10_000_000);
            }
            sched.note_buffer_level(30.0);
            sched.segment_served();
        }
        // The 150k rung (document index 2) must appear after the
        // crash; numbers continue 3..=8 on the switched rung.
        assert!(
            seen.iter().any(|(rung, _)| *rung == 2),
            "down-step to lo: {seen:?}"
        );
        let numbers: Vec<u64> = {
            let mut n: Vec<u64> = seen.iter().map(|(_, n)| *n).collect();
            n.sort_unstable();
            n
        };
        assert_eq!(numbers, vec![3, 4, 5, 6, 7, 8], "numbers stay contiguous");
    }

    #[test]
    fn dynamic_timeline_waits_then_joins() {
        let mpd = parse(
            "<?xml version=\"1.0\"?>\
             <MPD type=\"dynamic\" availabilityStartTime=\"1970-01-01T00:00:00Z\" \
             minimumUpdatePeriod=\"PT2S\">\
             <Period start=\"PT0S\"><AdaptationSet contentType=\"video\">\
             <Representation id=\"v\" bandwidth=\"300000\">\
             <SegmentTemplate timescale=\"1\" media=\"live-$Number$.m4s\" startNumber=\"1\">\
             <SegmentTimeline><S t=\"0\" d=\"2\" r=\"9\"/></SegmentTimeline>\
             </SegmentTemplate></Representation>\
             </AdaptationSet></Period></MPD>",
        );
        let mut sched = scheduler(&mpd);
        // At now=4 the newest safely-available segment is #2
        // (available at 4s, +3s safe distance → not yet); #1 is
        // available at 2s, safe at 5s… so at now=4 even #1 waits.
        match sched.next_fetch(Some(4)) {
            Ok(NextStep::Wait { available_in_s }) => assert!(available_in_s > 0.0),
            other => unreachable!("expected Wait at now=4: {other:?}"),
        }
        // At now=10: segments available at 2,4,6,8,10 → safe ones
        // (avail+3 ≤ 10) are #1..#3; the join takes the newest → #3.
        match sched.next_fetch(Some(10)) {
            Ok(NextStep::Fetch(step)) => {
                assert_eq!(step.url, "http://unit.test/vod/live-3.m4s"); // lumen-allow-literal: URL parse/redaction test vector — data under test
                assert_eq!(step.number, 3);
            }
            other => unreachable!("expected Fetch at now=10: {other:?}"),
        }
        // After serving it, #4 (t=6, d=2 → available at 8s, ready
        // at 11s) is not yet safe at now=10: wait 1s.
        sched.segment_served();
        match sched.next_fetch(Some(10)) {
            Ok(NextStep::Wait { available_in_s }) => assert!((available_in_s - 1.0).abs() < 1e-9),
            other => unreachable!("expected Wait for #4: {other:?}"),
        }
        // At now=11 segment #4 is ready.
        match sched.next_fetch(Some(11)) {
            Ok(NextStep::Fetch(step)) => assert_eq!(step.url, "http://unit.test/vod/live-4.m4s"), // lumen-allow-literal: URL parse/redaction test vector — data under test
            other => unreachable!("expected Fetch at now=11: {other:?}"),
        }
    }

    #[test]
    fn empty_ladder_refuses_typed() {
        let mpd = parse(
            "<?xml version=\"1.0\"?><MPD type=\"static\"><Period><AdaptationSet contentType=\"video\">\
             </AdaptationSet></Period></MPD>",
        );
        assert!(matches!(
            DashScheduler::new(
                &mpd,
                &mpd.periods[0],
                &mpd.periods[0].adaptation_sets[0],
                "http://x/y.mpd", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
                DashAbrConfig::default()
            ),
            Err(DashSchedError::BadLadder)
        ));
    }
}
