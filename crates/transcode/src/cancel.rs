//! The cooperative cancellation token (card 7.E1): one atomic flag,
//! cloned freely, fired once.
//!
//! Cancellation here is **cooperative and prompt**: no stage is
//! interrupted mid-call (the codec-api contract forbids yanking work
//! out from under a decoder); instead every loop iteration, every
//! bounded park and every queue hand-off checks this token, and every
//! waiter that must wake on cancel is woken by the job that fires it.
//! The pump/transform loops in [`crate::runner`] check the token:
//!
//! * before each demux event,
//! * between each decoded packet,
//! * inside every bounded queue park (the queue holds a clone and its
//!   `interrupt` wakes all waiters).
//!
//! A cancelled job still **finalizes its sink** (best effort) so the
//! partial output is structurally valid — sizes tell the truth about
//! the bytes that made it; the outcome distinguishes `Cancelled` from
//! `Completed` so no caller mistakes a prefix for the whole.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A shareable, one-way stop flag.
///
/// `Clone` shares the same underlying flag (the `Arc<AtomicBool>`
/// shape the streaming crate's event hubs use for their drop
/// counters). Firing is idempotent; there is no un-fire — a job is a
/// one-shot pipeline, not a session.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    /// A token that has not fired.
    #[must_use]
    pub fn new() -> CancelToken {
        CancelToken {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Fire the stop request. Returns whether it had already fired.
    ///
    /// Firing does **not** by itself wake parked queue waiters — the
    /// owning job calls [`crate::PacketStageQueue::interrupt`] for
    /// that; a bare token (no queue) is a plain flag.
    pub fn cancel(&self) -> bool {
        self.flag.swap(true, Ordering::SeqCst)
    }

    /// Whether the stop request has fired.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_the_flag_and_firing_is_idempotent() {
        let token = CancelToken::new();
        let twin = token.clone();
        assert!(!token.is_cancelled());
        assert!(!twin.is_cancelled());
        assert!(!token.cancel(), "first fire reports not-previously-fired");
        assert!(token.is_cancelled());
        assert!(twin.is_cancelled(), "clones observe the same flag");
        assert!(twin.cancel(), "second fire reports previously-fired");
        assert!(token.is_cancelled());
    }

    #[test]
    fn independent_tokens_do_not_interfere() {
        let a = CancelToken::new();
        let b = CancelToken::new();
        a.cancel();
        assert!(a.is_cancelled());
        assert!(!b.is_cancelled());
    }
}
