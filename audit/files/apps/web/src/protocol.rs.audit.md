# AUDIT: apps/web/src/protocol.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-21 |
| Lines / functions | 417 / 11 items (2 enums [7+10 variants], 2 `type_tag` fns, 1 struct, module docs, 6 tests) |
| File verdict | UNREACHABLE (real, wire-tested, ABI-wired contract; nothing shipped loads the wasm module it serves) |
| Reachable from shipped entry point? | NO from `lumen` CLI or tauri. In-crate wiring is real: `lib.rs:74` re-exports it; `abi.rs:193,217` deserializes `HostToWorker` from the ABI's JSON word lane; `apps/web/worker.js` implements the JS twin (verified: `presentFrame` raw-frame lane worker.js:103-105, `pumped.busy` re-arm :112-114). The wasm artifact builds (`audit/runs/baseline/rc-smoke-engine-only.log.txt:25,62-64` — `cargo build -p lumen-web --target wasm32-unknown-unknown` PASS, 38,062,701 bytes debug, `\0asm` magic; release floor 1,302,846 B / 301,101 B gzip per `gate-web_budget___check.log.txt`). But the ONLY consumers of the module are the e2e harness (`ui/e2e/web/site/harness.js:85` `new Worker("worker.js")`), the web-matrix lane (never run in baseline), and rc-smoke step 8 (SKIPPED in baseline: `RC_SMOKE_SKIP_WEB=1`). `ui/src/client/ClientProvider.tsx:6` — the shipped React UI forbids wasm imports and runs MockTransport; `rg "lumen_web\|WebAssembly\|worker.js"` in apps/desktop-tauri + apps/desktop → 0 hits. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `rg -n "HostToWorker\|WorkerToHost\|PumpReport" apps/web/src apps/web/worker.js` (wiring: lib.rs:74, abi.rs:193/217/407+); `rg -n "presentFrame\|pumped\|loadBlob\|blob:" apps/web/worker.js ui/e2e/web/site/harness.js` (JS-side contract matches); `rg -ln "lumen_web\|WebAssembly\|worker.js" apps/desktop-tauri apps/desktop ui/src` (0 — nothing shipped loads it); `rg -n "wasm-web-build\|lumen-web" audit/runs/baseline/rc-smoke-engine-only.log.txt` (wasm build PASS :25-64); `cat audit/runs/baseline/gate-web_budget___check.log.txt` (floor rebuilt 301,101 B gzip; PASS with sha drift note); `rg -n "lumen_web" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (:3087-3202 — 111 tests green incl. all 6 `protocol::tests` :3168-3173); `rg -n 'rename = "' crates/commands/src/catalog/{command,event}.rs -c` (65/26 — "64 commands / 23 events" doc claim ballpark); Read tool for worker.js quotes (sed in this sandbox eats `[m` sequences — verified: `sed -n '112p'` rendered `messagesessages` where the real line is `messages[messages.length - 1]`; all JS quotes below are Read-tool verbatim). |

## 1. One-paragraph verdict
A clean, small, genuinely two-sided wire contract: 7 downstream and 10 upstream MessagePort messages, camelCase `"type"`-tagged, riding the commands crate's own envelopes rather than redefining them, with round-trip tests that cover all 17 variants and a legacy-decode test for the bare `{"type":"boot"}` shape. The contract is not paper: the wasm ABI lane parses it (abi.rs:193,217) and the hand-written worker.js implements the same strings (`presentFrame`, `pumped`, `loadBlob`), and 111 native lumen-web tests are green including the full Boot→LoadBlob→Command→Pump→Shutdown walk over the ABI. Two real defects: nothing pins 15 of the 17 wire tags against drift — the `type_tag()` helpers and the serde tags are separately-written strings that today agree but would silently diverge under a rename (round-trip tests are self-consistent by construction and cannot catch it; the JS glue would break at runtime in a browser, which no baseline run ever exercised); and the `loadBlob` lane ships media bytes as a JSON number array — ~4 characters of JSON per payload byte — which the e2e harness itself uses to push the whole first-vertical fixture. And the standing domain answer: this module compiles into a wasm artifact that builds and is budgeted (301 KB gzip floor) but is loaded by no shipped product — the React UI explicitly forbids wasm imports, and the tauri shell references nothing from apps/web.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3087-3202` → lumen-web 111 tests green, including `protocol::tests::*` all 6 (:3168-3173), `abi::tests::the_lane_proof_over_the_abi` (:3128 — Boot → LoadBlob → Command → Pump loop → Shutdown through the raw extern "C" ABI), and `host::tests::pump_slices_play_the_fixture_to_its_end` (:3119).
— Verified by `audit/runs/baseline/rc-smoke-engine-only.log.txt:25,62-64` → `cargo build -p lumen-web --target wasm32-unknown-unknown --locked` PASS, artifact 38,062,701 bytes (debug), `\0asm` magic checked by the script.
— Verified by `audit/runs/baseline/gate-web_budget___check.log.txt` → the release floor was REBUILT during the gate (raw 1,302,846 B, gzip 301,101 B, 9.6% of ceiling) — the wasm module exists at release size, decoder-less by design (apps/web/Cargo.toml: wasm32 codecs edge is opt-in, "the SHIPPED module enables none of them").
— Verified by reading worker.js:62-115 against the enum: `send` pads JSON to 4-byte words → `lumen_web_write_u32`/`lumen_web_dispatch` → `pumpOutputs` reads the staged batch, attaches the RAW frame buffer to the single `presentFrame` (103-105, transferred not copied), and re-arms the pump on `last.report.busy` (112-114) — the exact `Pump { ticks }`/`Pumped { report }` contract this file defines, including the "ONE frame per pump message" doc claim.
— Verified `boot_without_renderer_facts_decodes` (291-295): the `#[serde(default)]` legacy lane decodes `{"type":"boot"}` → `Boot { renderer: None }` — old-glue compatibility is real, not asserted-only.

## 3. Findings

### F-001 [S3]
Where: apps/web/src/protocol.rs:408-416 (with 112-126, 208-225)
Code:
```
    /// The type-tag helpers name the wire tags (diagnostic parity).
    #[test]
    fn type_tags_match_the_wire() {
        assert_eq!(HostToWorker::Pump { ticks: 0 }.type_tag(), "pump");
        assert_eq!(
            WorkerToHost::ShutdownComplete.type_tag(),
            "shutdownComplete"
        );
    }
```
What is wrong: The wire tag is produced by serde (`#[serde(rename_all = "camelCase", tag = "type")]`, lines 54, 130) and separately by the hand-written `type_tag()` matches (115-124, 211-223). These are two independent sources of the same 17 strings, and the consumer is string-matching JavaScript: worker.js:103 `message.type === "presentFrame"`, :113 `last.type === "pumped"`; harness.js matches `blobLoaded`, `booted`, etc. The round-trip tests (255-286, 299-374) cannot catch tag drift — a renamed tag round-trips against itself. The only tests pinning ACTUAL tag strings are: `"type":"command"` (387), `"type":"boot"` (293), and this test's 2 of 17 tags. A rename of, say, `SurfaceLost`'s serde tag, or a `type_tag` string edit, compiles green and passes the whole suite while breaking the JS glue at runtime — in the browser lane that baseline never ran (rc-smoke step 8 SKIPPED, web-matrix lane not in baseline).
Why it matters: The contract this file exists to define is the Rust↔JS agreement; 15/17 of its tags have no test guarding the exact string on both sides. The `type_tag` functions' entire stated purpose ("diagnostic parity") is unverified for 15 of 17 cases.
What correct looks like: One test that serializes every variant and asserts the JSON contains the `type_tag()` string (`serde_json::to_value(msg)["type"] == msg.type_tag()` for all 17) — four lines, closes the hole.
Confidence: HIGH — Verification: read + enumerated the 4 tag-pinning assertions across the suite vs the 17 variants.

### F-002 [S3]
Where: apps/web/src/protocol.rs:69-75 (with module docs 27-35)
Code:
```
    /// Pre-stage one blob's bytes (doc 09 §6.1's File/Blob no-upload
    /// path — the file never leaves the page).
    LoadBlob {
        /// The registry name (referenced by `blob:<name>` URIs).
        name: String,
        /// The container bytes (JSON array in v1 — see the module
        /// docs).
        bytes: Vec<u8>,
    },
```
What is wrong: Media container bytes ride the protocol as `Vec<u8>` under `serde_json` — i.e. a JSON array of one number per byte (~4.2 chars/byte). The whole file-prestage path pays it: worker.js `JSON.stringify`s the message (worker.js:123), the wasm ABI parses the JSON text (abi.rs:193), serde_json materializes the Vec. For the first-vertical AVI fixture (~425 KB — the size `BlobLoaded { size: 425_516 }` in the test at 319 quotes) that is a ~1.8 MB JSON string, a full stringify + parse + Vec alloc per file. The e2e harness drives exactly this lane (harness.js:188 `ctx.lane.post({ type: "loadBlob", name: "fixture", bytes: Array.from(bytes) })` — also materializing a JS number array). The module doc (33-35) honestly defers the raw linear-memory lane to "the H4 follow-up" — the debt is acknowledged, but v1's only File/Blob path ships the 4x form.
Why it matters: This is the web target's user-facing "open a local file" lane; every byte of the user's media is copied three extra times and inflated 4x before the demuxer sees it. On multi-hundred-MB files this is a wall, not a wrinkle.
What correct looks like: Either land the H4 raw lane (stage bytes via the existing `lumen_web_write_u32` word lane or a bulk-memory import) or, minimally, base64 in the JSON (~1.37x instead of ~4.2x) with the registry storing decoded bytes.
Confidence: HIGH — Verification: read of the type, worker.js:123 send path, harness.js:188; test value 425_516 (protocol.rs:319) fixes the realistic payload size.

## 4. Per-function table (417 lines, whole file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module docs | 1-43 | REAL | F-002 | Honest debt markers ([PROPOSAL blob lane], H4 follow-up); "64 commands / 23 events" count NOT re-verified here (commands crate census: 65/26 serde renames incl. nested — commands auditor's row). |
| `HostToWorker` enum (7 variants) | 52-110 | REAL | — | Boot (defaulted renderer — legacy decode tested), LoadBlob (F-002), Command (flat envelope merge — tested 381-390), Pump, SurfaceLost, AudioContext, Shutdown. camelCase+tag wire. |
| `HostToWorker::type_tag` | 112-126 | REAL | F-001 | 7/7 strings match serde derivation today; unguarded (see F-001). |
| `WorkerToHost` enum (10 variants) | 128-206 | REAL | — | Booted (EngineInfo + HostCapabilities), BlobLoaded, Response, Events, Pumped, PresentFrame (pts_ns Optional), RenderDegraded, GestureRequired, ControlError (dotted code), ShutdownComplete. |
| `WorkerToHost::type_tag` | 208-225 | REAL | F-001 | 10/10 strings match today; 2 of 17 total pinned by tests. |
| `PumpReport` struct | 227-243 | REAL | — | state/position_us(i64 µs)/busy/events_flushed(u32) — consumed by host re-arm loop (worker.js:113 `last.report.busy`, host.rs SliceBudgets). |
| tests: `downstream_wire_roundtrips` | 254-286 | REAL | — | All 7 variants round-trip; would fail on any field/type drift. |
| tests: `boot_without_renderer_facts_decodes` | 291-295 | REAL | — | Legacy `{"type":"boot"}` pinned. |
| tests: `upstream_wire_roundtrips` | 299-374 | REAL | — | All 10 variants round-trip incl. full EngineInfo/Event shapes. |
| tests: `command_envelope_rides_the_transport_flat` | 380-390 | REAL | — | Pins the flat `{type, requestId, cmd}` merge (doc 05 §5.1 shape). |
| tests: `completed_response_roundtrips` | 394-406 | REAL | — | class-S payload + `"completed"` tag pinned. |
| tests: `type_tags_match_the_wire` | 409-416 | SHALLOW | F-001 | 2 of 17 tags. |

## 5. Test honesty (all 6 green: orchrun run :3168-3173)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `downstream_wire_roundtrips` | YES | Per-variant assert_eq on the round-tripped value with the JSON in the failure message (284). |
| `boot_without_renderer_facts_decodes` | YES | Removing `#[serde(default)]` (64-65) fails 293. |
| `upstream_wire_roundtrips` | YES | Same shape; covers every upstream arm incl. PumpReport fields. |
| `command_envelope_rides_the_transport_flat` | YES | A wrapper-nesting regression (double `type` tag) fails 387-389. |
| `completed_response_roundtrips` | YES | result_wire adapter drift fails 404-405. |
| `type_tags_match_the_wire` | NO for its stated purpose | Renaming 15 of 17 serde tags or type_tag strings leaves it green (F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "typed messages both directions over the port, with the catalog's own envelopes riding inside" | protocol.rs:1-3 | REAL | WireRequest/WireResponse/WireEvent from commands (45-46); flat-merge test 380-390. |
| "Round-trip fidelity … every variant survives to_string/from_str exactly" | protocol.rs:40-43 | REAL | 17/17 variants in the two round-trip tests; green :3168-3173. |
| "The catalog's 64 commands / 23 events" | protocol.rs:20 | REAL-UNPROVEN | Not re-verified here; commands crate rename census 65/26 (incl. nested enums) — within noise, commands-crate auditor owns the exact count. |
| "the bytes ride the JSON contract in v1 … a raw linear-memory blob lane is the H4 follow-up" | protocol.rs:33-35 | REAL (honest debt) | F-002 quantifies the v1 cost; harness.js:188 uses this exact lane. |
| PresentFrame: "ONE frame per pump message … bytes stage RAW in linear memory … never JSON-encoded" | protocol.rs:163-167 | REAL | worker.js:90-105 verified (raw frame read + transfer); the wasm side is abi.rs's lane (abi::tests::the_frame_lane_stages_the_present_bytes green, :3091). |
| GestureRequired "transport-level twin" `[PROPOSAL]` | protocol.rs:183-191 | REAL-UNPROVEN | Park/resume logic is host.rs/autoplay lane (autoplay tests green :3094-3098); the catalog-row twin is a Lead decision still pending. |
| (domain) "What does apps/web really build and does anything ship it?" | wave-12 hostile question | BUILDS, SHIPS NOWHERE | wasm build PASS (rc-smoke :64), budget PASS (gate log); consumers: e2e harness + skipped smoke lane only; UI forbids wasm imports (ClientProvider.tsx:6); tauri/desktop reference nothing (rg 0 hits). |

## 7. Dead / duplicated / unreachable code
- No dead items in this file — every variant has a producer or consumer in-crate (abi.rs lane proof) and the JS twin.
- `type_tag()` fns: production consumers are diagnostics/tests only; their drift risk is F-001.
- UNREACHABLE from shipped entry points (header) — the module it serves is an unshipped artifact.

## 8. Dependencies & flags
- Deps: commands (envelopes), serde/serde_json, crate::capabilities + crate::render::backend types — all in-crate or workspace; no new externals (matches the crate's "ZERO new dependencies" claim in Cargo.toml).
- wasm32 codec lanes are feature-gated OFF in the shipped module (apps/web/Cargo.toml) — the protocol is codec-agnostic, unaffected.

## 9. Open questions for the next agent
- abi.rs auditor (W12 parts): confirm the frame-lane invariant (staging buffer valid until next dispatch) matches worker.js's read-before-any-wasm-call discipline (worker.js:84-100) — both sides claim it, neither cross-tests it.
- rc-smoke/web-matrix owners: the browser-load lane (step 8 / playwright.web.config.ts) has NO baseline run — the only true end-to-end web proof (real browser, real Worker, real wasm) is UNVERIFIED. REQUEST-ORCH-RUN: `bash scripts/rc-smoke.sh` with chromium available (steps 7-8 unskipped).
- web-budget: the floor row sha256 in docs/web/size-record.toml DRIFTS from the fresh rebuild (gate log note: "the record predates this build") — same evidence-drift class as the SBOM finding (baseline #15). Owner: whoever audits docs/web/size-record.toml.
