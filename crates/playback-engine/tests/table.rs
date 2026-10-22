//! The exhaustive transition-table suite (card 2.C2 AC): every
//! `(state, input)` pair classified legal/illegal, every legal row
//! proven wire-conformant, the pipeline driver guard proven closed,
//! and every row walked from `Idle` through legal inputs only.
//!
//! "No invalid transition reachable" is the sum of three proofs, all
//! in this file:
//!
//! 1. [`apply_input`] agrees with [`TRANSITIONS`] exactly (104 pairs:
//!    29 rows, 75 typed rejections) — the table is the only truth.
//! 2. every row's `(from, to)` is legal for the wire conformance list
//!    (`PlaybackState::can_transition_to`, doc 05 §3.3) — no
//!    engine-side transition can surface as a conformance failure —
//!    and, pinched harder, the engine's edge projection is exactly
//!    the wire's 18-edge set.
//! 3. [`translate_pipeline_signal`] only ever yields table-legal
//!    inputs — the pipeline side cannot reach an illegal transition
//!    either.

use commands::{CommandId, PlaybackState};
use playback_engine::state::{
    PipelineSignal, RejectionReason, SessionInput, SessionState, TRANSITIONS, Transition,
    apply_input, rejection_reason, translate_pipeline_signal,
};
use playback_engine::{SessionCommandKind, transition_table};

// ---------------------------------------------------------------------------
// Helpers (pure-model)
// ---------------------------------------------------------------------------

fn table_row(state: SessionState, input: SessionInput) -> Option<&'static Transition> {
    TRANSITIONS
        .iter()
        .find(|row| row.from == state && row.input == input)
}

/// BFS over table rows from `start`; answers a parent map and, for
/// each state, one witness input that reaches it.
fn reachable_from(start: SessionState) -> Vec<(SessionState, Option<SessionInput>)> {
    let mut parents: Vec<(SessionState, Option<SessionInput>)> =
        SessionState::ALL.iter().map(|s| (*s, None)).collect();
    let mut queue = std::collections::VecDeque::new();
    let mut seen = vec![start];
    queue.push_back(start);
    while let Some(state) = queue.pop_front() {
        for row in TRANSITIONS {
            if row.from == state && !seen.contains(&row.to) {
                seen.push(row.to);
                let entry = parents
                    .iter_mut()
                    .find(|(s, _)| *s == row.to)
                    .unwrap_or_else(|| unreachable!("ALL covers every state"));
                entry.1 = Some(row.input);
                queue.push_back(row.to);
            }
        }
    }
    parents
}

/// A legal input path `Idle -> target`, walked backwards through the
/// BFS parent map (empty when `target == Idle`).
fn path_to(target: SessionState) -> Vec<SessionInput> {
    let parents = reachable_from(SessionState::Idle);
    let mut path = Vec::new();
    let mut cursor = target;
    while cursor != SessionState::Idle {
        let (_, step) = parents
            .iter()
            .find(|(s, _)| *s == cursor)
            .unwrap_or_else(|| unreachable!("ALL covers every state"));
        match step {
            Some(input) => {
                // The parent map keys edges by their TARGET; walk back
                // to the row's source.
                let row = TRANSITIONS
                    .iter()
                    .find(|row| row.input == *input && row.to == cursor)
                    .unwrap_or_else(|| unreachable!("parent edges are table rows"));
                path.push(*input);
                cursor = row.from;
            }
            None => unreachable!("{cursor} has no path from Idle"),
        }
    }
    path.reverse();
    path
}

// ---------------------------------------------------------------------------
// 1. Every (state, input) pair is classified, and the classification
//    is exactly the table.
// ---------------------------------------------------------------------------

#[test]
fn every_state_input_pair_is_classified_exactly_by_the_table() {
    let mut legal = 0usize;
    let mut illegal = 0usize;
    for state in SessionState::ALL {
        for input in SessionInput::ALL {
            match apply_input(state, input) {
                Ok(row) => {
                    legal += 1;
                    assert_eq!(row.from, state, "{state} + {input}");
                    assert_eq!(row.input, input, "{state} + {input}");
                    // The row IS the table's row for the pair (same
                    // static reference would be stronger than needed;
                    // identity of fields is the contract).
                    let table = table_row(state, input)
                        .unwrap_or_else(|| unreachable!("Ok implies a table row"));
                    assert_eq!(row.to, table.to);
                    assert_eq!(row.effects, table.effects);
                }
                Err(rejection) => {
                    illegal += 1;
                    assert_eq!(rejection.from, state);
                    assert_eq!(rejection.input, input);
                    assert!(
                        table_row(state, input).is_none(),
                        "{state} + {input} is a table row but was rejected"
                    );
                }
            }
        }
    }
    assert_eq!(legal, TRANSITIONS.len(), "legal == table rows");
    assert_eq!(illegal, 104 - TRANSITIONS.len(), "8 states x 13 inputs");
    assert_eq!(legal, 29, "the pinned minimal-machine row count");
    assert_eq!(illegal, 75, "75 typed rejections");
}

#[test]
fn the_table_has_no_duplicate_rows() {
    for (index, row) in TRANSITIONS.iter().enumerate() {
        for (other, later) in TRANSITIONS.iter().enumerate().skip(index + 1) {
            assert!(
                !(row.from == later.from && row.input == later.input),
                "duplicate row {} / {}: {} + {}",
                index,
                other,
                row.from,
                row.input
            );
        }
    }
}

#[test]
fn transition_table_accessor_is_the_table() {
    assert_eq!(transition_table().len(), TRANSITIONS.len());
    for (a, b) in transition_table().iter().zip(TRANSITIONS.iter()) {
        assert_eq!(a, b);
    }
}

// ---------------------------------------------------------------------------
// 2. Every legal row is wire-conformant (doc 05 §3.3), and the
//    engine's edge projection is exactly the wire's.
// ---------------------------------------------------------------------------

#[test]
fn every_legal_row_is_wire_conformant() {
    for row in TRANSITIONS {
        assert!(
            row.from.as_wire().can_transition_to(row.to.as_wire()),
            "engine row {} --{}--> {} is not wire-legal",
            row.from,
            row.input,
            row.to
        );
    }
}

fn state_index(state: SessionState) -> usize {
    SessionState::ALL
        .iter()
        .position(|s| *s == state)
        .unwrap_or_else(|| unreachable!("ALL covers every state"))
}

#[test]
fn the_engine_edge_projection_is_exactly_the_wire_edge_set() {
    // The distinct (from, to) pairs the engine can produce…
    let mut engine_edges: Vec<(SessionState, SessionState)> = TRANSITIONS
        .iter()
        .filter(|row| row.from != row.to)
        .map(|row| (row.from, row.to))
        .collect();
    engine_edges.sort_by_key(|(from, to)| (state_index(*from), state_index(*to)));
    engine_edges.dedup();
    // …and the distinct edges the wire conformance list allows.
    let mut wire_edges: Vec<(SessionState, SessionState)> = Vec::new();
    for from in SessionState::ALL {
        for to in SessionState::ALL {
            if from != to && from.as_wire().can_transition_to(to.as_wire()) {
                wire_edges.push((from, to));
            }
        }
    }
    wire_edges.sort_by_key(|(from, to)| (state_index(*from), state_index(*to)));
    assert_eq!(engine_edges, wire_edges);
    assert_eq!(engine_edges.len(), 18, "the doc 05 §3.3 edge count");
}

#[test]
fn self_transitions_are_the_idempotent_confirmations_and_seek_work_rows() {
    let self_rows: Vec<SessionInput> = TRANSITIONS
        .iter()
        .filter(|row| row.from == row.to)
        .map(|row| row.input)
        .collect();
    assert_eq!(
        self_rows,
        vec![
            SessionInput::Close, // Idle: idempotent close, no session
            SessionInput::Play,  // Buffering: play while priming (C8)
            SessionInput::Play,  // Playing: play while playing (C8)
            SessionInput::Seek,  // Paused: seek while paused (work-carrying)
            SessionInput::Seek,  // Buffering: seek while priming (work-carrying)
            SessionInput::Pause, // Paused: pause while paused (C8)
        ]
    );
}

// ---------------------------------------------------------------------------
// 3. Illegal pairs: typed rejections with total reasons.
// ---------------------------------------------------------------------------

#[test]
fn rejection_reason_agrees_with_apply_input_everywhere() {
    for state in SessionState::ALL {
        for input in SessionInput::ALL {
            if table_row(state, input).is_none() {
                let reason = rejection_reason(state, input);
                let rejection = match apply_input(state, input) {
                    Err(rejection) => rejection,
                    Ok(_) => unreachable!("{state} + {input} is legal"),
                };
                assert_eq!(rejection.reason, reason, "{state} + {input}");
            }
        }
    }
}

#[test]
fn the_interesting_rejections_carry_their_documented_reasons() {
    use RejectionReason as R;
    use SessionInput as I;
    use SessionState as S;
    let cases: &[(SessionState, SessionInput, RejectionReason)] = &[
        // No session work exists in Idle.
        (S::Idle, I::Play, R::NoSession),
        (S::Idle, I::Pause, R::NoSession),
        (S::Idle, I::Stop, R::NoSession),
        (S::Idle, I::Seek, R::NoSession),
        // Pipeline signals where no pipeline runs.
        (S::Idle, I::Primed, R::SignalNotRaisedHere),
        (S::Idle, I::Failed, R::SignalNotRaisedHere),
        (S::Ended, I::Drained, R::SignalNotRaisedHere),
        (S::Stopped, I::Underrun, R::SignalNotRaisedHere),
        (S::Error, I::OpenReady, R::SignalNotRaisedHere),
        // Opening is not ready for playback commands.
        (S::Opening, I::Play, R::NotReady),
        (S::Opening, I::Pause, R::NotReady),
        (S::Opening, I::Seek, R::NotReady),
        // The deliberately-absent v1 wire edges.
        (S::Opening, I::Stop, R::StopFirst),
        (S::Buffering, I::Stop, R::StopFirst),
        (S::Buffering, I::Close, R::StopFirst),
        (S::Playing, I::Close, R::StopFirst),
        (S::Paused, I::Close, R::StopFirst),
        // Pause during buffering has no v1 wire edge.
        (S::Buffering, I::Pause, R::NotReady),
        // Restart-from-ended is the Phase-3 seek flow.
        (S::Ended, I::Play, R::RestartNeedsSeek),
        // Seek-back-from-ended needs the missing wire edge (Lead flag).
        (S::Ended, I::Seek, R::EndedNeedsWireEdge),
        // Settled sessions are inert until close.
        (S::Ended, I::Pause, R::AlreadySettled),
        (S::Ended, I::Stop, R::AlreadySettled),
        (S::Stopped, I::Play, R::AlreadySettled),
        (S::Stopped, I::Pause, R::AlreadySettled),
        (S::Stopped, I::Stop, R::AlreadySettled),
        (S::Stopped, I::Seek, R::AlreadySettled),
        (S::Error, I::Play, R::AlreadySettled),
        (S::Error, I::Pause, R::AlreadySettled),
        (S::Error, I::Stop, R::AlreadySettled),
        (S::Error, I::Seek, R::AlreadySettled),
    ];
    for (state, input, reason) in cases {
        assert_eq!(
            rejection_reason(*state, *input),
            *reason,
            "{state} + {input}"
        );
    }
}

#[test]
fn open_on_a_live_session_is_source_replacement_everywhere() {
    // Every non-Idle open is the 3.A2 flow; the engine refuses it
    // honestly instead of silently replacing.
    for state in SessionState::ALL {
        if state == SessionState::Idle {
            continue;
        }
        assert_eq!(
            rejection_reason(state, SessionInput::Open),
            RejectionReason::SourceReplacementPhase3,
            "{state}"
        );
    }
}

#[test]
fn rejections_render_with_state_input_and_reason() {
    let rejection = match apply_input(SessionState::Ended, SessionInput::Play) {
        Err(rejection) => rejection,
        Ok(_) => unreachable!("play in ended is rejected"),
    };
    let text = rejection.to_string();
    assert!(text.contains("play"), "{text}");
    assert!(text.contains("ended"), "{text}");
    assert!(text.contains("seek"), "{text}");
}

// ---------------------------------------------------------------------------
// 4. The driver guard: the pipeline cannot reach an illegal
//    transition either.
// ---------------------------------------------------------------------------

#[test]
fn translate_never_yields_an_illegal_input() {
    for state in SessionState::ALL {
        for signal in [
            PipelineSignal::Fatal,
            PipelineSignal::Drained,
            PipelineSignal::EofReached,
            PipelineSignal::Primed,
            PipelineSignal::Starved,
        ] {
            if let Some(input) = translate_pipeline_signal(state, signal) {
                assert!(
                    apply_input(state, input).is_ok(),
                    "guard yielded illegal {input} for {state} + {signal:?}"
                );
            }
        }
    }
}

#[test]
fn translate_mappings_are_pinned() {
    use playback_engine::state::SessionInput as I;
    use playback_engine::state::SessionState as S;
    let cases: &[(SessionState, PipelineSignal, Option<SessionInput>)] = &[
        // Fatal is raised wherever a live pipeline can fail.
        (S::Buffering, PipelineSignal::Fatal, Some(I::Failed)),
        (S::Playing, PipelineSignal::Fatal, Some(I::Failed)),
        (S::Paused, PipelineSignal::Fatal, Some(I::Failed)),
        // Drain ends the media from the consuming states.
        (S::Playing, PipelineSignal::Drained, Some(I::Drained)),
        (S::Paused, PipelineSignal::Drained, Some(I::Drained)),
        // In Buffering the short-media start preempts the drain.
        (S::Buffering, PipelineSignal::Drained, None),
        // Short media: start with what exists.
        (
            S::Buffering,
            PipelineSignal::EofReached,
            Some(I::EofReached),
        ),
        // Priming completes the initial fill.
        (S::Buffering, PipelineSignal::Primed, Some(I::Primed)),
        // Rebuffer on a refillable underrun.
        (S::Playing, PipelineSignal::Starved, Some(I::Underrun)),
        // Absorbed facts (no session-level meaning): eof while
        // playing/paused, high water while consuming, low water while
        // buffering, and every signal where no pipeline exists.
        (S::Playing, PipelineSignal::EofReached, None),
        (S::Paused, PipelineSignal::EofReached, None),
        (S::Playing, PipelineSignal::Primed, None),
        (S::Paused, PipelineSignal::Primed, None),
        (S::Buffering, PipelineSignal::Starved, None),
        (S::Idle, PipelineSignal::Fatal, None),
        (S::Idle, PipelineSignal::Drained, None),
        (S::Opening, PipelineSignal::Primed, None),
        (S::Ended, PipelineSignal::Fatal, None),
        (S::Stopped, PipelineSignal::Starved, None),
        (S::Error, PipelineSignal::Drained, None),
    ];
    for (state, signal, want) in cases {
        assert_eq!(
            translate_pipeline_signal(*state, *signal),
            *want,
            "{state} + {signal:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Walks: every state reachable, every row walked from Idle.
// ---------------------------------------------------------------------------

#[test]
fn every_state_is_reachable_from_idle() {
    let parents = reachable_from(SessionState::Idle);
    for (state, step) in parents {
        assert!(
            step.is_some() || state == SessionState::Idle,
            "{state} unreachable from Idle"
        );
    }
}

#[test]
fn every_row_is_walked_from_idle_through_legal_inputs_only() {
    // For each of the 26 rows: walk a legal path to `row.from`, then
    // apply `row.input`, and confirm the machine lands on `row.to`.
    for (index, row) in TRANSITIONS.iter().enumerate() {
        let mut state = SessionState::Idle;
        for step in path_to(row.from) {
            let walked = match apply_input(state, step) {
                Ok(walked) => walked,
                Err(rejection) => {
                    unreachable!("path step {step} illegal at {state}: {rejection}")
                }
            };
            state = walked.to;
        }
        assert_eq!(state, row.from, "row {index}: path landed on {state}");
        let landed = match apply_input(state, row.input) {
            Ok(landed) => landed,
            Err(rejection) => unreachable!("row {index} rejected: {rejection}"),
        };
        assert_eq!(landed.to, row.to, "row {index}");
        assert_eq!(landed.effects, row.effects, "row {index}");
    }
}

#[test]
fn the_full_happy_path_and_exits_walk_cleanly() {
    // The nominal lives of a session, each walked input by input:
    // happy path to Ended, the stop exit, and the failure exit.
    use SessionInput as I;
    use SessionState as S;
    let happy = [
        (I::Open, S::Opening),
        (I::OpenReady, S::Buffering),
        (I::Primed, S::Playing),
        (I::Pause, S::Paused),
        (I::Play, S::Playing),
        (I::Drained, S::Ended),
        (I::Close, S::Idle),
    ];
    let stopped = [
        (I::Open, S::Opening),
        (I::OpenReady, S::Buffering),
        (I::EofReached, S::Playing), // short media start
        (I::Stop, S::Stopped),
        (I::Close, S::Idle),
    ];
    let failed = [
        (I::Open, S::Opening),
        (I::Failed, S::Error),
        (I::Close, S::Idle),
    ];
    for (label, walk) in [
        ("happy", &happy[..]),
        ("stopped", &stopped[..]),
        ("failed", &failed[..]),
    ] {
        let mut state = S::Idle;
        for (input, want) in walk {
            let row = match apply_input(state, *input) {
                Ok(row) => row,
                Err(rejection) => unreachable!("{label}: {rejection}"),
            };
            assert_eq!(row.to, *want, "{label}: {input}");
            state = row.to;
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Reconciliation with the wire catalog, as data (doc 05 §2/§4.2).
// ---------------------------------------------------------------------------

#[test]
fn session_commands_reconcile_with_the_wire_catalog() {
    use commands::catalog::spec::{CompletionClass, catalog_data};
    let cases = [
        (
            SessionCommandKind::Play,
            CommandId::PlaybackPlay,
            CompletionClass::A,
            false,
        ),
        (
            SessionCommandKind::Pause,
            CommandId::PlaybackPause,
            CompletionClass::A,
            false,
        ),
        (
            SessionCommandKind::Stop,
            CommandId::PlaybackStop,
            CompletionClass::A,
            true,
        ),
        (
            SessionCommandKind::Close,
            CommandId::SessionClose,
            CompletionClass::S,
            true,
        ),
        (
            SessionCommandKind::Seek,
            CommandId::PlaybackSeek,
            CompletionClass::A,
            true,
        ),
    ];
    for (kind, id, class, gen_bumping) in cases {
        assert_eq!(kind.as_command_id(), id);
        let spec = catalog_data()
            .iter()
            .find(|spec| spec.id == id)
            .unwrap_or_else(|| unreachable!("{id:?} missing from catalog_data"));
        assert_eq!(spec.class, class, "{id:?}");
        assert_eq!(spec.gen_bumping, gen_bumping, "{id:?}");
    }
}

#[test]
fn gen_bumping_commands_carry_the_bump_effect_and_vice_versa() {
    use SessionInput as I;
    use playback_engine::state::Effect;
    // The catalog says stop/close/seek bump the generation at accept
    // time; the table must encode the mint exactly there — and
    // nowhere else. The one exception is the `Idle + Close`
    // self-confirmation: no session exists, so there is nothing to
    // mint (the row documents the idempotent no-op, doc 05 §2.3).
    for row in TRANSITIONS {
        let bumps = row.effects.contains(&Effect::BumpPipeline);
        match row.input {
            I::Stop => assert!(bumps, "{} + {} must mint", row.from, row.input),
            I::Seek => assert!(bumps, "{} + {} must mint", row.from, row.input),
            I::Close if row.from != SessionState::Idle => {
                assert!(bumps, "{} + {} must mint", row.from, row.input)
            }
            _ => assert!(!bumps, "{} + {} must not mint", row.from, row.input),
        }
    }
    // And the minimal machine never mints on play/pause rows even
    // where the wire's class-A confirmation applies.
    let play_pause_rows: usize = TRANSITIONS
        .iter()
        .filter(|row| matches!(row.input, I::Play | I::Pause))
        .count();
    assert_eq!(play_pause_rows, 5, "3 play rows + 2 pause rows");
}

#[test]
fn effect_columns_match_the_doc_03_side_effects() {
    use SessionInput as I;
    use SessionState as S;
    use playback_engine::state::Effect;
    let has = |row: &Transition, effect: Effect| row.effects.contains(&effect);
    for row in TRANSITIONS {
        // Every entry into Error reports the fatal first.
        if row.to == S::Error {
            assert_eq!(row.effects.first(), Some(&Effect::ReportFatal), "{row:?}");
        }
        // Every entry into Error tears the pipeline down (doc 03
        // §4.6: "teardown as Stopped") — the Opening failure included.
        if row.to == S::Error {
            assert!(has(row, Effect::TeardownPipeline), "{row:?}");
        }
        // Buffering episodes bracket with progress events.
        if row.to == S::Buffering && row.from != S::Buffering {
            assert!(has(row, Effect::EmitBufferingEnter), "{row:?}");
        }
        if row.from == S::Buffering && row.to == S::Playing {
            assert!(has(row, Effect::EmitBufferingExit), "{row:?}");
            assert!(has(row, Effect::ResumeClock), "{row:?}");
        }
        // The initial track list precedes the first buffering.
        if row.from == S::Opening && row.to == S::Buffering {
            assert_eq!(row.effects.first(), Some(&Effect::EmitTracks), "{row:?}");
        }
        // Pause/underrun freeze; resume unfreezes.
        if (row.from == S::Playing && row.to == S::Paused)
            || (row.from == S::Playing && row.to == S::Buffering && row.input == I::Underrun)
        {
            assert!(has(row, Effect::FreezeClock), "{row:?}");
        }
        if row.to == S::Playing && row.from == S::Paused {
            assert!(has(row, Effect::ResumeClock), "{row:?}");
        }
        // The media ends with the clock stopped and the end event
        // before the state change.
        if row.to == S::Ended {
            assert!(has(row, Effect::StopClock), "{row:?}");
            assert!(has(row, Effect::EmitEndOfMedia), "{row:?}");
        }
        // Close destroys the session.
        if row.input == I::Close && row.to == S::Idle && row.from != S::Idle {
            assert!(has(row, Effect::DestroySession), "{row:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// 7. Vocabulary pins.
// ---------------------------------------------------------------------------

#[test]
fn the_state_and_input_vocabularies_are_unique_and_complete() {
    assert_eq!(SessionState::ALL.len(), 8);
    assert_eq!(SessionInput::ALL.len(), 12);
    for (index, state) in SessionState::ALL.iter().enumerate() {
        for other in SessionState::ALL.iter().skip(index + 1) {
            assert_ne!(state, other);
        }
    }
    for (index, input) in SessionInput::ALL.iter().enumerate() {
        for other in SessionInput::ALL.iter().skip(index + 1) {
            assert_ne!(input, other);
        }
    }
}

#[test]
fn engine_states_map_one_to_one_onto_the_wire_states() {
    let mut wire_seen: Vec<PlaybackState> = Vec::new();
    for state in SessionState::ALL {
        let wire = state.as_wire();
        assert!(!wire_seen.contains(&wire), "duplicate wire state {wire:?}");
        wire_seen.push(wire);
    }
    assert_eq!(wire_seen.len(), 8);
    // Display names are the wire names (doc 05 §3.3's quoted strings).
    let names = [
        "idle",
        "opening",
        "buffering",
        "playing",
        "paused",
        "stopped",
        "ended",
        "error",
    ];
    for (state, name) in SessionState::ALL.iter().zip(names) {
        assert_eq!(state.to_string(), name);
    }
    // Command inputs are exactly the six client-facing ones.
    let command_inputs: Vec<_> = SessionInput::ALL
        .iter()
        .filter(|input| input.is_command())
        .collect();
    assert_eq!(command_inputs.len(), 6);
}

#[test]
fn command_kinds_map_onto_table_inputs() {
    let cases = [
        (SessionCommandKind::Play, SessionInput::Play),
        (SessionCommandKind::Pause, SessionInput::Pause),
        (SessionCommandKind::Stop, SessionInput::Stop),
        (SessionCommandKind::Close, SessionInput::Close),
        (SessionCommandKind::Seek, SessionInput::Seek),
    ];
    for (kind, input) in cases {
        assert_eq!(kind.as_input(), input);
    }
}
