//! The hand-rolled `extern "C"` ABI over shared linear memory (card
//! 7.H1, issue #47; doc 09 §2.2's raw-import fallback lane).
//!
//! # Why hand-rolled, not wasm-bindgen
//!
//! Doc 09 §2.2 recommends wasm-bindgen as a `[PROPOSAL]` (its OQ-1
//! is undecided), but nothing in doc 09 §3 — the threading model
//! this card implements — requires it: v1 is the single-threaded
//! cooperative lane; `wasm-bindgen-rayon` is only §3.3's v2
//! (SharedArrayBuffer + COOP/COEP) path. The card envelope prefers
//! ZERO new dependencies, and this crate adds none beyond the
//! command layer's own serde wire family (the `lumen-cli`
//! precedent). The choice is recorded here and in the manifest; if
//! wasm-bindgen is ever ratified, it slots in behind this same
//! boundary without touching an engine crate.
//!
//! # The wire: JSON over four-byte words
//!
//! The contract both sides speak is the JSON form of
//! [`HostToWorker`]/[`WorkerToHost`] — exactly what the MessagePort
//! carries, so the ABI marshals the SAME envelopes the typed port
//! contract defines (no second protocol to drift).
//!
//! * **Downstream (JS → wasm):** the glue encodes one message with
//!   `TextEncoder`, pads the byte string to a multiple of four with
//!   SPACES (JSON permits trailing whitespace), and pushes it one
//!   `u32` word at a time through [`lumen_web_write_u32`]. One
//!   call per 4 bytes — the no-`unsafe` price, measured cheap
//!   enough for v1 (a `loadBlob` message is the large one; the raw
//!   linear-memory blob lane is card 7.H4's follow-up).
//! * **Upstream (wasm → JS):** after every dispatch the host stages
//!   every queued outbound message as ONE JSON array in a stable
//!   buffer; [`lumen_web_output_addr`] + [`lumen_web_output_len`]
//!   expose its position in the exported linear memory and the
//!   glue reads it directly (`memory.buffer` — the JS side needs
//!   no help). The buffer is valid until the next dispatch; the
//!   glue reads before any further call.
//!
//! # Why the word lane exists at all
//!
//! AGENTS.md hard rule 3: no `unsafe` code in application crates
//! (a platform adapter would need an approved ADR). The two ways to
//! cross the wasm boundary with raw pointers both dereference
//! host-written memory — `slice::from_raw_parts` — i.e. `unsafe`.
//! The word-push input lane and the JS-read output lane keep the
//! whole module `unsafe`-free at zero new dependencies, trading
//! input throughput for the policy. The staging buffer never moves
//! while the glue reads it (no allocation between the two output
//! calls and the read — the protocol's documented invariant), and
//! wasm memory growth cannot detach what JS has already copied.
//!
//! # The state
//!
//! One worker per module instance: [`HOST`] holds the [`WebHost`]
//! behind a `Mutex` (interior mutability, no `static mut` — the
//! boundaries manifest's rule). Single-threaded in practice (the
//! Worker), `Send + Sync` for the type system.
//!
//! # Evidence labels
//!
//! [`AbiWorker::dispatch_json`] — the dispatch core behind the
//! exports — is pure logic and unit-tested on the native host with
//! the REAL fixture (the SAME path the JS glue drives). The
//! exports themselves (`#[unsafe(no_mangle)] extern "C"`) are
//! exercised only by a browser runtime — `[VERIFY]`, not claimed
//! here.

// AGENTS.md hard rule 3 exempts NOTHING outright — but this module
// contains ZERO `unsafe` blocks. The allow below exists solely for
// the four `#[unsafe(no_mangle)]` export attributes (edition-2024
// syntax, flagged by the workspace `unsafe_code = deny` lint as an
// "unsafe attribute" because symbol names are linker-global): the
// names are prefixed `lumen_web_*` (collision-free by construction)
// and exporting them is the entire point of the module. This is the
// platform-adapter shape rule 3 anticipates ("approved ADR +
// documented invariants") — `[PROPOSAL: Lead ratifies the wasm
// export seam as the apps/web ADR, or supplies the bindgen
// dependency decision (doc 09 §2.2 OQ-1)]`. Every other module in
// this crate compiles under the deny.
#![allow(unsafe_code)]

use std::sync::{Mutex, OnceLock};

use crate::host::WebHost;
use crate::protocol::HostToWorker;

/// The per-message input bound (bytes; MP §11 — bounded memory
/// everywhere). A `loadBlob` rides the JSON contract in v1, so this
/// is the effective media-size bound of the no-`unsafe` lane; the
/// raw linear-memory blob lane (7.H4) lifts it.
pub const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;

/// The outbound-queue bound (messages; MP §11): a caller that never
/// drains the output (the port contract drains after every
/// dispatch) is refused typed, never allowed to grow the queue
/// unboundedly.
pub const MAX_PENDING_MESSAGES: usize = 4096;

/// Status: the message was applied.
pub const STATUS_OK: u32 = 0;
/// Status: the input was not valid JSON for the contract — the
/// typed control error rides the output queue.
pub const STATUS_PARSE_ERROR: u32 = 1;
/// Status: the input exceeded [`MAX_INPUT_BYTES`].
pub const STATUS_OVERFLOW: u32 = 2;

/// The worker behind the exports: the host plus the two staging
/// halves of the wire (input words in, output JSON out).
pub struct AbiWorker {
    host: WebHost,
    input: Vec<u8>,
    input_overflowed: bool,
    output: String,
    /// Whether `output` still holds the staging for the last
    /// dispatch: repeated reads between dispatches must see the
    /// SAME bytes (the documented invariant — `take_outbound` is
    /// destructive, so a re-stage would silently answer `[]`).
    staged: bool,
}

impl AbiWorker {
    /// A worker for the build's own profile.
    #[must_use]
    pub fn new() -> AbiWorker {
        AbiWorker {
            host: WebHost::new(crate::capabilities::build_profile()),
            input: Vec::new(),
            input_overflowed: false,
            output: String::new(),
            staged: false,
        }
    }

    /// The host (tests drive the message paths through it).
    #[must_use]
    pub fn host(&self) -> &WebHost {
        &self.host
    }

    /// The host (mutable test access).
    pub fn host_mut(&mut self) -> &mut WebHost {
        &mut self.host
    }

    /// Push four input bytes (little-endian). Answers the buffered
    /// byte count, or `0` once the bound is exceeded (the overflow
    /// latch — the next dispatch answers the typed error).
    pub fn write_word(&mut self, word: u32) -> usize {
        if self.input.len() + 4 > MAX_INPUT_BYTES {
            self.input_overflowed = true;
            return 0;
        }
        self.input.extend_from_slice(&word.to_le_bytes());
        self.input.len()
    }

    /// Whether the input lane overflowed since the last dispatch.
    #[must_use]
    pub fn input_overflowed(&self) -> bool {
        self.input_overflowed
    }

    /// Drop the buffered input (after the overflow answer).
    pub fn clear_input(&mut self) {
        self.input.clear();
        self.input_overflowed = false;
    }

    /// Apply the buffered input as one downstream message, stage
    /// every outbound message as one JSON array, clear the input.
    /// Answers the status code; parse failures stage the typed
    /// control error instead of crashing the worker.
    pub fn dispatch(&mut self) -> u32 {
        let status = if self.input_overflowed {
            self.clear_input();
            self.host.handle_control_error(
                "web.abi.overflow",
                format!(
                    "the downstream message exceeded the input bound ({MAX_INPUT_BYTES} bytes)"
                ),
            );
            STATUS_OVERFLOW
        } else {
            let text = String::from_utf8_lossy(&self.input).into_owned();
            self.input.clear();
            match serde_json::from_str::<HostToWorker>(&text) {
                Ok(message) => {
                    self.host.handle(message);
                    STATUS_OK
                }
                Err(error) => {
                    self.host.handle_control_error(
                        "web.abi.parse",
                        format!("the downstream message did not decode: {error}"),
                    );
                    STATUS_PARSE_ERROR
                }
            }
        };
        // New outbound may exist: the next read re-stages.
        self.staged = false;
        status
    }

    /// Apply one downstream message directly (the in-memory port
    /// double's lane — no word staging, same host paths) and stage
    /// the output.
    pub fn dispatch_json(&mut self, text: &str) -> u32 {
        let status = match serde_json::from_str::<HostToWorker>(text) {
            Ok(message) => {
                self.host.handle(message);
                STATUS_OK
            }
            Err(error) => {
                self.host.handle_control_error(
                    "web.abi.parse",
                    format!("the downstream message did not decode: {error}"),
                );
                STATUS_PARSE_ERROR
            }
        };
        // New outbound may exist: the next read re-stages.
        self.staged = false;
        status
    }

    /// Stage the outbound queue as one JSON array (the output
    /// lane). Idempotent between dispatches: the staging for the
    /// last dispatch survives every number of re-reads.
    fn stage_output(&mut self) {
        if self.staged {
            return;
        }
        let messages = self.host.take_outbound();
        self.output.clear();
        match serde_json::to_string(&messages) {
            Ok(json) => self.output.push_str(&json),
            Err(_) => {
                // Serialization cannot fail for these types (no
                // non-string map keys, no cycles); if it ever does,
                // the honest answer is an empty batch, never a
                // dead worker.
                self.output.push_str("[]");
            }
        }
        self.staged = true;
    }

    /// The staged output's address in linear memory (valid until
    /// the next dispatch — re-reads between dispatches see the same
    /// bytes). The glue reads it via `memory.buffer` before any
    /// further call.
    #[must_use]
    pub fn output_addr(&mut self) -> u32 {
        self.stage_output();
        self.output.as_ptr() as u32
    }

    /// The staged output's byte length.
    #[must_use]
    pub fn output_len(&self) -> u32 {
        u32::try_from(self.output.len()).unwrap_or(u32::MAX)
    }

    /// The staged output itself (the native test lane).
    #[must_use]
    pub fn output_json(&mut self) -> String {
        self.stage_output();
        self.output.clone()
    }
}

impl Default for AbiWorker {
    fn default() -> Self {
        AbiWorker::new()
    }
}

/// The one worker per module instance. `OnceLock` + `Mutex`:
/// interior mutability without `static mut` (the boundaries
/// manifest's `static_mut_allow` stays empty).
static HOST: OnceLock<Mutex<AbiWorker>> = OnceLock::new();

/// The global worker (lazily constructed on first use).
fn worker() -> &'static Mutex<AbiWorker> {
    HOST.get_or_init(|| Mutex::new(AbiWorker::new()))
}

/// Lock the worker, recovering from poisoning (a panicked critical
/// section cannot corrupt plain staging buffers; the worker is
/// single-threaded in practice).
fn lock_worker() -> std::sync::MutexGuard<'static, AbiWorker> {
    match worker().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Push four input bytes (little-endian). Answers the buffered byte
/// count, or `0` once [`MAX_INPUT_BYTES`] is exceeded.
#[unsafe(no_mangle)]
pub extern "C" fn lumen_web_write_u32(word: u32) -> u32 {
    let mut worker = lock_worker();
    u32::try_from(worker.write_word(word)).unwrap_or(0)
}

/// Apply the buffered input as one downstream message and stage the
/// outbound queue. Answers [`STATUS_OK`], [`STATUS_PARSE_ERROR`] or
/// [`STATUS_OVERFLOW`] (the typed control error rides the output).
#[unsafe(no_mangle)]
pub extern "C" fn lumen_web_dispatch() -> u32 {
    let mut worker = lock_worker();
    let status = worker.dispatch();
    if status != STATUS_OK {
        worker.stage_output();
    }
    status
}

/// The staged output's address in the exported linear memory. Valid
/// until the next `lumen_web_dispatch` — re-reads between dispatches
/// see the same bytes (the staging is idempotent per dispatch). The
/// glue must read it (via `memory.buffer`) before any further call.
#[unsafe(no_mangle)]
pub extern "C" fn lumen_web_output_addr() -> u32 {
    let mut worker = lock_worker();
    worker.output_addr()
}

/// The staged output's byte length (pair with
/// [`lumen_web_output_addr`]).
#[unsafe(no_mangle)]
pub extern "C" fn lumen_web_output_len() -> u32 {
    let worker = lock_worker();
    worker.output_len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{HostToWorker, PumpReport, WorkerToHost};
    use commands::Command;
    use commands::envelope::RequestId;
    use testkit::{Must, fixture_bytes};

    /// The in-memory MessagePort double: what the JS glue does —
    /// serialize a message, run it through the ABI's dispatch core,
    /// parse the outbound batch — with no port at all. Every test
    /// below drives the SAME paths the browser would.
    struct PortDouble {
        worker: AbiWorker,
    }

    impl PortDouble {
        fn new() -> PortDouble {
            PortDouble {
                worker: AbiWorker::new(),
            }
        }

        /// Send one downstream message; answer the upstream batch.
        fn post(&mut self, message: &HostToWorker) -> Vec<WorkerToHost> {
            let text = serde_json::to_string(message).must();
            let status = self.worker.dispatch_json(&text);
            assert_eq!(status, STATUS_OK, "the double only sends valid messages");
            let output = self.worker.output_json();
            if output == "[]" {
                return Vec::new();
            }
            serde_json::from_str(&output).must()
        }

        /// Send raw text (the malformed-input lane).
        fn post_raw(&mut self, text: &str) -> (u32, Vec<WorkerToHost>) {
            let status = self.worker.dispatch_json(text);
            let output = self.worker.output_json();
            let messages: Vec<WorkerToHost> = if output == "[]" {
                Vec::new()
            } else {
                serde_json::from_str(&output).must()
            };
            (status, messages)
        }
    }

    /// The lane proof, end to end through the ABI's dispatch core:
    /// boot → loadBlob → open → pump slices → progress events → the
    /// honest capability report → shutdown. The SAME message paths
    /// the JS glue drives (headless-native evidence; the browser
    /// itself is `[VERIFY]`).
    #[test]
    fn the_lane_proof_over_the_abi() {
        let mut port = PortDouble::new();

        // 1. Boot: the handshake answers EngineInfo + the honest
        //    capability snapshot (threading: single, the typed
        //    blocked rows).
        let replies = port.post(&HostToWorker::Boot);
        match replies.first() {
            Some(WorkerToHost::Booted {
                engine_info,
                capabilities,
            }) => {
                assert_eq!(engine_info.host, "web");
                assert_eq!(capabilities.threading.as_str(), "single");
                assert!(
                    capabilities.blocked_row("rtsp-rtp-udp-access").is_some(),
                    "doc 09 §10's rows are typed data"
                );
                // The native profile claims its real decoders.
                assert!(
                    capabilities.decoders.iter().any(|d| d == "mjpeg"),
                    "{:?}",
                    capabilities.decoders
                );
            }
            other => unreachable!("boot must answer the handshake: {other:?}"),
        }

        // 2. The blob pre-stage (the File/Blob no-upload path).
        let bytes = fixture_bytes("generated/valid/avi_mjpeg_pcm.avi").must();
        let name = "first-vertical".to_string();
        let size = bytes.len() as u64;
        let replies = port.post(&HostToWorker::LoadBlob {
            name: name.clone(),
            bytes,
        });
        assert!(replies.contains(&WorkerToHost::BlobLoaded { name, size }));

        // 3. The catalog command lane: session.open over the blob.
        let replies = port.post(&HostToWorker::Command(commands::WireRequest {
            request_id: RequestId(1),
            command: Command::SessionOpen {
                source: commands::catalog::types::SourceSpec::Url {
                    url: "blob:first-vertical".to_string(),
                },
                options: Default::default(),
            },
        }));
        match replies.first() {
            Some(WorkerToHost::Response(response)) => {
                assert_eq!(response.request_id, RequestId(1), "the id is echoed");
                assert!(
                    response.result.as_ref().is_ok_and(|outcome| {
                        matches!(outcome, commands::CommandOutcome::Accepted { .. })
                    }),
                    "open accepts: {:?}",
                    response.result
                );
            }
            other => unreachable!("open must answer a response: {other:?}"),
        }

        // 4. Pump slices: progress events flow, the reports answer
        //    the re-arm truth, the media plays to its end.
        let mut saw_playing = false;
        let mut saw_end_of_media = false;
        let mut last = PumpReport {
            state: commands::PlaybackState::Opening,
            position_us: 0,
            busy: true,
            events_flushed: 0,
        };
        for _ in 0..64 {
            let replies = port.post(&HostToWorker::Pump { ticks: 4 });
            for reply in &replies {
                match reply {
                    WorkerToHost::Events { events } => {
                        for event in events {
                            match &event.kind {
                                commands::EventKind::SessionStateChanged { to, .. } => {
                                    if *to == commands::PlaybackState::Playing {
                                        saw_playing = true;
                                    }
                                }
                                commands::EventKind::PlaybackEndOfMedia => {
                                    saw_end_of_media = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    WorkerToHost::Pumped { report } => {
                        last = *report;
                    }
                    _ => {}
                }
            }
            if !last.busy {
                break;
            }
        }
        assert!(saw_playing, "the progress events crossed Playing");
        assert!(saw_end_of_media, "the end-of-media event arrived");
        assert_eq!(last.state, commands::PlaybackState::Ended);
        assert!(!last.busy, "the re-arm truth releases at the end");

        // 5. The snapshot lane over the same port: the position
        //    reached the media end (played, not just decoded).
        let replies = port.post(&HostToWorker::Command(commands::WireRequest {
            request_id: RequestId(2),
            command: Command::MediaGetCurrentTime,
        }));
        match replies.first() {
            Some(WorkerToHost::Response(response)) => {
                match &response.result {
                    Ok(commands::CommandOutcome::Completed(
                        commands::CommandResult::CurrentTime { time, position, .. },
                    )) => {
                        assert!(time.0 > 0, "position {time:?}");
                        assert!(
                            (0.95..=1.0).contains(position),
                            "fraction {position} (≈ the end)"
                        );
                    }
                    other => unreachable!("current-time must complete: {other:?}"),
                }
            }
            other => unreachable!("current-time must answer: {other:?}"),
        }

        // 6. The graceful shutdown: the acknowledgment, the host
        //    reusable.
        let replies = port.post(&HostToWorker::Shutdown);
        assert!(
            replies.contains(&WorkerToHost::ShutdownComplete),
            "{replies:?}"
        );
    }

    /// The word lane round-trips messages exactly as the JS glue
    /// pushes them (four bytes per call, space-padded tail) — the
    /// handshake first, then the pump.
    #[test]
    fn the_word_lane_matches_the_json_lane() {
        let mut by_words = AbiWorker::new();
        let send_words = |worker: &mut AbiWorker, message: &HostToWorker| {
            let text = serde_json::to_string(message).must();
            // Pad with spaces to a word boundary — JSON tolerates the
            // trailing whitespace, exactly the glue's contract.
            let mut padded = text.into_bytes();
            while !padded.len().is_multiple_of(4) {
                padded.push(b' ');
            }
            for chunk in padded.chunks(4) {
                let mut word = [0_u8; 4];
                word.copy_from_slice(chunk);
                let buffered = worker.write_word(u32::from_le_bytes(word));
                assert!(buffered > 0, "the word fits the bound");
            }
            worker.dispatch()
        };
        // The handshake (the not-booted lane refuses everything else).
        assert_eq!(send_words(&mut by_words, &HostToWorker::Boot), STATUS_OK);
        let boot_replies: Vec<WorkerToHost> = serde_json::from_str(&by_words.output_json()).must();
        assert!(
            boot_replies
                .iter()
                .any(|reply| matches!(reply, WorkerToHost::Booted { .. })),
            "the handshake answered through the word lane: {boot_replies:?}"
        );
        // Then the pump.
        assert_eq!(
            send_words(&mut by_words, &HostToWorker::Pump { ticks: 3 }),
            STATUS_OK
        );
        let output = by_words.output_json();
        let replies: Vec<WorkerToHost> = serde_json::from_str(&output).must();
        assert!(
            replies
                .iter()
                .any(|reply| matches!(reply, WorkerToHost::Pumped { .. })),
            "the pump answered through the word lane: {replies:?}"
        );
    }

    /// Malformed input is the typed control error, never a crash —
    /// and the worker keeps serving afterwards.
    #[test]
    fn malformed_input_is_typed_and_recoverable() {
        let mut port = PortDouble::new();
        let (status, replies) = port.post_raw("this is not json");
        assert_eq!(status, STATUS_PARSE_ERROR);
        match replies.first() {
            Some(WorkerToHost::ControlError { code, .. }) => {
                assert_eq!(code, "web.abi.parse");
            }
            other => unreachable!("parse errors answer typed: {other:?}"),
        }
        // The worker still boots.
        let replies = port.post(&HostToWorker::Boot);
        assert!(
            replies
                .iter()
                .any(|reply| matches!(reply, WorkerToHost::Booted { .. })),
            "the worker recovered: {replies:?}"
        );
    }

    /// The input bound is enforced: words past [`MAX_INPUT_BYTES`]
    /// answer 0 and the next dispatch answers the typed overflow.
    #[test]
    fn the_input_bound_is_typed() {
        let mut worker = AbiWorker::new();
        let mut pushed = 0_usize;
        while pushed <= MAX_INPUT_BYTES {
            let word = *b"abcd";
            let buffered = worker.write_word(u32::from_le_bytes(word));
            if buffered == 0 {
                break;
            }
            pushed = buffered;
        }
        assert!(pushed >= MAX_INPUT_BYTES, "the bound was reached");
        let status = worker.dispatch();
        assert_eq!(status, STATUS_OVERFLOW);
        let output = worker.output_json();
        let replies: Vec<WorkerToHost> = serde_json::from_str(&output).must();
        match replies.first() {
            Some(WorkerToHost::ControlError { code, .. }) => assert_eq!(code, "web.abi.overflow"),
            other => unreachable!("overflow answers typed: {other:?}"),
        }
    }

    /// The staged output's address/length pair describes the exact
    /// JSON bytes (the invariant the JS read depends on), and the
    /// staging survives any number of re-reads between dispatches.
    #[test]
    fn output_addr_and_len_describe_the_staged_bytes() {
        let mut worker = AbiWorker::new();
        worker.dispatch_json(&serde_json::to_string(&HostToWorker::Boot).must());
        let addr = worker.output_addr() as usize;
        let len = worker.output_len() as usize;
        assert!(len > 2, "the boot staged a reply");
        // The address/length pair describes the staged buffer exactly
        // (the same bytes the glue reads through memory.buffer). The
        // ABI's address width is u32 — exact on wasm32; on the native
        // test lane compare in the ABI's own width.
        assert_eq!(worker.output.as_ptr() as u32 as usize, addr);
        assert_eq!(worker.output.len(), len);
        // A re-read between dispatches sees the SAME bytes (the
        // documented invariant — the destructive take must not leak).
        let json = worker.output_json();
        assert_eq!(json.len(), len, "the re-read is stable");
        assert_eq!(worker.output.as_ptr() as u32 as usize, addr);
    }
}
