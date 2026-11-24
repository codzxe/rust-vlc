# AUDIT: apps/web/src/protocol.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-02-14 |
| Lines / functions | 417 / 4 (2 serde enums + PumpReport + 2 type_tag impls; 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | WEB LANE ONLY: abi.rs:193 `serde_json::from_str::<HostToWorker>` → host.rs:984 `handle()` (all 7 variants dispatched) → replies staged as `WorkerToHost` JSON. Not in native release lane; wasm artifact staged only by the e2e matrix lane. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | rg (variant handling, glue usage); sed quote checks; baseline citation |

## 1. One-paragraph verdict
A real, well-tested wire contract: both directions round-trip through the exact JSON the JS glue and wasm ABI share, the catalog envelopes ride flat under a `"type"` tag (pinned by test), the legacy `{"type":"boot"}` decode is pinned, and the host dispatches every downstream variant with a bounded outbound queue (host.rs:984-1074, cap `MAX_PENDING_MESSAGES`=4096 with typed backpressure). All 6 tests passed in the baseline (:3409-3414). The one material defect is the LoadBlob design: blob bytes ride the contract as a serde `Vec<u8>` — a JSON array of numbers, 2-6× the media size in text — through a 32 MiB input bound, making the web host's ONLY file-loading lane unusable for typical media files (a 700 MB movie would need ~2-3 GB of JSON). It is honestly documented as the v1 lane with the raw linear-memory blob lane deferred to 7.H4, but the ceiling is real and the deferred lane does not exist.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:3409-3414: all 6 `protocol::tests` `ok` (round-trips, flat envelope, completed-response, type tags).
— Dispatch completeness verified: host.rs:1012-1072 handles Boot/LoadBlob/Command/Pump/SurfaceLost/AudioContext/Shutdown — every `HostToWorker` variant; pre-boot messages get the typed `web.worker.not_booted` refusal (host.rs:1002-1010).
— JS glue reality verified: ui/e2e/web/site/harness.js:188 posts `{ type: "loadBlob", name: "fixture", bytes: Array.from(bytes) }` — the contract is exercised end-to-end by the browser-matrix lane.

## 3. Findings
F-001 [S2]
Where: apps/web/src/protocol.rs:69-75 (+ module docs 32-35; abi.rs:91)
Code:
```
    LoadBlob {
        /// The registry name (referenced by `blob:<name>` URIs).
        name: String,
        /// The container bytes (JSON array in v1 — see the module
        /// docs).
        bytes: Vec<u8>,
    },
```
What is wrong: `Vec<u8>` without `serde_bytes` serializes as a JSON array of integers — 2-6 bytes of text per media byte. The message rides the ABI input buffer, bounded at `MAX_INPUT_BYTES = 32 * 1024 * 1024` (abi.rs:91, which self-describes as "the effective media-size bound of the no-`unsafe` lane"). Effective media ceiling ≈ 5-15 MB. A typical 40 MB album file or 700 MB movie answers the typed overflow — the web host cannot load realistic media through its only blob lane. The e2e fixture is 425 KB (fits comfortably), so nothing in the test lane approaches the ceiling.
Why it matters: the web host's headline capability (open the user-selected file) silently maxes out at small files; the deferred raw lane ("the raw linear-memory blob lane is the H4 follow-up") does not exist in code (rg: no raw blob lane in abi.rs — only the RGBA frame lane).
What correct looks like: stage blob bytes raw through linear memory (the pattern `PresentFrame` already uses for RGBA via the ABI frame lane), or base64 with `serde_bytes`-style handling — and raise the ceiling accordingly.
Confidence: HIGH — Verification: read (serde Vec<u8> default behavior; abi.rs:91 bound; harness.js:188 `Array.from`).

F-002 [S4]
Where: apps/web/src/protocol.rs:112-125, 208-225, 409-416
Code:
```
    pub fn type_tag(&self) -> &'static str {
        match self {
            HostToWorker::Boot { .. } => "boot",
```
What is wrong: `type_tag` hand-duplicates the serde `rename_all="camelCase"` tags for 17 variants; the parity test pins only 2 (`pump`, `shutdownComplete`). A variant rename would silently drift the diagnostic tag from the wire tag.
Confidence: HIGH — Verification: read (test L410-416 covers 2/17).
What correct looks like: derive the tag from a serialized probe (`serde_json::to_value` → `["type"]`) in the test for every variant.

F-003 [S4]
Where: apps/web/src/protocol.rs:198-203
Code:
```
    ControlError {
        /// The stable dotted code (branchable, like `CommandError`'s).
        code: String,
```
What is wrong: stringly-typed control-error codes ("web.blob_registry", "web.abi.overflow", "web.worker.not_booted", "web.worker.output_backpressure") with no shared const/enum — the commands crate's typed-code discipline is not mirrored; a typo in a code string is uncatchable. Documented as the transport-level twin, so hygiene only.
Confidence: HIGH — Verification: read (codes scattered across abi.rs/host.rs call sites).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HostToWorker (enum) | 55-110 | REAL | — | 7 variants; serde internally tagged camelCase; all dispatched in host.rs. |
| HostToWorker::type_tag | 115-125 | REAL | S4 | Hand-duplicated tags (F-002). |
| WorkerToHost (enum) | 131-206 | REAL | — | 10 variants; frame bytes deliberately NOT here (raw ABI lane — correct). |
| WorkerToHost::type_tag | 211-225 | REAL | S4 | Same duplication. |
| PumpReport | 231-243 | REAL | — | state/position_us/busy/events_flushed — the re-arm contract; used by host.rs pump. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| downstream_wire_roundtrips | YES | Every variant through to_string/from_str with equality assert — any serde-shape drift fails. |
| boot_without_renderer_facts_decodes | YES | Pins the `#[serde(default)]` legacy form byte-exactly. |
| upstream_wire_roundtrips | YES | All 10 variants incl. a real EventKind payload and LaneTransition. |
| command_envelope_rides_the_transport_flat | YES | Asserts the merged flat JSON (`"type":"command"`, `"requestId":42`, `"cmd":"playback.pause"`). |
| completed_response_roundtrips | YES | Class-S payload through the Response arm. |
| type_tags_match_the_wire | WEAK | Only 2 of 17 tags pinned (F-002). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Round-trip fidelity is pinned by the tests ... every variant survives to_string/from_str exactly" | L40-43 | REAL | Baseline :3409-3414; round-trip tests cover every variant. |
| "The bytes ride the JSON contract in v1 (a serde byte array) — a raw linear-memory blob lane is the H4 follow-up" | L33-35 | REAL (and the confession is the defect) | F-001: the ceiling is ~5-15 MB effective; the follow-up lane does not exist. |
| "the catalog's 64 commands / 23 events are NOT re-defined here" | L20-23 | REAL | WireRequest/WireResponse/WireEvent re-used from commands crate (L46). |

## 7. Dead / duplicated / unreachable code
None dead in this file. Duplication: type_tag vs serde tags (F-002). Cross-file: the H4 staged-source lane (io/staged_source.rs) never reaches this contract either — no `http` open lane exists to carry it (see open.rs audit F-001).

## 8. Dependencies & flags
serde/serde_json only (the command layer's admitted wire family). No feature gates touch this file. Wasm32-clean.

## 9. Open questions for the next agent
- Does any doc/matrix row claim the web host loads arbitrary user media? If yes, the ~5-15 MB LoadBlob ceiling contradicts it (F-001).
- Is there a size-report entry (docs/web/size-report.md) that accounts for the JSON blob lane cost?
