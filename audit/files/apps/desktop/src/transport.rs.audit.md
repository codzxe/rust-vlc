# AUDIT: apps/desktop/src/transport.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2026-11-21 |
| Lines / functions | 279 / 12 items (1 trait, 1 enum + 3 impls, 3 structs ×(impl+FrameTransport impl), 1 enum + 1 impl, 1 const, 1 type alias) |
| File verdict | REAL (with one evidence-refuted perf claim and one dead transport) |
| Reachable from shipped entry point? | YES by design — chain: `lumen-tauri` shell (apps/desktop-tauri; its build is UNVERIFIED in this sandbox per baseline #2: exit 101 on missing gdk-3.0/gtk pkg-config — a sandbox limitation, not a repo defect) → apps/desktop shell.rs:180-187 `FrameTap::new()` + `set_present_tap` → frame_tap.rs:46 `CanvasBridge<MailboxTransport>` → transport.rs `MailboxTransport::send`. LIB-LEVEL EXECUTION in baseline: apps/desktop/tests/pipeline.rs 4/4 ok (orchrun log 2330-2340: nominal latency exact, slow-consumer latest-wins, generation-bump flush) + tests/protocol.rs 4/4 ok (2342-2351: 60-frame bridge round-trip, byte-exact composited output) + bridge unit tests over CaptureTransport; the zero-alloc test FAILED (see F-001). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `Read` over the full file + callers: `rg "MailboxTransport|CaptureTransport|NullTransport|FrameTransport|WebviewHostKind" apps/` (frame_tap.rs:40-57, bridge.rs:99-141/240+, tests/pipeline.rs:21-61, lib.rs:111-112); `Read` apps/desktop/tests/zero_alloc_transport.rs (the failing measurement); `sed -n` of the failure in orchrun log (2360-2361: "allocated 48 bytes across 1 allocation events … NOT zero-allocation"; INDEX run #3 records "608 bytes"); `rg "pub fn offer" -A18 apps/desktop/src/mailbox.rs` (never blocks/fails/reorders, drops oldest — the trait contract honored); `rg "fn decode_record" -A30 apps/desktop/src/wire.rs` (pooled payload, bounded slice work); `rg "NullTransport" repo` (definition + re-export ONLY — zero users); `rg "Null|discard|LatencyAccountant" docs/prototypes/video-in-webview.md` + section map (no null-transport evidence anywhere in the doc; LatencyAccountant lives in mailbox.rs, not transport.rs); `rg "FrameTap|set_present_tap" apps/desktop/src` (shell.rs:180-187 installs the tap); `head` of apps/desktop/examples/emit_stream.rs (the §4 evidence generator uses `wire::encode_into` directly, not NullTransport). |

## 1. One-paragraph verdict
The transport seam is real, small, and honest about what does not exist: three headless transports (null/capture/mailbox) behind a two-method trait whose "never block the presenter" contract the mailbox implementation verifiably honors (bounded slice decode + a latest-wins mailbox that drops the oldest rather than blocking), plus a `WebviewHostKind` enum whose `status()` strings correctly report that no real webview transport exists (pending ADR-004). The desktop pipeline over `MailboxTransport` executed green in the baseline (pipeline.rs and protocol.rs, 4/4 each). Two claims do not survive contact with the evidence: (1) the C7 fix's headline "no per-frame allocation after warm-up (measured by tests/zero_alloc_transport.rs)" is refuted by that very test — it FAILED in the baseline (48 bytes / 1 allocation event over 256 cycles in the no-fail-fast rerun; "608 bytes" in run #3), so the doc cites as proof a measurement that disproves it; (2) `NullTransport` is dead code — zero users repo-wide — and its doc attributes to it "the encode-side half of the throughput evidence in docs/prototypes/video-in-webview.md §4", a section that is a compatibility matrix containing no null-transport evidence at all.

## 2. What actually works (verified)
— `MailboxTransport` executed end-to-end in the baseline: tests/pipeline.rs `nominal_pipeline_delivers_every_frame_with_exact_latency` and `slow_consumer_latest_wins_with_structural_bounds` ok (orchrun log 2332-2334) — the present → send → offer → draw path delivers every frame at exact virtual-time latency and the bounded latest-wins policy holds under a slow consumer.
— Protocol conformance executed: tests/protocol.rs `bridge_stream_round_trips_sixty_frames_varied_sizes` + `renderer_composited_output_streams_byte_exact` ok (log 2345-2347) — records through the transport shape survive with payload equality.
— The never-blocks contract honored by construction (read): `send` = `decode_record` (wire.rs:572-600 — header slice-window, one pooled-buffer copy, no unbounded work) + `mailbox.offer` (mailbox.rs:140-154 — "Never blocks, never fails, never reorders; drops the oldest when full").
— The one-decoder-for-life shape (the C7 fix's structure) is real: `MailboxTransport` holds `decoder: WireDecoder` (186-189), `send` reuses it (225-231) — the old per-send construction is gone; the structural claim is true even though the zero-BYTES claim is not (F-001).
— `WebviewHostKind::status()` is truthful: "pending ADR-004: tauri/wry not imported" — verified, the desktop crate has no tauri dependency (the tauri shell is the separate apps/desktop-tauri workspace member; baseline #2 documents its sandbox-blocked build).
— `From<MailboxError>` is exhaustive (single-variant source), `TransportError` Display is redaction-safe (structured variants + pre-redacted Hosted messages).

## 3. Findings

### F-001 [S2] The C7 zero-allocation claim is refuted by the project's own recorded baseline: the cited measurement test FAILED — the mailbox frame path allocates
Where: apps/desktop/src/transport.rs:174-179 (claim), measured by apps/desktop/tests/zero_alloc_transport.rs:69-102 (FAILED)
Code:
```
/// the C7 fix's zero-allocation shape: **one** [`WireDecoder`] lives for
/// the transport's whole life (the old code built a fresh one per
/// `send`), and each record is decoded **in place** — slice-window
/// header parse, one copy into a pooled payload buffer, no
/// pending-buffer round trip, no per-frame allocation after warm-up
/// (measured by `tests/zero_alloc_transport.rs`).
```
What is wrong: the doc's proof citation is a failing test. Baseline run #3 (INDEX: 1681 passed / 1 failed — "apps/desktop/tests/zero_alloc_transport.rs:98 zero-alloc claim (608 bytes allocated)") and the no-fail-fast rerun (orchrun log 2360-2361): "mailbox frame path: present -> serialize -> send -> decode_record -> offer -> draw over 256 cycles: allocated 48 bytes across 1 allocation events (dealloc events: 2) — the path is NOT zero-allocation". The measured window includes this file's `send` (the reused `WireDecoder` + `mailbox.offer`) plus bridge.rs's serialize and the mailbox draw — the root-cause site is not localized by the recorded output (48 bytes, one event, could be a single pool/queue boundary miss in any of the four crates on the path), but the CLAIM as written ("no per-frame allocation after warm-up", with that test named as the measurement) is false per the project's own evidence.
Why it matters: this is the desktop frame path's headline performance property (the C7/ED-FIX-2 audit row), documented as proven while the proving test is red in the baseline — a standing failing test is bad enough; a doc that cites it as passed is evidence inversion. The INDEX already registers the failure; the doc comment contradicts the INDEX.
What correct looks like: either fix the allocation (profile the one 48-byte event — REQUEST-ORCH-RUN below) and keep the claim, or weaken the doc to "bounded allocation (measured: ≤ N bytes/window; the zero-alloc target is tracked by the failing test until fixed)". The doc and the test must agree.
Confidence: HIGH — Verification: read of the claim + the two recorded baseline failures (INDEX #3; orchrun log 2360-2361). Root-cause site UNVERIFIED (needs an allocation-profile run).

### F-002 [S3] `NullTransport` is dead code, and its doc attributes throughput evidence to a prototype-doc section that contains none
Where: apps/desktop/src/transport.rs:93-95 (doc) and 96-132 (the type)
Code:
```
/// The measurement transport: accepts and discards, counts bytes and
/// records (the encode-side half of the throughput evidence in
/// `docs/prototypes/video-in-webview.md` §4).
```
What is wrong: (a) zero users — `rg "NullTransport"` across the repo (excluding target/) finds only the definition and the lib.rs re-export; no test, example, or prototype drives it (the actual §4 evidence generator, apps/desktop/examples/emit_stream.rs, uses `desktop::wire::{encode_into, WireDecoder}` directly and never constructs a transport). (b) The cited §4 of docs/prototypes/video-in-webview.md is the "Compatibility matrix" section — it mentions no null transport, no throughput measurement, and no discard path (`rg "Null|discard"` over the doc: no hits); the doc's measured-throughput section (§ "What WAS measured") credits the tests suites, not NullTransport.
Why it matters: a public API type whose raison d'être is a citation that does not exist — dead code carrying a false evidence pointer, the exact pattern that inflates a capability story (the same prototype doc already carries a phantom `harness.mjs` reference found by the wire.rs#part1 audit).
What correct looks like: either delete NullTransport, or wire it into a real throughput measurement and fix the citation; at minimum the doc must point at evidence that exists.
Confidence: HIGH — Verification: repo-wide grep (definition + re-export only) + read of the prototype doc's section map and §4 content + read of emit_stream.rs's imports.

### F-003 [S4] `CaptureTransport` is an unbounded accumulator in the shipped lib's public API; the safety discipline is prose only
Where: apps/desktop/src/transport.rs:134-165
Code:
```
/// The conformance transport: keeps every encoded record (bounded by
/// the caller's discipline — it is a test/fixture type, never wired
/// into a playback session).
```
What is wrong: `send` pushes an unbounded `Vec<Vec<u8>>` copy per record; the "never wired into a playback session" rule exists only in this comment. Verified today: only bridge.rs's tests construct it — so no live violation — but nothing structural (a capacity bound, a debug_assert, a #[doc(hidden]-style demotion to a test-support crate) enforces the discipline for future callers, and a playback session wired to it would grow without bound (one full-frame copy per presented frame).
Why it matters: the blast radius is future misuse, not current behavior; hence S4.
What correct looks like: a `max_records` bound with a typed overflow error (mirroring the mailbox's own discipline), or a move into a `#[cfg(test)]`-visible module / dev-dependency crate.
Confidence: HIGH — Verification: read + grep (bridge.rs tests are the sole constructors).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `trait FrameTransport` (+`send`, `name`) | 40-50 | REAL | — | Two-method seam; the never-blocks contract is documented at the trait and honored by the mailbox impl (§2). Consumed by `CanvasBridge<T: FrameTransport>` (bridge.rs:99) — generic, no god-trait. |
| `enum TransportError` (+`Display`, `Error`, `From<MailboxError>`) | 52-91 | REAL | — | Two structured variants + pre-redacted Hosted; exhaustive From; no stringly errors. |
| `NullTransport` (+`new`, `records`, `bytes`, impl) | 93-132 | STUB-adjacent dead code | S3 (F-002) | Correct little counter — with zero users and a false evidence citation (F-002). |
| `CaptureTransport` (+`new`, `records`, impl) | 134-165 | REAL | S4 (F-003) | Test/fixture type; unbounded by design, discipline prose-only; executed by bridge.rs tests (records asserted). |
| `MailboxTransport` (+`new`, `mailbox`, `mailbox_mut`) | 167-215 | REAL | — | `new` types the zero-capacity config bug; the consumer half exposed for draining. Executed via pipeline.rs/protocol.rs. |
| `MailboxTransport::send` (FrameTransport impl) | 217-232 | REAL | S2-adjacent (F-001 path) | Reused decoder + pooled decode + bounded offer — structurally the C7 shape; the zero-BYTES claim refuted (F-001). Malformed record → typed Hosted failure (a host bug surfaced, never garbage shipped). |
| `MailboxTransport::name` | 234-236 | REAL | — | "mailbox" — truthful capability naming. |
| `enum WebviewHostKind` (+`GATING_ADR`, `status`) | 239-274 | REAL | — | The honest pending state: three variants, none implemented, `status()` strings match reality (no tauri/wry import — verified); the `UnsupportedBackend` "enumerate nothing, never fake success" precedent. |
| `type ConsumerFrame` | 276-278 | REAL | — | Re-export alias; no new semantics. |

## 5. Test honesty
| Test (file — transport-adjacent) | Would it fail if the code were broken? | Reason |
|---|---|---|
| zero_alloc_transport::mailbox_frame_path (FAILED) | YES — and it DID | The counting-allocator guard caught a real 48-byte allocation the warm-up did not cover; the test is honest, the doc claim it pins is not (F-001). |
| zero_alloc_transport::streaming_decoder_path (ok) | YES | Same discipline over feed+next_frame; passed — narrowing the culprit to the mailbox-path-only code (bridge serialize / decode_record / offer / draw). |
| pipeline.rs nominal/slow-consumer/generation (4/4 ok) | YES | Exact-latency and latest-wins bounds asserted on the real path (orchrun log 2332-2334). |
| protocol.rs round-trips (4/4 ok) | YES | Payload equality through the transport shape at 1080p and varied sizes. |
| bridge.rs tests over CaptureTransport | YES | Record content asserted; capture breakage fails them. |
| (No test constructs NullTransport) | — | F-002. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no per-frame allocation after warm-up (measured by tests/zero_alloc_transport.rs)" | this file 174-179 | FALSE per recorded evidence | The cited test FAILED in baseline #3 (608 bytes per INDEX) and in the no-fail-fast rerun (48 bytes / 1 event, orchrun log 2360-2361) — F-001. |
| "NullTransport … the encode-side half of the throughput evidence in docs/prototypes/video-in-webview.md §4" | this file 93-95 | FALSE | §4 of that doc is the compatibility matrix; no null/discard/throughput mention (rg: no hits); emit_stream.rs (the §-referenced generator) never constructs it — F-002. |
| "v0 ships the headless trio — NullTransport … MailboxTransport … CaptureTransport" | module docs 6-9 | PARTLY MISLEADING | The trio exists, but one third is dead code (F-002); "ships" overstates a lib that no shipped binary builds in this sandbox (desktop-tauri build UNVERIFIED, baseline #2). |
| "Tauri IPC channel … gated on ADR-004 … [PROPOSAL]" / WebSocket / SAB | module docs 12-22 | REAL (as pending) | No such transports exist — honestly marked `[PROPOSAL]`, mirrored by `WebviewHostKind::status()`; consistent with wire.rs#part1 audit's finding. |
| "send never blocks the presenter" (trait contract) | this file 31-39 | REAL | Read-verified on the mailbox impl (mailbox.rs:140-154 offer semantics; wire.rs:572-600 bounded decode); pipeline.rs slow-consumer test executed the policy. |

## 7. Dead / duplicated / unreachable code
- `NullTransport` (93-132): dead pub API, zero users (F-002).
- `CaptureTransport`: live only in tests (documented as such; F-003).
- Nothing else; `MailboxTransport` and the trait have real consumers (frame_tap.rs, bridge.rs, tests).

## 8. Dependencies & flags
- std-only file; depends on crate-internal error/mailbox/wire (all in-bounds; mailbox.rs and wire.rs are A04's/A03+A07's files this wave). No feature gates, no unsafe, no panics; `#[must_use]` discipline on constructors/accessors; no `#[allow]` in this file.

## 9. Open questions for the next agent
- A06 (bridge.rs) and A03/A07 (wire.rs): the 48-byte allocation in F-001's measured window crosses your files (bridge serialize scratch / decode_record pool / mailbox queue) — the recorded output does not localize it. REQUEST-ORCH-RUN: `cargo test -p desktop --test zero_alloc_transport -- --nocapture` with an allocation backtrace (testkit's CountingGlobalAlloc may need a temporary site print — an xtask/debug build), or `RUST_BACKTRACE=1` plus a patched guard, to pin the site; expected outcome: one 48-byte allocation event in the first measured cycle.
- A04 (mailbox.rs): the prototype doc's "transport.rs (byte transport + LatencyAccountant)" line misattributes LatencyAccountant to transport.rs — it lives in your file (mailbox.rs:202); doc drift worth a row in your audit.
- Lead/ADR-004: when the Tauri channel transport lands, `WebviewHostKind::status()` strings and this file's module docs need the same one-commit update discipline as the control-server gate test (my file-4 audit).
