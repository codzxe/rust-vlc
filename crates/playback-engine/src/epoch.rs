//! Two-level cancellation generations (doc 03 §5) — session epochs and
//! pipeline generations, plus the shared [`CancelToken`].
//!
//! # The two levels (doc 03 §5, `[PROPOSAL]` in the doc — implemented
//! here as specified)
//!
//! - **Epoch** (session generation, [`SessionEpoch`]): minted on
//!   `Open` (and on source replacement once card 3.A2 wires it).
//!   Everything owned by an epoch — demuxer, queues, decoders, device
//!   session — dies with it (invariant S4). On the wire, the epoch IS
//!   the session identity: `commands::SessionId` (doc 05 §4.2's
//!   explicit mapping "SessionId ↔ 03 session epoch").
//! - **Pipeline generation** ([`PipelineGeneration`]): a sub-counter
//!   that **resets each epoch** and is minted on the gen-bumping
//!   events — in the minimal machine, `stop` (final) and `close`
//!   (terminal); seek/track-switch joins with Phase 3. On the wire
//!   this is doc 05 §4.2's `gen` ("starting at 0 at open").
//!
//! Comparisons are pair-wise: *different epoch OR different pipeline
//! ⇒ stale* ([`GenerationPair`] equality). Because a pipeline
//! generation resets per epoch, equality of the pair is the only
//! sound liveness test — a bare counter would call epoch 2's
//! generation 0 "current" for epoch 1.
//!
//! # Minting and publishing
//!
//! Only the session thread mints (doc 03 §5 "Minting"): the
//! [`TokenMinter`] lives behind `&mut` in the session, and each mint
//! atomically publishes the new pair to the [`CancelToken`] — an
//! `Arc`-shared snapshot (epoch, pipeline, interrupt flag) that
//! pipeline stages hold. Critical sections copy or write three words;
//! the lock is never held across anything else (the media-clock
//! precedent).
//!
//! # Queue generations
//!
//! The packet queues' [`GenerationId`](media_types::GenerationId)
//! space is the **pipeline generation** of the current epoch
//! ([`PipelineGeneration::as_queue_generation`]): queues are
//! constructed fresh per epoch (S4 — no cross-epoch queue reuse), and
//! within an epoch every mint flushes the queues with the new value,
//! which is exactly the boundary at which packet-queue drops stale
//! pushes (doc 03 §5 checking point 3/4). Cross-epoch collisions in
//! the queue space are therefore unobservable by construction: an
//! old-epoch producer only ever holds old-epoch queue handles.

use std::sync::{Arc, Mutex};

use commands::{Generation, SessionId};
use media_types::{EngineError, GenerationId};

/// The published token state (doc 03 §5: "epoch u64, pipeline u64,
/// plus an interrupt flag").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenState {
    epoch: u64,
    pipeline: u64,
    interrupted: bool,
}

/// The shared cancellation snapshot handed to pipeline stages (doc 03
/// §5). Cheap to clone (`Arc`); read-only for holders — only the
/// session's [`TokenMinter`] writes.
#[derive(Debug, Clone)]
pub struct CancelToken {
    state: Arc<Mutex<TokenState>>,
}

impl CancelToken {
    /// The currently published pair.
    #[must_use]
    pub fn snapshot(&self) -> GenerationPair {
        self.pair()
    }

    /// Whether `pair` is the live generation (same epoch AND same
    /// pipeline, and not interrupted).
    #[must_use]
    pub fn is_current(&self, pair: &GenerationPair) -> bool {
        !self.is_stale(pair)
    }

    /// Whether `pair` is stale: different epoch, different pipeline
    /// generation, or the token was interrupted (teardown).
    ///
    /// This is the doc 03 §5 comparison — "different epoch OR
    /// different pipeline ⇒ stale" — plus the interrupt flag set by
    /// teardown, which marks even the current pair stale (no new work
    /// under it may proceed).
    #[must_use]
    pub fn is_stale(&self, pair: &GenerationPair) -> bool {
        let state = self.lock();
        state.interrupted || state.epoch != pair.epoch.0 || state.pipeline != pair.pipeline.0
    }

    /// Whether teardown interrupted this token (doc 03 §5 teardown
    /// step 2: "cancel token interrupt" — checked by every stage
    /// between chunks).
    #[must_use]
    pub fn interrupted(&self) -> bool {
        self.lock().interrupted
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, TokenState> {
        // Poisoning cannot originate here: critical sections are
        // three-word copies (the packet-queue `unpoison` precedent).
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn pair(&self) -> GenerationPair {
        let state = self.lock();
        GenerationPair {
            epoch: SessionEpoch(state.epoch),
            pipeline: PipelineGeneration(state.pipeline),
        }
    }
}

/// The session-thread mint authority (doc 03 §5: "only the session
/// thread mints"). Held by value behind `&mut` in the session; never
/// cloned to stages.
#[derive(Debug)]
pub struct TokenMinter {
    token: CancelToken,
    epoch: SessionEpoch,
    pipeline: u64,
}

impl TokenMinter {
    /// A minter for a freshly opened session: `epoch` was minted by
    /// the engine, the pipeline generation starts at 0 (doc 05 §4.2:
    /// "starting at 0 at open"). Publishing the initial pair is the
    /// atomic hand-off stages observe.
    #[must_use]
    pub fn new(epoch: SessionEpoch) -> TokenMinter {
        let token = CancelToken {
            state: Arc::new(Mutex::new(TokenState {
                epoch: epoch.0,
                pipeline: PipelineGeneration::FIRST.0,
                interrupted: false,
            })),
        };
        TokenMinter {
            token,
            epoch,
            pipeline: PipelineGeneration::FIRST.0,
        }
    }

    /// The read-side token stages hold.
    #[must_use]
    pub fn token(&self) -> CancelToken {
        self.token.clone()
    }

    /// The live pair.
    #[must_use]
    pub fn pair(&self) -> GenerationPair {
        GenerationPair {
            epoch: self.epoch,
            pipeline: PipelineGeneration(self.pipeline),
        }
    }

    /// Mint the next pipeline generation and publish it (doc 03 §5:
    /// minted on seek / track switch / final stop / close — the
    /// minimal machine mints on `stop` and `close` only).
    ///
    /// # Errors
    /// [`EngineError`] (stage `Control`, kind `ResourceLimit`) when
    /// the u64 pipeline counter is exhausted — the L0
    /// [`GenerationId`] vocabulary's own limit.
    pub fn mint_pipeline(&mut self) -> Result<PipelineGeneration, EngineError> {
        let next = self
            .pipeline
            .checked_add(1)
            .ok_or_else(generation_exhausted)?;
        self.pipeline = next;
        self.publish();
        Ok(PipelineGeneration(next))
    }

    /// Interrupt the token (doc 03 §5 teardown step 2). Idempotent;
    /// every subsequent staleness check answers stale.
    pub fn interrupt(&mut self) {
        {
            let mut state = self.token.lock();
            state.interrupted = true;
        }
    }

    fn publish(&self) {
        let mut state = self.token.lock();
        state.epoch = self.epoch.0;
        state.pipeline = self.pipeline;
        state.interrupted = false;
    }
}

fn generation_exhausted() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Control,
        media_types::ErrorKind::ResourceLimit {
            limit: media_types::error::LimitKind::SessionGenerations,
            budget: u64::MAX,
            attempted: u64::MAX,
        },
        media_types::error::ErrorContext::new("generation.exhausted"),
    )
}

/// A session epoch (doc 03 §5): minted by the engine on open; the
/// wire session identity (`commands::SessionId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionEpoch(pub u64);

impl SessionEpoch {
    /// The first epoch an [`crate::Engine`] mints. Request ids start
    /// at 1 in doc 05 §4.1; epochs follow the same convention so an
    /// epoch is never zero `[PROPOSAL: doc 05 does not pin the session
    /// id's start; 1 keeps "0 means none" available for Option
    /// encodings]`.
    pub const FIRST: SessionEpoch = SessionEpoch(1);

    /// The next epoch in the engine's monotonic sequence.
    ///
    /// # Errors
    /// [`EngineError`] (stage `Control`) when the counter is
    /// exhausted.
    pub fn next(self) -> Result<SessionEpoch, EngineError> {
        self.0
            .checked_add(1)
            .map(SessionEpoch)
            .ok_or_else(generation_exhausted)
    }

    /// The wire session id (doc 05 §4.2 mapping).
    #[must_use]
    pub fn as_session_id(self) -> SessionId {
        SessionId(self.0)
    }
}

/// A pipeline generation (doc 03 §5): resets each epoch, minted on
/// gen-bumping events. The wire `gen` (doc 05 §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PipelineGeneration(pub u64);

impl PipelineGeneration {
    /// The generation a session starts at (doc 05 §4.2: "starting at
    /// 0 at open").
    pub const FIRST: PipelineGeneration = PipelineGeneration(0);

    /// The wire generation (doc 05 §4.2 mapping).
    #[must_use]
    pub fn as_wire(self) -> Generation {
        Generation(self.0)
    }

    /// The queue/decoder generation stamp for this pipeline
    /// generation of the current epoch (see the module docs: queues
    /// are per-epoch, so the pipeline counter alone is the space).
    #[must_use]
    pub fn as_queue_generation(self) -> GenerationId {
        GenerationId::new(self.0)
    }
}

/// The two-level identity compared for staleness (doc 03 §5:
/// "Epoch = (epoch, pipeline) pair compared for equality;
/// comparisons are 'different epoch OR different pipeline ⇒
/// stale'").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationPair {
    /// The owning session epoch.
    pub epoch: SessionEpoch,
    /// The pipeline generation within the epoch.
    pub pipeline: PipelineGeneration,
}

impl GenerationPair {
    /// The wire session id of the pair's epoch.
    #[must_use]
    pub fn session_id(self) -> SessionId {
        self.epoch.as_session_id()
    }

    /// The wire `gen` of the pair's pipeline generation.
    #[must_use]
    pub fn wire_generation(self) -> Generation {
        self.pipeline.as_wire()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minter_publishes_and_stages_observe() {
        let mut minter = TokenMinter::new(SessionEpoch::FIRST);
        let token = minter.token();
        let pair = minter.pair();
        assert_eq!(pair.pipeline, PipelineGeneration::FIRST);
        assert!(token.is_current(&pair));

        let minted = match minter.mint_pipeline() {
            Ok(g) => g,
            Err(e) => unreachable!("mint must succeed: {e}"),
        };
        assert_eq!(minted, PipelineGeneration(1));
        // The stage-held token observes the new pair; the old one is
        // stale (doc 03 §5's comparison).
        assert!(token.is_stale(&pair));
        assert!(token.is_current(&minter.pair()));
    }

    #[test]
    fn interrupt_marks_even_the_current_pair_stale() {
        let mut minter = TokenMinter::new(SessionEpoch::FIRST);
        let token = minter.token();
        let pair = minter.pair();
        assert!(token.is_current(&pair));
        minter.interrupt();
        assert!(token.interrupted());
        assert!(token.is_stale(&pair), "teardown invalidates the pair");
    }

    #[test]
    fn cross_epoch_equal_pipeline_counters_are_stale() {
        // Epoch 2's pipeline 0 is NOT epoch 1's pipeline 0: the pair
        // is the identity (the doc 03 §5 rule that motivates
        // per-epoch fresh queues).
        let mut minter = TokenMinter::new(SessionEpoch::FIRST);
        let _ = match minter.mint_pipeline() {
            Ok(g) => g,
            Err(e) => unreachable!("{e}"),
        };
        let second = TokenMinter::new(match SessionEpoch::FIRST.next() {
            Ok(e) => e,
            Err(e) => unreachable!("{e}"),
        });
        let second_pair = second.pair();
        assert_eq!(second_pair.pipeline, PipelineGeneration::FIRST);
        assert!(minter.token().is_stale(&second_pair));
        assert!(second.token().is_stale(&minter.pair()));
    }

    #[test]
    fn wire_mappings_are_the_raw_counters() {
        assert_eq!(SessionEpoch(7).as_session_id(), SessionId(7));
        assert_eq!(PipelineGeneration(3).as_wire(), Generation(3));
        assert_eq!(
            PipelineGeneration(5).as_queue_generation(),
            GenerationId::new(5)
        );
        let pair = GenerationPair {
            epoch: SessionEpoch(2),
            pipeline: PipelineGeneration(4),
        };
        assert_eq!(pair.session_id(), SessionId(2));
        assert_eq!(pair.wire_generation(), Generation(4));
    }
}
