//! Local file sources (card 2.A1): the `plugin_api::Access`
//! implementation doc 01 §2.4 assigns to this crate, plus the raw
//! [`FileSource`] it opens and the [`FileStream`] adapter that satisfies
//! the plugin-api `SourceStream` contract over a
//! [`BufferedReader`](crate::BufferedReader).
//!
//! Regular files only: directories, FIFOs and device nodes are typed
//! rejections at open time — a device like `/dev/zero` would be an
//! infinite read (MP §12 bounds memory; "no unbounded read-ahead",
//! doc 01 §2.4).
