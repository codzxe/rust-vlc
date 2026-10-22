//! The three-way exhaustive proof (card 3.C1's model-based AC): for
//! every one of the `8 × 12 = 96` `(state, input)` pairs, three
//! **independently authored** opinions of the documented machine must
//! agree exactly — target state, side effects, and typed rejection
//! reason:
//!
//! 1. the engine's data table (`state.rs::TRANSITIONS`, through
//!    [`apply_input`] — the only mover);
//! 2. the imperative doc-cited oracle (`tests/model/oracle.rs` —
//!    straight-line code, no table lookup);
//! 3. the committed expected-table artifact
//!    (`tests/model/expected_table.toml` — the human review surface).
//!
//! This kills the self-referential echo chamber the 2.C2 suite
//! carried (directive D-5): `table.rs` proof #1 checked the table
//! against itself; here a second and a third author must write the
//! same spec-derived truth. The artifact is parsed by a hand-rolled
//! std-only reader for its deliberately restricted grammar
//! (playback-engine has a zero-external-dependency contract, C5 — no
//! `toml` crate; the grammar is `[[pair]]` headers with `state`,
//! `input`, and either `to`+`effects` or `reject` string/array
//! values, exactly as the artifact's header documents).
//!
//! Beside the per-pair agreement, this file pins the shape invariants
//! the design's row-count arithmetic demands: exactly 96 artifact
//! entries with no duplicate pair, 30 legal rows / 66 typed
//! rejections, no duplicate rows, every legal row wire-conformant
//! (`PlaybackState::can_transition_to`, doc 05 §3.3), and the
//! artifact's own edge projection equal to the wire edge set.

// The oracle is a module beside the artifact (design: tests/model/),
// pulled in by path — cargo only auto-discovers top-level tests/*.rs.
#[path = "model/oracle.rs"]
mod oracle;

use oracle::{ModelAnswer, model_answer};
use playback_engine::state::{
    Effect, RejectionReason, SessionInput, SessionState, TRANSITIONS, apply_input,
};
use testkit::Must;

// ---------------------------------------------------------------------------
// The artifact reader (restricted grammar, std-only)
// ---------------------------------------------------------------------------

/// One artifact entry: the pair and its expected classification.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PairExpectation {
    state: SessionState,
    input: SessionInput,
    answer: ModelAnswer,
}

/// Parse the artifact. The grammar is fixed (see the artifact's own
/// header): lines are `[[pair]]`, `state = "…"`, `input = "…"`,
/// `to = "…"`, `effects = ["…", …]` (possibly empty/multi-line), or
/// `reject = "…"`. Anything else must be blank or a `#` comment —
/// an unknown line is a parse failure, never skipped (D-3 posture:
/// the reader is as strict as the writer).
fn parse_artifact(text: &str) -> Vec<PairExpectation> {
    let mut out = Vec::new();
    let mut state: Option<SessionState> = None;
    let mut input: Option<SessionInput> = None;
    let mut to: Option<SessionState> = None;
    let mut reject: Option<RejectionReason> = None;
    let mut effects: Vec<Effect> = Vec::new();
    let mut in_effects = false;
    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if in_effects {
            // Inside an effects array: consume items until `]`.
            let body = line.strip_suffix(']').unwrap_or(line);
            for item in body.split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                effects.push(parse_effect(unquote(item), number + 1));
            }
            if line.ends_with(']') {
                in_effects = false;
            }
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[pair]]" {
            if let (Some(state), Some(input)) = (state, input) {
                out.push(finish(state, input, to, reject.take(), &effects));
            }
            state = None;
            input = None;
            to = None;
            reject = None;
            effects.clear();
            continue;
        }
        let (key, value) = line
            .split_once(" = ")
            .unwrap_or_else(|| unreachable!("line {}: not `key = value`: {line}", number + 1));
        match key {
            "state" => state = Some(parse_state(unquote(value), number + 1)),
            "input" => input = Some(parse_input(unquote(value), number + 1)),
            "to" => to = Some(parse_state(unquote(value), number + 1)),
            "reject" => reject = Some(parse_reason(unquote(value), number + 1)),
            "effects" => {
                let inner = value
                    .strip_prefix('[')
                    .and_then(|rest| rest.strip_suffix(']'));
                match inner {
                    // Single-line array (the artifact's only shape).
                    Some(items) => {
                        for item in items.split(',') {
                            let item = item.trim();
                            if !item.is_empty() {
                                effects.push(parse_effect(unquote(item), number + 1));
                            }
                        }
                    }
                    None => in_effects = true,
                }
            }
            other => unreachable!("line {}: unknown key {other}", number + 1),
        }
    }
    if let (Some(state), Some(input)) = (state, input) {
        out.push(finish(state, input, to, reject, &effects));
    }
    out
}

fn finish(
    state: SessionState,
    input: SessionInput,
    to: Option<SessionState>,
    reject: Option<RejectionReason>,
    effects: &[Effect],
) -> PairExpectation {
    let answer = match (to, reject) {
        (Some(to), None) => ModelAnswer::Row {
            to,
            effects: leak_effects(effects),
        },
        (None, Some(reason)) => ModelAnswer::Reject(reason),
        (to, reject) => unreachable!(
            "{state} + {input}: exactly one of to/reject required (to={to:?}, reject={reject:?})"
        ),
    };
    PairExpectation { state, input, answer }
}

/// The artifact's `effects` are `&'static` in [`ModelAnswer`] (the
/// oracle spells them as statics); the parsed Vec needs a matching
/// 'static lifetime for comparison. Leaking a handful of 0-4 element
/// test-side slices is bounded by the 96-entry artifact (a test-only
/// convenience — at most a few KiB over the process life).
fn leak_effects(effects: &[Effect]) -> &'static [Effect] {
    Box::leak(effects.to_vec().into_boxed_slice())
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_else(|| unreachable!("not a quoted string: {value}"))
}

fn parse_state(name: &str, line: usize) -> SessionState {
    match name {
        "idle" => SessionState::Idle,
        "opening" => SessionState::Opening,
        "buffering" => SessionState::Buffering,
        "playing" => SessionState::Playing,
        "paused" => SessionState::Paused,
        "stopped" => SessionState::Stopped,
        "ended" => SessionState::Ended,
        "error" => SessionState::Error,
        other => unreachable!("line {line}: unknown state {other}"),
    }
}

fn parse_input(name: &str, line: usize) -> SessionInput {
    match name {
        "open" => SessionInput::Open,
        "play" => SessionInput::Play,
        "pause" => SessionInput::Pause,
        "stop" => SessionInput::Stop,
        "close" => SessionInput::Close,
        "seek" => SessionInput::Seek,
        "open-ready" => SessionInput::OpenReady,
        "primed" => SessionInput::Primed,
        "underrun" => SessionInput::Underrun,
        "eof-reached" => SessionInput::EofReached,
        "drained" => SessionInput::Drained,
        "failed" => SessionInput::Failed,
        other => unreachable!("line {line}: unknown input {other}"),
    }
}

fn parse_effect(name: &str, line: usize) -> Effect {
    match name {
        "emit-tracks" => Effect::EmitTracks,
        "emit-buffering-enter" => Effect::EmitBufferingEnter,
        "emit-buffering-exit" => Effect::EmitBufferingExit,
        "emit-end-of-media" => Effect::EmitEndOfMedia,
        "report-fatal" => Effect::ReportFatal,
        "bump-pipeline" => Effect::BumpPipeline,
        "destroy-session" => Effect::DestroySession,
        "teardown-pipeline" => Effect::TeardownPipeline,
        "seek-pipeline" => Effect::SeekPipeline,
        "freeze-clock" => Effect::FreezeClock,
        "resume-clock" => Effect::ResumeClock,
        "stop-clock" => Effect::StopClock,
        other => unreachable!("line {line}: unknown effect {other}"),
    }
}

fn parse_reason(name: &str, line: usize) -> RejectionReason {
    match name {
        "no-session" => RejectionReason::NoSession,
        "not-ready" => RejectionReason::NotReady,
        "settle-first" => RejectionReason::SettleFirst,
        "already-settled" => RejectionReason::AlreadySettled,
        "restart-needs-seek" => RejectionReason::RestartNeedsSeek,
        "source-replacement-phase3" => RejectionReason::SourceReplacementPhase3,
        "signal-not-raised-here" => RejectionReason::SignalNotRaisedHere,
        "ended-needs-wire-edge" => RejectionReason::EndedNeedsWireEdge,
        other => unreachable!("line {line}: unknown reason {other}"),
    }
}

fn artifact() -> Vec<PairExpectation> {
    parse_artifact(include_str!("model/expected_table.toml"))
}

// ---------------------------------------------------------------------------
// The three-way proof
// ---------------------------------------------------------------------------

/// The artifact is complete over the vocabulary: exactly 96 entries,
/// every `(state, input)` pair exactly once — the completeness half
/// of the model-based AC (the generalization of `table.rs`'s count
/// pins to the review surface).
#[test]
fn the_artifact_covers_every_pair_exactly_once() {
    let table = artifact();
    assert_eq!(table.len(), 96, "8 states x 12 inputs");
    let mut seen = std::collections::HashSet::new();
    for entry in &table {
        assert!(
            seen.insert((entry.state, entry.input)),
            "duplicate artifact entry: {} + {}",
            entry.state,
            entry.input
        );
    }
    for state in SessionState::ALL {
        for input in SessionInput::ALL {
            assert!(
                seen.contains(&(state, input)),
                "artifact is missing {} + {}",
                state,
                input
            );
        }
    }
}

/// THE three-way agreement: `apply_input` ≡ oracle ≡ artifact, per
/// pair, on target + effects + typed reason. Any disagreement names
/// the pair and all three opinions.
#[test]
fn table_oracle_and_artifact_agree_on_every_pair() {
    let table = artifact();
    let mut rows = 0usize;
    let mut rejections = 0usize;
    for entry in &table {
        let table_opinion = match apply_input(entry.state, entry.input) {
            Ok(row) => {
                rows += 1;
                ModelAnswer::Row {
                    to: row.to,
                    effects: row.effects,
                }
            }
            Err(refused) => {
                rejections += 1;
                ModelAnswer::Reject(refused.reason)
            }
        };
        let oracle_opinion = model_answer(entry.state, entry.input);
        assert_eq!(
            table_opinion, oracle_opinion,
            "{} + {}: engine table vs oracle",
            entry.state, entry.input
        );
        assert_eq!(
            oracle_opinion, entry.answer,
            "{} + {}: oracle vs committed artifact",
            entry.state, entry.input
        );
    }
    assert_eq!(rows, 30, "the pinned completed-machine row count");
    assert_eq!(rejections, 66, "66 typed rejections");
    assert_eq!(TRANSITIONS.len(), 30, "the data table carries the rows");
}

/// The artifact's row set has no duplicates and is exactly the
/// engine's row set (the same rows, not merely the same count —
/// belt-and-braces for the agreement test above, which compares per
/// pair over all 96 anyway).
#[test]
fn the_artifact_row_set_is_the_engine_row_set() {
    let mut artifact_rows = artifact()
        .into_iter()
        .filter_map(|entry| match entry.answer {
            ModelAnswer::Row { to, effects } => Some((entry.state, entry.input, to, effects)),
            ModelAnswer::Reject(_) => None,
        })
        .collect::<Vec<_>>();
    artifact_rows.sort_by_key(|(from, input, to, _)| {
        (
            SessionState::ALL.iter().position(|s| *s == *from).must(),
            SessionInput::ALL.iter().position(|i| *i == *input).must(),
            SessionState::ALL.iter().position(|s| *s == *to).must(),
        )
    });
    let mut engine_rows = TRANSITIONS
        .iter()
        .map(|row| (row.from, row.input, row.to, row.effects))
        .collect::<Vec<_>>();
    engine_rows.sort_by_key(|(from, input, to, _)| {
        (
            SessionState::ALL.iter().position(|s| *s == *from).must(),
            SessionInput::ALL.iter().position(|i| *i == *input).must(),
            SessionState::ALL.iter().position(|s| *s == *to).must(),
        )
    });
    assert_eq!(artifact_rows, engine_rows);
}

/// Every artifact row is wire-conformant (doc 05 §3.3), and the
/// artifact's edge projection is exactly the wire's 21-edge set —
/// the artifact-side twin of `table.rs`'s projection proof (a second
/// author must reach the same edge set).
#[test]
fn the_artifact_edges_are_exactly_the_wire_edge_set() {
    use playback_engine::state::SessionState as S;
    let index = |state: SessionState| SessionState::ALL.iter().position(|s| *s == state).must();
    let mut artifact_edges: Vec<(SessionState, SessionState)> = artifact()
        .into_iter()
        .filter_map(|entry| match entry.answer {
            ModelAnswer::Row { to, .. } if to != entry.state => Some((entry.state, to)),
            _ => None,
        })
        .collect();
    artifact_edges.sort_by_key(|(from, to)| (index(*from), index(*to)));
    artifact_edges.dedup();
    let mut wire_edges: Vec<(SessionState, SessionState)> = Vec::new();
    for from in SessionState::ALL {
        for to in SessionState::ALL {
            if from != to
                && from
                    .as_wire()
                    .can_transition_to(to.as_wire())
            {
                wire_edges.push((from, to));
            }
        }
    }
    wire_edges.sort_by_key(|(from, to)| (index(*from), index(*to)));
    assert_eq!(artifact_edges, wire_edges);
    assert_eq!(artifact_edges.len(), 21, "the doc 05 §3.3 edge count");
    // The three 3.C1 completions are present as edges (they were the
    // deliberately-absent debt this card paid).
    for edge in [
        (S::Opening, S::Stopped),
        (S::Buffering, S::Stopped),
        (S::Buffering, S::Paused),
    ] {
        assert!(
            artifact_edges.contains(&edge),
            "the 3.C1 completion edge {edge:?} is missing"
        );
    }
}

/// The artifact's rejection reasons agree with `rejection_reason`
/// directly (the per-pair test goes through `apply_input`'s Err; this
/// pins the total function the module docs promise).
#[test]
fn the_artifact_reasons_match_the_total_rejection_function() {
    for entry in artifact() {
        if let ModelAnswer::Reject(reason) = entry.answer {
            assert_eq!(
                playback_engine::state::rejection_reason(entry.state, entry.input),
                reason,
                "{} + {}: rejection_reason vs artifact",
                entry.state,
                entry.input
            );
        }
    }
}
