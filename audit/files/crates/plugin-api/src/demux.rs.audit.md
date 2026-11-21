# AUDIT: crates/plugin-api/src/demux.rs
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2026-11-21 |
| Lines / functions | 400 / 14 (1 trait w/ 5 methods (2 defaulted), 2 enums, 1 sink trait w/ 1 method, 2 helper fns, 1 helper struct + 2 impls, 3 tests + 3 test helpers) |
| File verdict | REAL (the trait contract the shipped demux lane runs on; two dead helper surfaces inside — F-001/F-002) |
| Reachable from shipped entry point? | YES for the trait core: `lumen open` → apps/cli/src/demux_host.rs:61 (`use plugin_api::{…Demuxer…}`) → `impl Demuxer for` in containers/{avi,mp4,ts,ps,mkv,ivf,ogg,adts}.rs (avi.rs:17, mp4.rs:160, ts.rs:513, ps.rs:493 …) → `next_event` per packet. Executed end-to-end by the baseline AVI first-vertical (INDEX.md: `v_frames=60 v_decoded=60` — 60 next_event pulls). The DEFAULT `reanchor`/`seekability` bodies are exercised in-tree only by tests + the unshipped apps/web `SelectDemuxer` (which forwards `seekability()` but not `reanchor`, per audit/files/apps/web/src/open.rs.audit.md). `PacketSink`, `container_duration`, `track_timing_conflict`: NO production caller (F-001/F-002). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | Read demux.rs 1-400 (full) · Grep `impl Demuxer for` (8 impls: apps/cli containers ps:493, ts:513, mp4:160 + rg avi/ogg/adts/mkv/ivf imports; apps/web/src/open.rs:350) · Grep `\.reanchor\(|seekability\(\)` (engine seek flow apps/cli/src/host.rs:1229 → demux_host.rs:615; per-container tests) · Grep `PacketSink for\|dyn PacketSink\|demux::PacketSink` repo-wide (every hit is codec-api's OWN encode-side PacketSink — plugin-api's demux PacketSink has zero implementors/users) · Grep `container_duration\|track_timing_conflict` (definition + own tests only — zero production callers) · baseline `orchrun-test-workspace-nofailfast.log.txt:4435-4437` (all 3 tests ok; plugin_api binary :4427) · Read audit/files/crates/plugin-api/src/lib.rs.audit.md (prior wave's facade audit — deferred per-item demux audit to this file's holder) · `rg -n "demux" audit/files/crates/demux/src/mp4/mod.rs.part4.audit.md` (W05's mp4 seek findings cross-ref). Quotes verified with the Read tool. |

## 1. One-paragraph verdict
This is the engine's REAL demux seam — the trait every shipped container implements and the baseline's one working lane (AVI MJPEG+PCM, 60/60 frames) pulls through `next_event` — with honest defaults (unseekable unless claimed; reanchor refuses typed rather than faking) that are properly pinned by tests. The hostile findings are on the edges: the module documentation describes a push dataflow ("pushes demuxed packets into the engine's PacketSink") that the implemented trait contradicts — packets are PULLED as `DemuxEvent::Packet` return values, and the `PacketSink` trait thus described has zero implementors and zero users in the entire tree (every `PacketSink` hit elsewhere is codec-api's unrelated same-named encode trait) — it is dead contract surface whose doc actively misleads a plugin author about the threading/dataflow model. The two published helpers (`container_duration`, `track_timing_conflict`) likewise have no production caller, the flagship test drives a FakeDemux (a mock of the subject under test, pinning no production behavior), and `container_duration`'s only nontrivial code path (cross-base conversion, checked subtraction, negative spans) is untested — the test feeds same-base stamps where the conversion is trivial.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/INDEX.md` AVI contrast row + `avi-firstvertical-open-session.txt`: the shipped `lumen` binary demuxes through this trait (demux_host.rs wiring) to a full Idle→…→Idle lifecycle; `decode-bench` on the AVI fixture: `v_frames=60 a_frames=60` — sixty `next_event` pulls per limb through the real trait object.
— Verified by `orchrun-test-workspace-nofailfast.log.txt:4435-4437`: `demux::tests::{default_seek_hooks_refuse_typed, events_flow_packets_then_eos, typed_error_shapes}` all `ok`.
— Verified by Grep: 8 in-tree `impl Demuxer for` sites (CLI containers ×7 + web SelectDemuxer), engine seek flow calls `reanchor` (apps/cli/src/host.rs:1229) and `seekability` gates it — the trait is load-bearing, not decorative.

## 3. Findings
### F-001 [S3]
Where: crates/plugin-api/src/demux.rs:169-183 (with module doc 5-8)
Code:
```
/// Where demuxed packets go: the engine's queue, created and consumed
/// on the demux worker thread (no `Send` bound — the codec-api
/// `FrameSink` precedent).
pub trait PacketSink {
```
(and the module header, lines 6-8: "pushes demuxed packets into the engine's\n//! [`PacketSink`]")
What is wrong: The trait is dead. Repo-wide Grep for `PacketSink for`, `dyn PacketSink`, `demux::PacketSink`: every implementation and every use is codec-api's OWN `encode::PacketSink` (codec-api/src/encode.rs:99, contract.rs:174,272, tests) — a different trait that shares the name. No type anywhere implements plugin-api's demux `PacketSink`; no function takes it; the `Demuxer` trait itself never references it. Worse, the module doc's dataflow section (lines 5-8) states the demuxer "pushes demuxed packets into the engine's [`PacketSink`]" — the implemented model is the exact opposite: the engine PULLS one `DemuxEvent::Packet(Packet)` per `next_event` return (line 32-35 documents the pull; line 6-8 contradicts it).
Why it matters: This is the contract crate — the file external plugin authors would implement against. It publishes a sink trait nobody uses and a dataflow description the trait's own signature refutes; an author wiring a `PacketSink` per the module doc builds against a fiction. The doc-vs-code contradiction is exactly the class of drift this audit exists to catch (same shape as the lib.rs facade's false CI-enforcement claim found by the earlier wave).
What correct looks like: Delete `PacketSink` (and its two lib.rs re-exports) until a push model exists, or make the dataflow real; fix the module doc to describe the pull model the trait implements.
Confidence: HIGH — Verification: read + repo-wide greps (all name-colliding hits enumerated and attributed).
### F-002 [S4]
Where: crates/plugin-api/src/demux.rs:188-198, 215-230
Code:
```
pub fn track_timing_conflict(track: &TrackId) -> EngineError {
```
(line 215: `pub fn container_duration(span: TimeSpan) -> Option<Duration> {`)
What is wrong: Both published helpers have zero production callers — Grep repo-wide returns their definitions and their own unit test (demux.rs:377, 397-398) and nothing else. Every real container computes duration/timing inside crates/demux or apps/cli containers; nothing routes through these "engine-facing" helpers.
Why it matters: Dead public API in the contract crate: maintained, tested, documented — and reachable by no code. Test coverage of a dead function is theater (the test proves the helper works, not that anything uses it).
What correct looks like: Either wire the containers' duration computation through `container_duration` (it is the carefully base-correct implementation — see F-004 for its one untested path) or mark both `#[doc(hidden)]`/remove until a consumer exists.
Confidence: HIGH — Verification: read + repo-wide grep.
### F-003 [S4]
Where: crates/plugin-api/src/demux.rs:240-262, 287-342
Code:
```
    /// One in-memory packet list standing in for a demuxed container.
    struct FakeDemux {
        packets: Vec<Packet>,
        emitted: usize,
        tracks: Vec<Track>,
    }
```
What is wrong: `events_flow_packets_then_eos` (287-342) tests the FakeDemux's own bookkeeping: it asserts that a hand-written in-memory Vec yields its packets then EndOfStream — behavior defined entirely inside the test. No production code path executes beyond the trait's definition and `DemuxEvent`'s derived `PartialEq`. It is a mock of the subject under test whose assertions cannot fail for any defect in shipped code (the real demuxers live in crates/demux and apps/cli, tested there).
Why it matters: Inflates the file's apparent coverage; a reader counting green tests believes the demux event flow is verified here. It is verified in the containers' own suites, not here.
What correct looks like: Either delete it or reframe honestly as a compile-and-shape contract test (name it `trait_shape_compiles_and_events_compare`), and let the containers' suites carry the behavior.
Confidence: HIGH — Verification: read; mutation check: breaking any real demuxer leaves this test green.
### F-004 [S4]
Where: crates/plugin-api/src/demux.rs:215-230 (test at 384-398)
Code:
```
        (Ok(a), Ok(b)) => {
            // End − start in the end stamp's base, expressed in
            // nanoseconds via the L0 checked conversion.
            let converted = a.convert(&b.base).ok()?;
            let ticks = b.ticks.checked_sub(converted.ticks)?;
```
What is wrong: The only nontrivial logic in `container_duration` — cross-base conversion (`a.convert(&b.base)`), the negative-span `checked_sub` → None path, and the `as_nanos` overflow → None path — is untested. The test feeds both stamps the SAME base (1/1000, lines 385-392), where `convert` is the identity, and only asserts `is_some()`/`is_none()` — not the value. A mutation that drops the base conversion entirely (computing `b.ticks - a.ticks` raw) passes the test; so does one that returns `Some(Duration::Known(0))` for any non-UNKNOWN span.
Why it matters: Cross-base spans are the realistic case (track timing bases differ between containers); the helper's correctness there is precisely what its careful implementation promises and nothing verifies. (Mitigated only by the fact that nothing calls it — F-002.)
What correct looks like: Assert the VALUE (`Some(Duration::Known(90 s))`), add a cross-base case (start in 1/1000, end in 1/90000), a negative span (end < start ⇒ None), and an overflow case.
Confidence: HIGH — Verification: read of both code and test; mutation reasoning as stated.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `trait Demuxer` (decl) | 52 | REAL | — | `Send` bound (doc 06 §1.2 calling context); 8 in-tree impls; load-bearing in the shipped CLI lane. |
| ↳ `start` | 58 | REAL | — | Header parse + track enumeration; implemented by all 8; executed by baseline AVI open. |
| ↳ `next_event` | 59-68 | REAL | — | The pull engine; doc contract "one container chunk per call, never read-the-whole-file" (per-container enforcement audited in the demux crate's own audits — W05/W06 territory). |
| ↳ `seek` (pure query) | 80 | REAL | — | No default (required); engine validates targets pre-session (doc 03 §6 step 1); CLI `--start-time` path uses it (worklog: demux.ts.seek.unsupported typed refusal pinned in container tests). |
| ↳ `seekability` (default `None`) | 89-91 | REAL | — | Honest default; pinned by `default_seek_hooks_refuse_typed` (assert_eq None). Forwarded by demux_host.rs:603 and every container. |
| ↳ `reanchor` (default typed refusal) | 113-120 | REAL | — | `let _ = target` + typed `Unsupported` w/ code `plugin.demux.reanchor.unsupported`; pinned (code+stage+kind asserted); engine calls reanchor at host.rs:1229 (real impls). Known in-tree consumer of the DEFAULT: apps/web SelectDemuxer (unshipped) — its seekability-without-reanchor gap is W12-A02's recorded finding. Invariant D4 (Err ⇒ unmoved) is a documented discipline, per-container verification lives in demux-crate audits (mp4 part4 found violations there). |
| `enum SeekSupport` | 128-145 | REAL | — | Doc 03 §4.3(b) shape (`Fast{indexed}`, `SlowLinear`, `None`); consumed by engine seek flow; per-container values pinned in container tests (ps.rs:738 SlowLinear, ts.rs:788 None). |
| `enum DemuxEvent` | 148-167 | REAL | — | Packet/Recovered{skipped,reason}/Discontinuity/EndOfStream; Recovered counted against engine recovery budget (MP §8) — engine side verified in playback-engine audits. |
| `trait PacketSink` + `accept` | 172-183 | **DEAD** | S3(F-001) | Zero implementors, zero users repo-wide; contradicts module doc's push claim; name-collides with codec-api's live encode trait. |
| `track_timing_conflict` | 188-198 | REAL (code) / DEAD (reach) | S4(F-002) | Correct typed error with source chain (pinned); no caller. |
| `DemuxSourceInfo` + Display + Error | 203-211 | REAL (code) / DEAD (reach) | — | Leaf source for the above; display `demux track #N`; exercised only via F-002's test. |
| `container_duration` | 215-230 | REAL (code) / DEAD (reach) | S4(F-002, F-004) | Base-correct checked arithmetic (convert → checked_sub → as_nanos, all None-propagating); untested on its nontrivial paths. |
| tests: `events_flow_packets_then_eos` | 287-342 | SHALLOW | S4(F-003) | Mock-of-subject; pins only the fake's own sequencing. Ran ok (orchrun:4436). |
| tests: `default_seek_hooks_refuse_typed` | 344-373 | REAL | — | Strong: seekability default, reanchor refusal, stage/kind/code exact-asserted. Mutation: make the default return Ok ⇒ `unreachable!("default reanchor must refuse")` fires. Ran ok (:4435). |
| tests: `typed_error_shapes` | 375-399 | REAL (partial) | S4(F-004) | Pins helper taxonomy + source-chain presence + secret-free Display; weak on container_duration (F-004). Ran ok (:4437). |
| test helpers (FakeDemux impl, packet, Bare) | 240-285, 349-360 | REAL | — | Compile-shape scaffolding; `unreachable!`-on-Err posture (loud) used throughout. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| events_flow_packets_then_eos | NO (for any production defect) | The subject is the test's own FakeDemux; only trait-definition/derive breakage (compile errors) fail it. |
| default_seek_hooks_refuse_typed | YES | Defaults are production code; exact stage/kind/code asserts + unreachable-on-Ok. |
| typed_error_shapes | PARTIALLY | Helpers' taxonomy: YES (exact asserts). container_duration behavior: NO — same-base trivial case, is_some/is_none only, value never asserted (F-004). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The demuxer … pushes demuxed packets into the engine's [`PacketSink`]" | demux.rs module doc:5-8 | FALSE | The trait pulls (`next_event -> DemuxEvent::Packet`); `PacketSink` has zero implementors/users (F-001). |
| "one container chunk per next_event, never an internal read-the-whole-file loop" | demux.rs doc:20-22 | REAL-UNPROVEN (here) | Contract statement; per-implementation verification belongs to the demux-crate/container audits (W05/W06 found mp4 seek violations; bounded-read discipline not contradicted there for next_event). |
| "Default: `SeekSupport::None` … Seeks are typed failures, never fake successes (MP §8 bullet 4)" | demux.rs:84-91 + doc 03 §4.3(b) | REAL | Default pinned by test (orchrun:4435); None-refusal flow exercised by ts container tests (ts.rs:788) and CLI `--start-time` lane. |
| "Invariant D4: on `Err` the demuxer is left at its pre-seek position" | demux.rs:100-103 (doc 03 §4.3(e)) | REAL-UNPROVEN (here) | Contract only; demux-crate audits own the proof — mp4 part4 (W05) found the seek story partially hollow there. |
| Demuxer kind 3 contract "container parse → timestamped packets + seek tables" | doc 06 §1.2 | REAL | 8 impls; the AVI first-vertical lane executes it end to end (baseline INDEX). |
| matrix row `plugin-codec-api` status "not_started" ("decoder/demuxer/access plugin contracts behind codec-api") | feature-matrix.toml:2309-2319 | MISLEADING (row scope) | The demuxer/access/codec CONTRACTS exist, compile, and carry the shipped lane (this file + access.rs); what does not exist is any dynamic plugin REGISTRY loading external implementors. The row's status speaks to the registry, its description to the contracts — internally inconsistent wording, matrix-owner's call. |

## 7. Dead / duplicated / unreachable code
- `PacketSink` (+ its lib.rs:90/:106 re-exports): dead (F-001).
- `container_duration`, `track_timing_conflict` (+ `DemuxSourceInfo`): dead-in-tree helpers (F-002).
- Name collision: plugin-api `demux::PacketSink` vs codec-api `encode::PacketSink` — only demux's is re-exported at the crate root (lib.rs:90) and in the prelude (lib.rs:106, verified); codec's rides solely as the `codec::EncodePacketSink` alias (codec.rs:20, itself flagged a dead re-export by the codec.rs audit). A prelude user importing `PacketSink` gets the dead one.

## 8. Dependencies & flags
- `media-types` only (TrackId, TimeSpan, Duration, EngineError, Packet, Track) — pure contract crate, no wasmi, no std beyond core types; `Send` bounds per doc; no unsafe; no feature gates.
- No tests are cfg-gated; all three run in the default workspace suite (orchrun:4435-4437).

## 9. Open questions for the next agent
- crates/demux owners: does any container demuxer route duration through `container_duration`? My grep says no — if you find a caller I missed, F-002 downgrades for that helper.
- demux_host.rs / host.rs owner (W13?): the default `reanchor` is reachable only via tests + unshipped apps/web; confirm no shipped container relies on the default (mp4/ts/ps all implement it — rg :548, :177, demux_host.rs:615 forward).
- Whoever owns the feature-matrix plugin rows: resolve the `plugin-codec-api` row's description-vs-status scope (contracts exist and ship; registry does not) — likely a wording fix, possibly a status split into two rows.
