//! The reusable codec contract suite (doc 01 §2.4(e), card 1.D1-R C2) —
//! the executable version of the trait contracts, for every codec in
//! `codecs` and the conformance kit (MP §6/§8).
//!
//! The suite is generic over any [`Decoder`]/[`Encoder`]: a codec crate
//! configures its implementation, supplies a few fixture packets or
//! frames, and calls the checks from its own tests:
//!
//! ```no_run
//! use codec_api::contract;
//! use codec_api::prelude::*;
//! use media_types::GenerationId;
//!
//! # struct MyDecoder;
//! # impl Decoder for MyDecoder {
//! #     fn configure(&mut self, _: &DecoderConfig) -> Result<(), EngineError> { Ok(()) }
//! #     fn decode(&mut self, _: Packet, _: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> { Ok(DecodeOutcome::NeedMoreInput) }
//! #     fn flush(&mut self, _: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> { Ok(DecodeOutcome::Drained) }
//! # }
//! # fn fixtures() -> Vec<Packet> { Vec::new() }
//! # fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let mut decoder = MyDecoder;
//! let current = GenerationId::FIRST.next()?;
//! decoder.configure(&DecoderConfig::new(
//!     CodecDescriptor::new("mycodec")?,
//!     current,
//!     ThreadBudget::default(),
//!     None,
//!     None,
//! ))?;
//! let packets = fixtures(); // valid packets for YOUR codec
//! for packet in &packets {
//!     decoder.decode(packet.clone(), &mut contract::CountingFrameSink::new())?;
//! }
//! contract::flush_drains(&mut decoder)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Preconditions
//!
//! * The implementation is already configured, and the configured
//!   generation equals the `current` passed to the stale checks.
//! * Fixtures are honest: "emitting" fixtures must produce output, and
//!   "corrupt" fixtures must be corruption the codec is required to
//!   detect. The suite reports fixture bugs as violations with distinct
//!   wording.
//!
//! # What is mechanically enforced
//!
//! Stale-packet/stale-frame drop on sight; flush draining to `Drained`
//! and staying there; corrupt input producing bounded recovery
//! (`Recovered` with a sane `skipped` count) or a *typed* fatal error;
//! sink-error propagation; outcome/sink agreement. "Bounded work" is
//! enforced as *termination*: every check is a finite driving loop, so a
//! decoder that never returns hangs the test (CI timeouts catch it).
//! Panic-freedom cannot be enforced without poisoning the worker; it
//! stays a documented contract violation (see [`crate::decode`]).

use crate::decode::{DecodeOutcome, Decoder, FrameSink, RecoverReason};
use crate::encode::{EncodeOutcome, Encoder, PacketSink};
use media_types::error::{DeviceCause, DeviceSlot, ErrorContext};
use media_types::{EngineError, ErrorKind, Frame, GenerationId, Packet, PipelineStage};

/// How many `flush` calls the drain check will make before declaring
/// the drain unbounded. Generous: each call must make progress (accept
/// at least one frame), so this bounds buffered backlog, not per-call
/// work.
const MAX_FLUSH_CALLS: u32 = 1024;

/// A violated contract, described for the failing test.
///
/// `Display` carries the explanation (including fixture-bug wording);
/// there is no machine-readable code — callers format the violation
/// into their test failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractViolation {
    detail: String,
}

impl ContractViolation {
    /// The explanation.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for ContractViolation {}

/// Internal shorthand for the check bodies.
fn violation(detail: impl Into<String>) -> ContractViolation {
    ContractViolation {
        detail: detail.into(),
    }
}

// ---- suite-owned sinks --------------------------------------------------

/// A [`FrameSink`] that counts and retains everything accepted — for
/// driving decoders in tests and asserting outcome/sink agreement.
#[derive(Debug, Default)]
pub struct CountingFrameSink {
    accepted: u32,
    frames: Vec<Frame>,
}

impl CountingFrameSink {
    /// An empty sink.
    pub fn new() -> CountingFrameSink {
        CountingFrameSink::default()
    }

    /// Frames accepted so far.
    pub fn accepted(&self) -> u32 {
        self.accepted
    }

    /// The retained frames, in acceptance order.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Forget the count and the retained frames.
    pub fn reset(&mut self) {
        self.accepted = 0;
        self.frames.clear();
    }
}

impl FrameSink for CountingFrameSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        self.accepted = self.accepted.saturating_add(1);
        self.frames.push(frame);
        Ok(())
    }
}

/// A [`PacketSink`] mirror of [`CountingFrameSink`].
#[derive(Debug, Default)]
pub struct CountingPacketSink {
    accepted: u32,
    packets: Vec<Packet>,
}

impl CountingPacketSink {
    /// An empty sink.
    pub fn new() -> CountingPacketSink {
        CountingPacketSink::default()
    }

    /// Packets accepted so far.
    pub fn accepted(&self) -> u32 {
        self.accepted
    }

    /// The retained packets, in acceptance order.
    pub fn packets(&self) -> &[Packet] {
        &self.packets
    }

    /// Forget the count and the retained packets.
    pub fn reset(&mut self) {
        self.accepted = 0;
        self.packets.clear();
    }
}

impl PacketSink for CountingPacketSink {
    fn accept(&mut self, packet: Packet) -> Result<(), EngineError> {
        self.accepted = self.accepted.saturating_add(1);
        self.packets.push(packet);
        Ok(())
    }
}

/// Marker chained into the suite's synthetic sink failures so that
/// propagation can be proven even when an implementation re-mints the
/// error with its own context (verbatim propagation is detected by the
/// context code).
#[derive(Debug)]
struct SinkFailureMarker;

impl std::fmt::Display for SinkFailureMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("contract sink failure marker")
    }
}

impl std::error::Error for SinkFailureMarker {}

/// The synthetic error the failing sinks return — shaped like a
/// downstream presentation failure (a kind no decoder mints itself).
fn sink_failure() -> EngineError {
    EngineError::new(
        PipelineStage::Present,
        ErrorKind::Device {
            which: DeviceSlot::Display,
            cause: DeviceCause::Disconnected,
        },
        ErrorContext::new("contract.sink.failure"),
    )
    .with_source(SinkFailureMarker)
}

/// Whether `err` is the suite's sink failure — verbatim (context code)
/// or chained via `with_source`.
fn is_sink_failure(err: &EngineError) -> bool {
    if err.context.code == "contract.sink.failure" {
        return true;
    }
    let mut source = std::error::Error::source(err);
    while let Some(link) = source {
        if link.downcast_ref::<SinkFailureMarker>().is_some() {
            return true;
        }
        source = link.source();
    }
    false
}

/// A [`FrameSink`] that fails every `accept` with the suite's marker
/// error, counting attempts so tests can tell "swallowed" from "never
/// called".
#[derive(Debug, Default)]
pub struct FailingFrameSink {
    attempts: u32,
}

impl FailingFrameSink {
    /// A fresh failing sink.
    pub fn new() -> FailingFrameSink {
        FailingFrameSink::default()
    }

    /// How many times `accept` was called (and refused).
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

impl FrameSink for FailingFrameSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.attempts = self.attempts.saturating_add(1);
        Err(sink_failure())
    }
}

/// A [`PacketSink`] mirror of [`FailingFrameSink`].
#[derive(Debug, Default)]
pub struct FailingPacketSink {
    attempts: u32,
}

impl FailingPacketSink {
    /// A fresh failing sink.
    pub fn new() -> FailingPacketSink {
        FailingPacketSink::default()
    }

    /// How many times `accept` was called (and refused).
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

impl PacketSink for FailingPacketSink {
    fn accept(&mut self, _packet: Packet) -> Result<(), EngineError> {
        self.attempts = self.attempts.saturating_add(1);
        Err(sink_failure())
    }
}

// ---- outcome/sink agreement ---------------------------------------------

/// Outcome/sink agreement for decode calls: `Emitted { frames: n }`
/// means exactly `n` accepts happened in that call (and `n >= 1`);
/// `NeedMoreInput`/`Drained` mean zero; `Recovered` is unconstrained
/// (concealed output is legal).
///
/// # Errors
/// [`ContractViolation`] describing the mismatch.
pub fn decode_outcome_agrees_with_sink(
    outcome: DecodeOutcome,
    accepts: u32,
) -> Result<(), ContractViolation> {
    match outcome {
        DecodeOutcome::Emitted { frames } => {
            if frames == 0 {
                Err(violation(
                    "Emitted { frames: 0 } lies: nothing emitted is NeedMoreInput",
                ))
            } else if frames != accepts {
                Err(violation(format!(
                    "outcome/sink disagreement: Emitted {{ frames: {frames} }} after {accepts} accept(s)"
                )))
            } else {
                Ok(())
            }
        }
        DecodeOutcome::NeedMoreInput if accepts != 0 => Err(violation(format!(
            "outcome/sink disagreement: NeedMoreInput after {accepts} accept(s)"
        ))),
        DecodeOutcome::Drained if accepts != 0 => Err(violation(format!(
            "outcome/sink disagreement: Drained after {accepts} accept(s)"
        ))),
        _ => Ok(()),
    }
}

/// Outcome/sink agreement for encode calls — mirror of
/// [`decode_outcome_agrees_with_sink`].
///
/// # Errors
/// [`ContractViolation`] describing the mismatch.
pub fn encode_outcome_agrees_with_sink(
    outcome: EncodeOutcome,
    accepts: u32,
) -> Result<(), ContractViolation> {
    match outcome {
        EncodeOutcome::Emitted { packets } => {
            if packets == 0 {
                Err(violation(
                    "Emitted { packets: 0 } lies: nothing emitted is NeedMoreInput",
                ))
            } else if packets != accepts {
                Err(violation(format!(
                    "outcome/sink disagreement: Emitted {{ packets: {packets} }} after {accepts} accept(s)"
                )))
            } else {
                Ok(())
            }
        }
        EncodeOutcome::NeedMoreInput if accepts != 0 => Err(violation(format!(
            "outcome/sink disagreement: NeedMoreInput after {accepts} accept(s)"
        ))),
        EncodeOutcome::Drained if accepts != 0 => Err(violation(format!(
            "outcome/sink disagreement: Drained after {accepts} accept(s)"
        ))),
        _ => Ok(()),
    }
}

// ---- decoder checks ------------------------------------------------------

/// Contract: a stale packet is dropped on sight — `Ok(NeedMoreInput)`,
/// zero frames, no error — and the decoder still drains afterwards.
///
/// `stale` must be stale against `current` (the configured generation);
/// otherwise the violation is a fixture bug.
///
/// # Errors
/// [`ContractViolation`] naming the broken rule (or the fixture bug).
#[must_use = "a contract check's result is the test's verdict"]
pub fn stale_packet_is_dropped_on_sight<D: Decoder>(
    decoder: &mut D,
    stale: &Packet,
    current: GenerationId,
) -> Result<(), ContractViolation> {
    if !stale.is_stale(current) {
        return Err(violation(
            "fixture bug: the packet is not stale against the supplied generation",
        ));
    }
    let mut sink = CountingFrameSink::new();
    match decoder.decode(stale.clone(), &mut sink) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        Ok(other) => {
            return Err(violation(format!(
                "stale packet must be dropped on sight as NeedMoreInput, got {other:?}"
            )));
        }
        Err(err) => {
            return Err(violation(format!(
                "stale drop is not an error (dropped on sight, reported as NeedMoreInput): {err}"
            )));
        }
    }
    if sink.accepted() != 0 {
        return Err(violation(format!(
            "stale drop emitted {} frame(s): cancellation must not decode",
            sink.accepted()
        )));
    }
    let mut drain = CountingFrameSink::new();
    match decoder.flush(&mut drain) {
        Ok(_) => Ok(()),
        Err(err) => Err(violation(format!(
            "decoder unusable after a stale drop: {err}"
        ))),
    }
}

/// Contract: `flush` reaches `Drained` within bounded calls (making
/// progress on the way) and stays `Drained` once there.
///
/// # Errors
/// [`ContractViolation`] naming the broken rule.
#[must_use = "a contract check's result is the test's verdict"]
pub fn flush_drains<D: Decoder>(decoder: &mut D) -> Result<(), ContractViolation> {
    let mut sink = CountingFrameSink::new();
    for _ in 0..MAX_FLUSH_CALLS {
        let before = sink.accepted();
        match decoder.flush(&mut sink) {
            Err(err) => {
                return Err(violation(format!(
                    "flush must not fail while draining: {err}"
                )));
            }
            Ok(DecodeOutcome::Drained) => {
                decode_outcome_agrees_with_sink(DecodeOutcome::Drained, sink.accepted() - before)?;
                // Drained is sticky.
                let mut sticky = CountingFrameSink::new();
                return match decoder.flush(&mut sticky) {
                    Ok(DecodeOutcome::Drained) if sticky.accepted() == 0 => Ok(()),
                    Ok(other) => Err(violation(format!(
                        "Drained must be sticky: further flush returned {other:?} with {} frame(s)",
                        sticky.accepted()
                    ))),
                    Err(err) => Err(violation(format!(
                        "flush must not fail once drained: {err}"
                    ))),
                };
            }
            Ok(DecodeOutcome::NeedMoreInput) => {
                return Err(violation(
                    "flush cannot ask for more input: draining means input is over",
                ));
            }
            Ok(outcome @ DecodeOutcome::Emitted { .. }) => {
                decode_outcome_agrees_with_sink(outcome, sink.accepted() - before)?;
                if sink.accepted() == before {
                    return Err(violation(
                        "flush made no progress: Emitted without accepting a frame",
                    ));
                }
            }
            Ok(outcome @ DecodeOutcome::Recovered { .. }) => {
                // Recovery mid-drain is legal (e.g. HwFallback); the
                // loop's bound still applies.
                decode_outcome_agrees_with_sink(outcome, sink.accepted() - before)?;
            }
        }
    }
    Err(violation(format!(
        "flush never reached Drained within {MAX_FLUSH_CALLS} bounded calls"
    )))
}

/// Contract: corrupt input yields bounded recovery —
/// `Ok(Recovered { skipped, reason })` — or a *typed* fatal error
/// ([`ErrorKind::Decoder`] or [`ErrorKind::Corrupt`]); silent success is
/// a violation, and so is an unrelated error kind.
///
/// After a `Recovered`, the decoder must still drain (it stayed
/// usable); after a typed `Err` it merely has to have answered this
/// call — fatality is per-generation by contract.
///
/// # Errors
/// [`ContractViolation`] naming the broken rule (or the fixture bug).
#[must_use = "a contract check's result is the test's verdict"]
pub fn corrupt_input_recovers_or_errors_typed<D: Decoder>(
    decoder: &mut D,
    corrupt: &Packet,
) -> Result<(), ContractViolation> {
    let mut sink = CountingFrameSink::new();
    match decoder.decode(corrupt.clone(), &mut sink) {
        Ok(outcome @ DecodeOutcome::Recovered { skipped, reason }) => {
            decode_outcome_agrees_with_sink(outcome, sink.accepted())?;
            if reason == RecoverReason::CorruptInput && skipped == 0 {
                return Err(violation(
                    "CorruptInput recovery must skip at least one unit (the corrupt one)",
                ));
            }
            let mut drain = CountingFrameSink::new();
            match decoder.flush(&mut drain) {
                Ok(_) => Ok(()),
                Err(err) => Err(violation(format!(
                    "decoder unusable after recovering: {err}"
                ))),
            }
        }
        Err(err) => match &err.kind {
            ErrorKind::Decoder { .. } | ErrorKind::Corrupt { .. } => Ok(()),
            other => Err(violation(format!(
                "corrupt input produced an unrelated error kind ({other:?}): {err}"
            ))),
        },
        Ok(DecodeOutcome::Emitted { frames }) => Err(violation(format!(
            "corruption went undetected: decode reported Emitted {{ frames: {frames} }}"
        ))),
        Ok(DecodeOutcome::NeedMoreInput) => Err(violation(
            "corruption went undetected: decode reported NeedMoreInput",
        )),
        Ok(DecodeOutcome::Drained) => Err(violation(
            "corruption went undetected: decode reported Drained",
        )),
    }
}

/// Contract: an [`EngineError`] from the sink stops the call and
/// propagates — verbatim or chained via `with_source`.
///
/// `emitting` must be a packet that produces at least one frame; the
/// check distinguishes a swallowing decoder from a non-emitting fixture.
///
/// # Errors
/// [`ContractViolation`] naming the broken rule (or the fixture bug).
#[must_use = "a contract check's result is the test's verdict"]
pub fn sink_errors_propagate<D: Decoder>(
    decoder: &mut D,
    emitting: &Packet,
) -> Result<(), ContractViolation> {
    let mut sink = FailingFrameSink::new();
    match decoder.decode(emitting.clone(), &mut sink) {
        Err(err) if is_sink_failure(&err) => {
            // The decoder stopped and told the engine; it must remain
            // drainable afterwards (the failure was downstream, not its
            // own state).
            let mut drain = CountingFrameSink::new();
            match decoder.flush(&mut drain) {
                Ok(_) => Ok(()),
                Err(err) => Err(violation(format!(
                    "decoder unusable after a downstream sink failure: {err}"
                ))),
            }
        }
        Err(err) => Err(violation(format!(
            "decode failed for a non-sink reason — supply a packet that emits: {err}"
        ))),
        Ok(outcome) => {
            if sink.attempts() == 0 {
                Err(violation(
                    "fixture bug: the packet emitted nothing — supply a packet that produces at least one frame",
                ))
            } else {
                Err(violation(format!(
                    "sink failure swallowed: accept refused {attempt} frame(s) yet decode returned {outcome:?}",
                    attempt = sink.attempts()
                )))
            }
        }
    }
}

// ---- encoder checks (mirrors) --------------------------------------------

/// Contract mirror of [`stale_packet_is_dropped_on_sight`] for frames.
///
/// # Errors
/// [`ContractViolation`] naming the broken rule (or the fixture bug).
#[must_use = "a contract check's result is the test's verdict"]
pub fn stale_frame_is_dropped_on_sight<E: Encoder>(
    encoder: &mut E,
    stale: &Frame,
    current: GenerationId,
) -> Result<(), ContractViolation> {
    if !stale.is_stale(current) {
        return Err(violation(
            "fixture bug: the frame is not stale against the supplied generation",
        ));
    }
    let mut sink = CountingPacketSink::new();
    match encoder.encode(stale.clone(), &mut sink) {
        Ok(EncodeOutcome::NeedMoreInput) => {}
        Ok(other) => {
            return Err(violation(format!(
                "stale frame must be dropped on sight as NeedMoreInput, got {other:?}"
            )));
        }
        Err(err) => {
            return Err(violation(format!(
                "stale drop is not an error (dropped on sight, reported as NeedMoreInput): {err}"
            )));
        }
    }
    if sink.accepted() != 0 {
        return Err(violation(format!(
            "stale drop emitted {} packet(s): cancellation must not encode",
            sink.accepted()
        )));
    }
    let mut drain = CountingPacketSink::new();
    match encoder.flush(&mut drain) {
        Ok(_) => Ok(()),
        Err(err) => Err(violation(format!(
            "encoder unusable after a stale drop: {err}"
        ))),
    }
}

/// Contract mirror of [`flush_drains`].
///
/// # Errors
/// [`ContractViolation`] naming the broken rule.
#[must_use = "a contract check's result is the test's verdict"]
pub fn encoder_flush_drains<E: Encoder>(encoder: &mut E) -> Result<(), ContractViolation> {
    let mut sink = CountingPacketSink::new();
    for _ in 0..MAX_FLUSH_CALLS {
        let before = sink.accepted();
        match encoder.flush(&mut sink) {
            Err(err) => {
                return Err(violation(format!(
                    "flush must not fail while draining: {err}"
                )));
            }
            Ok(EncodeOutcome::Drained) => {
                encode_outcome_agrees_with_sink(EncodeOutcome::Drained, sink.accepted() - before)?;
                let mut sticky = CountingPacketSink::new();
                return match encoder.flush(&mut sticky) {
                    Ok(EncodeOutcome::Drained) if sticky.accepted() == 0 => Ok(()),
                    Ok(other) => Err(violation(format!(
                        "Drained must be sticky: further flush returned {other:?} with {} packet(s)",
                        sticky.accepted()
                    ))),
                    Err(err) => Err(violation(format!(
                        "flush must not fail once drained: {err}"
                    ))),
                };
            }
            Ok(EncodeOutcome::NeedMoreInput) => {
                return Err(violation(
                    "flush cannot ask for more input: draining means input is over",
                ));
            }
            Ok(outcome @ EncodeOutcome::Emitted { .. }) => {
                encode_outcome_agrees_with_sink(outcome, sink.accepted() - before)?;
                if sink.accepted() == before {
                    return Err(violation(
                        "flush made no progress: Emitted without accepting a packet",
                    ));
                }
            }
            Ok(outcome @ EncodeOutcome::Recovered { .. }) => {
                encode_outcome_agrees_with_sink(outcome, sink.accepted() - before)?;
            }
        }
    }
    Err(violation(format!(
        "flush never reached Drained within {MAX_FLUSH_CALLS} bounded calls"
    )))
}

/// Contract mirror of [`corrupt_input_recovers_or_errors_typed`] for
/// malformed input frames (e.g. a video frame whose buffer does not
/// match its declared dimensions).
///
/// # Errors
/// [`ContractViolation`] naming the broken rule (or the fixture bug).
#[must_use = "a contract check's result is the test's verdict"]
pub fn corrupt_frame_recovers_or_errors_typed<E: Encoder>(
    encoder: &mut E,
    corrupt: &Frame,
) -> Result<(), ContractViolation> {
    let mut sink = CountingPacketSink::new();
    match encoder.encode(corrupt.clone(), &mut sink) {
        Ok(outcome @ EncodeOutcome::Recovered { skipped, reason }) => {
            encode_outcome_agrees_with_sink(outcome, sink.accepted())?;
            if reason == RecoverReason::CorruptInput && skipped == 0 {
                return Err(violation(
                    "CorruptInput recovery must skip at least one unit (the corrupt one)",
                ));
            }
            let mut drain = CountingPacketSink::new();
            match encoder.flush(&mut drain) {
                Ok(_) => Ok(()),
                Err(err) => Err(violation(format!(
                    "encoder unusable after recovering: {err}"
                ))),
            }
        }
        Err(err) => match &err.kind {
            ErrorKind::Decoder { .. } | ErrorKind::Corrupt { .. } => Ok(()),
            other => Err(violation(format!(
                "corrupt frame produced an unrelated error kind ({other:?}): {err}"
            ))),
        },
        Ok(EncodeOutcome::Emitted { packets }) => Err(violation(format!(
            "corruption went undetected: encode reported Emitted {{ packets: {packets} }}"
        ))),
        Ok(EncodeOutcome::NeedMoreInput) => Err(violation(
            "corruption went undetected: encode reported NeedMoreInput",
        )),
        Ok(EncodeOutcome::Drained) => Err(violation(
            "corruption went undetected: encode reported Drained",
        )),
    }
}

/// Contract mirror of [`sink_errors_propagate`].
///
/// # Errors
/// [`ContractViolation`] naming the broken rule (or the fixture bug).
#[must_use = "a contract check's result is the test's verdict"]
pub fn packet_sink_errors_propagate<E: Encoder>(
    encoder: &mut E,
    emitting: &Frame,
) -> Result<(), ContractViolation> {
    let mut sink = FailingPacketSink::new();
    match encoder.encode(emitting.clone(), &mut sink) {
        Err(err) if is_sink_failure(&err) => {
            let mut drain = CountingPacketSink::new();
            match encoder.flush(&mut drain) {
                Ok(_) => Ok(()),
                Err(err) => Err(violation(format!(
                    "encoder unusable after a downstream sink failure: {err}"
                ))),
            }
        }
        Err(err) => Err(violation(format!(
            "encode failed for a non-sink reason — supply a frame that emits: {err}"
        ))),
        Ok(outcome) => {
            if sink.attempts() == 0 {
                Err(violation(
                    "fixture bug: the frame emitted nothing — supply a frame that produces at least one packet",
                ))
            } else {
                Err(violation(format!(
                    "sink failure swallowed: accept refused {attempt} packet(s) yet encode returned {outcome:?}",
                    attempt = sink.attempts()
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::error::DecodeCause;
    use media_types::packet::SharedBytes;

    #[test]
    fn agreement_rejects_lies_and_accepts_truth() {
        // Truth.
        assert!(decode_outcome_agrees_with_sink(DecodeOutcome::Emitted { frames: 2 }, 2).is_ok());
        assert!(decode_outcome_agrees_with_sink(DecodeOutcome::NeedMoreInput, 0).is_ok());
        assert!(decode_outcome_agrees_with_sink(DecodeOutcome::Drained, 0).is_ok());
        assert!(
            decode_outcome_agrees_with_sink(
                DecodeOutcome::Recovered {
                    skipped: 1,
                    reason: RecoverReason::ConcealedFrame
                },
                5
            )
            .is_ok(),
            "Recovered may conceal-and-emit"
        );
        // Lies.
        assert!(decode_outcome_agrees_with_sink(DecodeOutcome::Emitted { frames: 2 }, 1).is_err());
        assert!(decode_outcome_agrees_with_sink(DecodeOutcome::Emitted { frames: 0 }, 0).is_err());
        assert!(decode_outcome_agrees_with_sink(DecodeOutcome::NeedMoreInput, 1).is_err());
        assert!(decode_outcome_agrees_with_sink(DecodeOutcome::Drained, 3).is_err());
        // Encode mirror.
        assert!(encode_outcome_agrees_with_sink(EncodeOutcome::Emitted { packets: 1 }, 1).is_ok());
        assert!(encode_outcome_agrees_with_sink(EncodeOutcome::Emitted { packets: 1 }, 2).is_err());
        assert!(encode_outcome_agrees_with_sink(EncodeOutcome::NeedMoreInput, 0).is_ok());
        assert!(encode_outcome_agrees_with_sink(EncodeOutcome::Emitted { packets: 0 }, 0).is_err());
    }

    #[test]
    fn sink_failure_is_detectable_verbatim_and_chained() {
        // Verbatim propagation: context code matches.
        let err = sink_failure();
        assert!(is_sink_failure(&err));
        // Re-minted but chained via with_source: marker found in the
        // source chain.
        let reminted = EngineError::new(
            PipelineStage::Decode,
            ErrorKind::Decoder {
                cause: DecodeCause::BadState,
            },
            ErrorContext::new("mock.reminted"),
        )
        .with_source(err);
        assert!(is_sink_failure(&reminted));
        // An unrelated error is not mistaken for ours.
        let unrelated = EngineError::new(
            PipelineStage::Decode,
            ErrorKind::Decoder {
                cause: DecodeCause::Bitstream,
            },
            ErrorContext::new("mock.bitstream"),
        );
        assert!(!is_sink_failure(&unrelated));
    }

    #[test]
    fn failing_sinks_refuse_and_count_attempts() {
        let mut frames = FailingFrameSink::new();
        let frame = Frame::Subtitle(media_types::frame::SubtitleFrame {
            cues: Vec::new(),
            pts: media_types::Stamp::Missing,
            generation: GenerationId::FIRST,
        });
        assert!(frames.accept(frame).is_err());
        assert_eq!(frames.attempts(), 1);

        let mut packets = FailingPacketSink::new();
        let packet = Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(vec![0x00]),
            keyframe: false,
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
        assert!(packets.accept(packet).is_err());
        assert_eq!(packets.attempts(), 1);
    }

    #[test]
    fn counting_sinks_retain_and_reset() {
        let mut sink = CountingFrameSink::new();
        assert_eq!(sink.accepted(), 0);
        assert!(sink.frames().is_empty());
        let frame = Frame::Subtitle(media_types::frame::SubtitleFrame {
            cues: Vec::new(),
            pts: media_types::Stamp::Missing,
            generation: GenerationId::FIRST,
        });
        match sink.accept(frame) {
            Ok(()) => {}
            Err(err) => unreachable!("{err}"),
        }
        assert_eq!(sink.accepted(), 1);
        assert_eq!(sink.frames().len(), 1);
        sink.reset();
        assert_eq!(sink.accepted(), 0);
        assert!(sink.frames().is_empty());

        let mut packets = CountingPacketSink::new();
        let packet = Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(vec![0x00]),
            keyframe: false,
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
        match packets.accept(packet) {
            Ok(()) => {}
            Err(err) => unreachable!("{err}"),
        }
        assert_eq!(packets.accepted(), 1);
        assert_eq!(packets.packets().len(), 1);
        packets.reset();
        assert_eq!(packets.accepted(), 0);
    }

    #[test]
    fn violation_reports_its_detail() {
        let v = violation("something broke");
        assert_eq!(v.detail(), "something broke");
        assert_eq!(v.to_string(), "something broke");
    }
}
