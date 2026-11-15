//! The tauri frame channel (w31.1, lane `luf1-channel`): the
//! webview-registered channel id rehydrated into a real
//! `tauri::ipc::Channel` bound to the main webview, so the pacer's
//! per-tick drain ships LUF1 records as BINARY payloads.
//!
//! # Why this lives in the wrapper (not `apps/desktop`)
//!
//! The std-only library cannot name `tauri` types (boundaries.toml:
//! the L5 wrapper owns the webview seam). The id → channel
//! rehydration is three lines of tauri API — it lives here, beside
//! the invocation seam it serves.
//!
//! # The payload discipline
//!
//! `Channel<InvokeResponseBody>` with [`InvokeResponseBody::Raw`]:
//! raw bytes ride the fetch-based IPC path for large payloads (a
//! 1280×720 RGBA frame is ~3.7 MB — the JSON-array form a plain
//! `Channel<Vec<u8>>` would produce is 4-6× that and evaluates as a
//! script; the Raw form is the binary lane tauri's channel protocol
//! exists for). The webview side (the UI's channel consumer, the
//! `lumen-frames.mjs` reference port) receives an `ArrayBuffer` per
//! message — one complete LUF1 record.

use tauri::ipc::{InvokeResponseBody, JavaScriptChannelId};
use tauri::{AppHandle, Manager, Wry};

/// The send side of the webview's frame channel (one record per
/// message; see the module docs for the payload discipline).
pub type FrameChannel = tauri::ipc::Channel<InvokeResponseBody>;

/// Rehydrate a bound channel id (the bare `u32` the UI sent in its
/// `session.set-frame-channel` command — tauri's `__CHANNEL__:<id>`
/// reference form) into the send-side channel on the main webview.
///
/// # Errors
/// A `String` sentence (logged by the ticker, never fatal): the id
/// form is invalid (a UI bug), or the main webview is absent (the
/// window closed mid-bind — the next bind retries).
pub fn frame_channel(app: &AppHandle<Wry>, channel: u32) -> Result<FrameChannel, String> {
    let reference = format!("__CHANNEL__:{channel}");
    let id: JavaScriptChannelId = reference
        .parse()
        .map_err(|err: &str| format!("the channel reference {reference:?} is invalid: {err}"))?;
    let webview = app
        .get_webview_window("main")
        .ok_or_else(|| "the main webview window is absent".to_string())?;
    Ok(id.channel_on(webview.as_ref().clone()))
}

/// Ship one LUF1 record as a raw binary message.
///
/// # Errors
/// The tauri send error (the webview went away — a reload or close;
/// the caller drops the channel and the next bind re-establishes).
pub fn send_record(channel: &FrameChannel, record: &[u8]) -> Result<(), tauri::Error> {
    channel.send(InvokeResponseBody::Raw(record.to_vec()))
}
