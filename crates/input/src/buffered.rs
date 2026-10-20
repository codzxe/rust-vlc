//! The bounded buffered reader (card 2.A1, doc 03 §4.1 + MP §12).
//!
//! [`BufferedReader`] wraps any [`RawRead`] + [`Seekable`](crate::Seekable)
//! source with a **fixed-capacity** buffer allocated once at
//! construction and never grown. Reads larger than the capacity are
//! streamed through it in capacity-sized chunks; a single read request
//! above [`MAX_READ_BYTES`] is refused with a typed `ResourceLimit`
//! error instead of an allocation. A file that shrinks below its
//! open-time length surfaces `Corrupt::Truncated`, never a silent EOF.
//!
//! # Memory accounting (the card's "no unbounded buffering" proof)
//!
//! Steady-state memory = the fixed buffer (≤ [`MAX_BUFFER_BYTES`]) + the
//! caller-visible result of one bounded read (≤ [`MAX_READ_BYTES`]).
//! The [`contract`](crate::contract) checks instrument the underlying
//! source so tests can prove no read ever asks it for more than the
//! buffer's capacity.
