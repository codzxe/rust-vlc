//! The positional-source surface (card 2.A1): position, length, typed
//! seeks, and the raw chunk primitive the bounded reader is generic
//! over.
//!
//! [`Seekable`] is the card's seek contract: a value and its absence are
//! different types (`length()` is `Option`, MP §8), positions are
//! absolute, and seek failures are typed
//! [`EngineError`](media_types::EngineError)s — never sentinel returns.
//! [`RawRead`] is the deliberately small read primitive: chunked reads
//! into caller memory, `Ok(0)` only at the underlying end of stream.
