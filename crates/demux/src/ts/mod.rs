//! The MPEG transport-stream demux layers (card 5.I1).
//!
//! **I1b seam state (this tree):** [`pes`] — the PES layer, the
//! 90 kHz timing model, and the minimal PID→PES assembly seam —
//! landed here first. The parallel PSI half (I1a: PAT/PMT/SDT and
//! the transport-packet walk) and the full `Demuxer` implementation
//! build on top of it. Merge rule for the Lead's I1 unification:
//! append `pub mod` lines, never reorder the existing ones.

pub mod pes;
