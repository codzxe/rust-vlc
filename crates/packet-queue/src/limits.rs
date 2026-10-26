//! Queue limits, usage accounting and watermark thresholds (doc 03 §4.4).
//!
//! A queue is bounded on three axes at once — packets, bytes and media
//! duration — and *whichever hits first* refuses the next push (doc 03
//! §4.4(b), invariant Q1). The [`QueueLimits`] constructor is typed so an
//! unbounded configuration cannot be expressed: packet and byte caps are
//! [`NonZeroUsize`] (zero is unrepresentable at the type level) and the
//! duration cap must be a known, positive [`Duration`] — `Unknown` and
//! zero are rejected with a typed [`QueueConfigError`].

use std::num::NonZeroUsize;

use media_types::time::{Duration, Nanoseconds};

/// A non-zero cap from a compile-time literal (audit F-3, the
/// video-engine `policy.rs:186-190` precedent).
///
/// Only ever called with literals in this crate (the doc 03 §4.4(b)
/// defaults), and only from `const` items — so the impossible arm is
/// a **build failure**, not a runtime abort: a `loop` in const
/// position never yields a value, which means zeroing one of the
/// literal call sites below fails compilation instead of panicking
/// (or hanging) when the default is first used. The runtime abort
/// arm this replaces (`unreachable!`) only ever fired after the
/// damage was already shipped.
///
/// Scoped `allow`: clippy's suggested fix for the empty loop is
/// `panic!()` — denied by the workspace lint block — so this is a
/// false positive here; the arm is provably dead const code.
#[allow(clippy::empty_loop)]
const fn cap(literal: usize) -> NonZeroUsize {
    match NonZeroUsize::new(literal) {
        Some(n) => n,
        None => loop {},
    }
}

/// Doc 03 §4.4(b) audio-queue packet cap: 200. A `const` item so the
/// literal is const-evaluated (see [`cap`]).
const AUDIO_DEFAULT_PACKETS: NonZeroUsize = cap(200);

/// Doc 03 §4.4(b) video-queue packet cap: 300.
const VIDEO_DEFAULT_PACKETS: NonZeroUsize = cap(300);

/// Doc 03 §4.4(b) subtitle-queue packet cap: 128 (card 3.B3's
/// "small and duration-only" figures).
const SUBTITLE_DEFAULT_PACKETS: NonZeroUsize = cap(128);

/// One axis of a queue's boundedness — the limit that refused a push.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitAxis {
    /// The packet-count cap (`max_packets`).
    Packets,
    /// The byte cap (`max_bytes`).
    Bytes,
    /// The media-duration cap (`max_duration`).
    Duration,
}

impl std::fmt::Display for LimitAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LimitAxis::Packets => "packets",
            LimitAxis::Bytes => "bytes",
            LimitAxis::Duration => "duration",
        })
    }
}

/// Resident usage of a queue on all three bounded axes.
///
/// `packets` is the item count; `bytes` and `duration` are the sums of
/// the [`Weigh`](crate::Weigh) weights of the resident items. The sums
/// saturate rather than overflow (a queue at its caps can never exceed
/// `u64` nanoseconds or `usize` bytes anyway — see Q1 in the crate docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueUsage {
    /// Items currently resident.
    pub packets: usize,
    /// Sum of resident items' byte weights.
    pub bytes: usize,
    /// Sum of resident items' media durations.
    pub duration: Nanoseconds,
}

impl QueueUsage {
    /// All-zero usage (an empty queue).
    pub const EMPTY: QueueUsage = QueueUsage {
        packets: 0,
        bytes: 0,
        duration: Nanoseconds(0),
    };
}

/// The three simultaneous caps of a bounded queue (doc 03 §4.4(b)):
/// whichever is hit first refuses the next push.
///
/// An unbounded queue is unconstructible: see the module docs. The
/// doc 03 §4.4(b) defaults are exposed as [`QueueLimits::audio_default`]
/// and [`QueueLimits::video_default`] `[PROPOSAL: config in product.toml
/// cache limits per doc 03 — the engine reads its budget from there]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueLimits {
    /// Maximum resident packets.
    pub max_packets: NonZeroUsize,
    /// Maximum resident byte weight.
    pub max_bytes: NonZeroUsize,
    /// Maximum resident media duration.
    pub max_duration: Nanoseconds,
}

/// Invalid queue configuration — the only fallible thing this crate
/// owns (doc 03 §4.4(d): queue *operations* are total, typed signals).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueConfigError {
    /// `Duration::Unknown` was passed as a duration cap. An unknown
    /// duration cannot bound a queue — the queue would be unbounded.
    UnknownDuration,
    /// A zero duration cap was passed. Zero would refuse every item
    /// with a positive duration, which is a misconfiguration, not a
    /// limit.
    ZeroDuration,
}

impl std::fmt::Display for QueueConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueConfigError::UnknownDuration => {
                f.write_str("queue duration cap must be known, got Duration::Unknown")
            }
            QueueConfigError::ZeroDuration => {
                f.write_str("queue duration cap must be positive, got zero nanoseconds")
            }
        }
    }
}

impl std::error::Error for QueueConfigError {}

impl QueueLimits {
    /// Build limits from non-zero packet/byte caps and a duration cap.
    ///
    /// # Errors
    /// [`QueueConfigError::UnknownDuration`] when `max_duration` is
    /// `Duration::Unknown`, [`QueueConfigError::ZeroDuration`] when it
    /// is `Duration::Known(Nanoseconds(0))`. Packet and byte caps cannot
    /// be zero at all — the [`NonZeroUsize`] type makes that a compile
    /// error instead of a runtime one.
    pub fn new(
        max_packets: NonZeroUsize,
        max_bytes: NonZeroUsize,
        max_duration: Duration,
    ) -> Result<QueueLimits, QueueConfigError> {
        match max_duration {
            Duration::Known(n) if n.0 > 0 => Ok(QueueLimits {
                max_packets,
                max_bytes,
                max_duration: n,
            }),
            Duration::Known(_) => Err(QueueConfigError::ZeroDuration),
            Duration::Unknown => Err(QueueConfigError::UnknownDuration),
        }
    }

    /// Doc 03 §4.4(b) audio-queue defaults `[PROPOSAL]`: 1.0 s /
    /// 200 packets, hard-capped in bytes by the memory budget the
    /// caller passes (doc 03 leaves the byte figure to product.toml).
    #[must_use]
    pub fn audio_default(max_bytes: NonZeroUsize) -> QueueLimits {
        QueueLimits {
            max_packets: AUDIO_DEFAULT_PACKETS,
            max_bytes,
            max_duration: Nanoseconds(1_000_000_000),
        }
    }

    /// Doc 03 §4.4(b) video-queue defaults `[PROPOSAL]`: 2.0 s /
    /// 300 packets, hard-capped in bytes by the memory budget the
    /// caller passes.
    #[must_use]
    pub fn video_default(max_bytes: NonZeroUsize) -> QueueLimits {
        QueueLimits {
            max_packets: VIDEO_DEFAULT_PACKETS,
            max_bytes,
            max_duration: Nanoseconds(2_000_000_000),
        }
    }

    /// Doc 03 §4.4(b) third-kind defaults — the subtitle/event queue
    /// is "small and duration-only" (card 3.B3, design A3): 128
    /// packets / 10 s, hard-capped in bytes by the caller's budget
    /// `[PROPOSAL: figures — duration-led; wiring a subtitle limb is
    /// card 3.A3's, this is only the caps vocabulary]`.
    #[must_use]
    pub fn subtitle_default(max_bytes: NonZeroUsize) -> QueueLimits {
        QueueLimits {
            max_packets: SUBTITLE_DEFAULT_PACKETS,
            max_bytes,
            max_duration: Nanoseconds(10_000_000_000),
        }
    }

    /// Which axis (if any) an additional item of the given weight would
    /// violate right now — `None` when the item fits.
    ///
    /// **Single-item admission (the livelock invariant, 6.FIX-pcm):**
    /// an empty queue always admits one item, whatever its weight.
    /// The caps bound *accumulation* — how much may pile up behind
    /// residents — never the admission of a lone item: a producer
    /// whose every item is heavier than a cap (measured: the wav
    /// demuxer's 64 KiB PCM packets carry 2.73 s at 8 kHz against the
    /// audio lane's 1 s duration cap) would otherwise be refused
    /// *into the empty queue* forever — a livelock that stalls the
    /// session at its anchor with the held packet blocking the drain
    /// signal. Admitting alone keeps the queue bounded (usage ≤ cap +
    /// one item's weight — the item is beyond-cap by construction, and
    /// the consumer drains it) and preserves every accumulation
    /// refusal: with any resident present the axes answer exactly as
    /// before.
    ///
    /// The byte and duration sums are computed in `u128` so the
    /// *check* can never overflow even for adversarial weights; only a
    /// fitting item's weight is added to the resident sums.
    pub(crate) fn binding_axis(
        &self,
        resident: QueueUsage,
        add_packets: usize,
        add_bytes: usize,
        add_duration: Nanoseconds,
    ) -> Option<LimitAxis> {
        // The livelock invariant: a queue with nothing resident admits
        // one item unconditionally (see the method docs — the caps
        // bound accumulation, never a lone producer's admission).
        if resident.packets == 0 {
            return None;
        }
        let packets = (resident.packets + add_packets) as u128;
        let bytes = resident.bytes as u128 + add_bytes as u128;
        let duration = resident.duration.0 as u128 + add_duration.0 as u128;
        if packets > self.max_packets.get() as u128 {
            Some(LimitAxis::Packets)
        } else if bytes > self.max_bytes.get() as u128 {
            Some(LimitAxis::Bytes)
        } else if duration > self.max_duration.0 as u128 {
            Some(LimitAxis::Duration)
        } else {
            None
        }
    }
}

/// High/low watermark thresholds (doc 03 §4.4(b) `watermark()`, Q4).
///
/// A queue's watermark flags are the single input to `Buffering` state
/// transitions (Q4) — this crate only reports the state, the playback
/// engine owns the policy. Predicate `[PROPOSAL]`:
///
/// - `high` is hit when usage is **at or above the high threshold on
///   any axis** (eager: the queue is full enough in some dimension —
///   mirroring the "whichever hits first" enforcement philosophy);
/// - `low` is hit when usage is **at or below the low threshold on all
///   axes** (conservative: the queue is genuinely running dry, and a
///   stream of unknown-duration items can still reach `low` through
///   its packet count alone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatermarkThresholds {
    /// Usage at/above any axis threshold hits `high`.
    pub high: QueueUsage,
    /// Usage at/below all axis thresholds hits `low`.
    pub low: QueueUsage,
}

impl WatermarkThresholds {
    /// Thresholds at given positions.
    #[must_use]
    pub fn new(high: QueueUsage, low: QueueUsage) -> WatermarkThresholds {
        WatermarkThresholds { high, low }
    }

    /// Default thresholds derived from the limits: `high` fires exactly
    /// at capacity (any axis at its cap), `low` at one quarter of each
    /// cap, floored at one unit so a cap of 1 still yields a usable low
    /// signal `[PROPOSAL: the quarter is this crate's default; doc 03
    /// assigns the real policy to product.toml]`.
    #[must_use]
    pub fn from_limits(limits: &QueueLimits) -> WatermarkThresholds {
        let quarter = |cap_value: u64| (cap_value / 4).max(1);
        WatermarkThresholds {
            high: QueueUsage {
                packets: limits.max_packets.get(),
                bytes: limits.max_bytes.get(),
                duration: limits.max_duration,
            },
            low: QueueUsage {
                packets: quarter(limits.max_packets.get() as u64) as usize,
                bytes: quarter(limits.max_bytes.get() as u64) as usize,
                duration: Nanoseconds(quarter(limits.max_duration.0)),
            },
        }
    }
}

/// The watermark state of a queue (doc 03 §4.4(b)): hit flags plus the
/// resident usage they were computed from ("bytes, span" of the doc's
/// sketch — `usage.bytes` and `usage.duration` are the resident byte
/// weight and media span).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Watermark {
    /// Usage is at/above the high threshold on at least one axis.
    pub high: bool,
    /// Usage is at/below the low threshold on every axis.
    pub low: bool,
    /// The resident usage the flags were computed from.
    pub usage: QueueUsage,
}

impl Watermark {
    /// Evaluate the flags for a usage against thresholds.
    #[must_use]
    pub fn evaluate(usage: QueueUsage, marks: &WatermarkThresholds) -> Watermark {
        let high = usage.packets >= marks.high.packets
            || usage.bytes >= marks.high.bytes
            || usage.duration.0 >= marks.high.duration.0;
        let low = usage.packets <= marks.low.packets
            && usage.bytes <= marks.low.bytes
            && usage.duration.0 <= marks.low.duration.0;
        Watermark { high, low, usage }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    fn cap_of(v: usize) -> NonZeroUsize {
        NonZeroUsize::new(v).must()
    }

    #[test]
    fn limits_accept_known_positive_duration() {
        let l = QueueLimits::new(
            cap_of(10),
            cap_of(4096),
            Duration::Known(Nanoseconds(1_000)),
        )
        .must();
        assert_eq!(l.max_packets, cap_of(10));
        assert_eq!(l.max_bytes, cap_of(4096));
        assert_eq!(l.max_duration, Nanoseconds(1_000));
    }

    #[test]
    fn limits_reject_unknown_duration() {
        let err = assert_err(
            QueueLimits::new(cap_of(10), cap_of(4096), Duration::Unknown),
            "unknown duration cannot bound a queue",
        );
        assert_eq!(err, QueueConfigError::UnknownDuration);
        assert!(err.to_string().contains("known"), "{}", err);
    }

    #[test]
    fn limits_reject_zero_duration() {
        let err = assert_err(
            QueueLimits::new(cap_of(10), cap_of(4096), Duration::Known(Nanoseconds(0))),
            "zero duration cannot bound a queue",
        );
        assert_eq!(err, QueueConfigError::ZeroDuration);
        assert!(err.to_string().contains("positive"), "{}", err);
    }

    #[test]
    fn audio_default_matches_doc03() {
        let l = QueueLimits::audio_default(cap_of(1 << 20));
        assert_eq!(l.max_packets, cap_of(200));
        assert_eq!(l.max_duration, Nanoseconds(1_000_000_000));
        assert_eq!(l.max_bytes, cap_of(1 << 20));
    }

    #[test]
    fn video_default_matches_doc03() {
        let l = QueueLimits::video_default(cap_of(4 << 20));
        assert_eq!(l.max_packets, cap_of(300));
        assert_eq!(l.max_duration, Nanoseconds(2_000_000_000));
    }

    #[test]
    fn subtitle_default_is_small_and_duration_led() {
        // Doc 03 §4.4(b): the third kind's queue is "small and
        // duration-only" — card 3.B3's [PROPOSAL] figures.
        let l = QueueLimits::subtitle_default(cap_of(16 << 20));
        assert_eq!(l.max_packets, cap_of(128));
        assert_eq!(l.max_bytes, cap_of(16 << 20));
        assert_eq!(l.max_duration, Nanoseconds(10_000_000_000));
    }

    #[test]
    fn binding_axis_reports_the_first_violated_cap() {
        let l =
            QueueLimits::new(cap_of(2), cap_of(100), Duration::Known(Nanoseconds(1_000))).must();
        // Two resident, one more → packets.
        let resident = QueueUsage {
            packets: 2,
            bytes: 0,
            duration: Nanoseconds(0),
        };
        assert_eq!(
            l.binding_axis(resident, 1, 0, Nanoseconds(0)),
            Some(LimitAxis::Packets)
        );
        // Bytes: 60 resident, another 60 → 120 > 100.
        let resident = QueueUsage {
            packets: 0,
            bytes: 60,
            duration: Nanoseconds(0),
        };
        assert_eq!(
            l.binding_axis(resident, 1, 60, Nanoseconds(0)),
            Some(LimitAxis::Bytes)
        );
        // Duration: 600 resident, 600 more → 1200 > 1000.
        let resident = QueueUsage {
            packets: 0,
            bytes: 0,
            duration: Nanoseconds(600),
        };
        assert_eq!(
            l.binding_axis(resident, 1, 0, Nanoseconds(600)),
            Some(LimitAxis::Duration)
        );
        // Fits on all axes.
        assert_eq!(
            l.binding_axis(QueueUsage::EMPTY, 1, 100, Nanoseconds(1_000)),
            None
        );
    }

    #[test]
    fn binding_axis_checks_in_u128_so_huge_weights_cannot_wrap() {
        let l = QueueLimits::new(cap_of(4), cap_of(10), Duration::Known(Nanoseconds(10))).must();
        let resident = QueueUsage {
            packets: 0,
            bytes: usize::MAX,
            duration: Nanoseconds(u64::MAX),
        };
        assert_eq!(
            l.binding_axis(resident, 1, usize::MAX, Nanoseconds(u64::MAX)),
            Some(LimitAxis::Bytes)
        );
    }

    #[test]
    fn watermark_default_high_is_capacity_low_is_a_quarter() {
        let l = QueueLimits::new(cap_of(8), cap_of(400), Duration::Known(Nanoseconds(800))).must();
        let m = WatermarkThresholds::from_limits(&l);
        assert_eq!(m.high.packets, 8);
        assert_eq!(m.high.bytes, 400);
        assert_eq!(m.high.duration, Nanoseconds(800));
        assert_eq!(m.low.packets, 2);
        assert_eq!(m.low.bytes, 100);
        assert_eq!(m.low.duration, Nanoseconds(200));
    }

    #[test]
    fn watermark_flags_follow_the_predicate() {
        let l = QueueLimits::new(cap_of(8), cap_of(400), Duration::Known(Nanoseconds(800))).must();
        let m = WatermarkThresholds::from_limits(&l);
        // Empty: low only.
        let w = Watermark::evaluate(QueueUsage::EMPTY, &m);
        assert!(w.low && !w.high);
        // At capacity on one axis: high (eager), not low.
        let w = Watermark::evaluate(
            QueueUsage {
                packets: 8,
                bytes: 1,
                duration: Nanoseconds(0),
            },
            &m,
        );
        assert!(w.high && !w.low);
        // Below low on packets but above low on bytes: neither — the
        // low predicate requires ALL axes low.
        let w = Watermark::evaluate(
            QueueUsage {
                packets: 1,
                bytes: 300,
                duration: Nanoseconds(0),
            },
            &m,
        );
        assert!(!w.low && !w.high);
        // All axes at/below their low thresholds: low.
        let w = Watermark::evaluate(
            QueueUsage {
                packets: 2,
                bytes: 100,
                duration: Nanoseconds(200),
            },
            &m,
        );
        assert!(w.low && !w.high);
    }

    #[test]
    fn config_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(QueueConfigError::ZeroDuration);
        assert!(!err.to_string().is_empty());
    }
}
