# CLAIM AUDIT: apps/web/README.md
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Claims extracted | 19 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 14 / 1 / 2 / 0 / 0 / 1 / 1 |
| Internal contradictions found | 4 |

## 1. Blunt verdict
This README is the most honest doc audited so far in this tree — it volunteers its own biggest defect (the shipped wasm module decodes nothing) in a bold bullet, labels every browser behavior `[VERIFY]`, and its layout table matches the code file-for-file. Its failures are staleness and one wiring overstatement: the catalog counts are wrong (64/23 vs the real 65/26), the Network bullet says the 7.H4 fetch lane "landed" when 2,823 loc of it are unreachable from the host's open path (http open still refuses), and the evidence paragraph folds "the mock-fetch playback" into "the SAME message paths the JS glue drives" — which is false for the fetch lane. Nothing here is fabricated; the gaps are drift.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "the same std-only engine core the CLI drives (probe → demux → engine → queues → clock), exposed to a Web Worker" | README:3-5 | REAL | host.rs wires probe/demux/Engine/clock; `cargo test -p lumen-web --lib host::tests` → 14 passed |
| 2 | "64 commands / 23 events ride the `crates/commands` catalog" | README:6-7 | FALSE | `rg -c "^\s+CommandSpec \{" crates/commands/src/catalog/spec.rs` → 65 (table fn: "61 rows + the issue #32 bookmark trio + the w31.1 frame-channel bind", spec.rs:161-162); EventSpec count → 26 ("23 rows + the issue #42 additive trio", spec.rs:698-699) |
| 3 | "a hand-rolled `extern "C"` ABI over shared linear memory with **zero new dependencies**" | README:8-9 | REAL | abi.rs externs L348-401; direct externals are only serde+serde_json (apps/web/Cargo.toml:70-71), both already the command layer's family (crates/commands/Cargo.toml:39-40) — "zero" holds only under that reading |
| 4 | "the raw frame lane (`lumen_web_frame_addr`/`len`) stages present bytes apart from the JSON" | README:20 | REAL | abi.rs:391-401 exports; worker.js:97-105 reads them; `the_present_path_ships_one_frame_per_pump` passed |
| 5 | "`WebHost`: the engine, the cooperative pump/slice budgets …, the render lane + present mailbox + autoplay gate wiring, the served command surface" | README:21 | REAL | host.rs:1120-1138 (pump), 1145-1166 (present), 1171-1176 (autoplay), 762-775 (SERVED); tests ran 14/14 |
| 6 | "the backend negotiation state machine (WebGPU → WebGL2 → honest `unavailable`), the DPR-aware canvas seam, and the CPU RGBA upload path + latest-wins present mailbox" | README:22 | REAL | render/backend.rs:183-210 negotiate + tests passed; render/present.rs store/take with MAX_UPLOAD_BYTES=32MiB bound |
| 7 | "`src/io/` — Card 7.H4 …: the fetch seam (`FetchPort`), seekability detection, and the staged bounded-window source" | README:24 | UNREACHABLE | modules exist (io/ = 2,823 loc) but `rg "crate::io|FetchPort|StagedSource" apps/web/src/{host,abi,open}.rs` → 0 hits; open.rs:182-186 refuses http ("network streaming is card 7.H4"); no fetch messages in protocol.rs |
| 8 | "worker.js … drains the JSON output lane and the raw frame lane, re-arms the pump" | README:28 | REAL | worker.js:74-116 (send → dispatch → output drain → frame slice → setTimeout re-arm on `pumped.busy`) |
| 9 | "audio-worklet.js … the audited transcription of the tested `WorkletQueue` model (bounded queue, 128-frame quantum, silence-on-underrun, played counter)" | README:29 | REAL-UNPROVEN | file exists (9,012 B, matches size-report.md:100); header constants pin sink.rs (WORKLET_QUANTUM=128, queue 8192, prebuffer 256); line-by-line transcription equivalence and real-AudioWorklet behavior unverified (no browser, no audio in sandbox) |
| 10 | "cargo build -p lumen-web --target wasm32-unknown-unknown --locked" (builds the module) | README:33-35 | REAL | baseline rc-smoke.txt addendum: `Finished dev profile [unoptimized] target(s) in 11.45s`; artifact on disk: `file target/.../lumen_web.wasm` → WebAssembly, 4,243,806 B |
| 11 | "the glue degrades to array-buffer instantiation — honest, not dead — when the MIME is wrong" | README:40-42 | REAL | worker.js:147-155 implements exactly the streaming/arrayBuffer split on content-type |
| 12 | "No COOP/COEP headers are needed … the boot handshake's capability snapshot says `threading: \"single\"`" | README:42-44 | REAL | capabilities.rs:37-52 Threading::Single; `capabilities::tests` 5 passed (snapshot_serializes asserts `"threading":"single"`) |
| 13 | "The page probes `navigator.gpu` / `getContext(\"webgl2\")` BEFORE sending `boot`" | README:44-46 | REAL | as a contract: harness.js:160-161 probes both before boot; caveat — no product page in-repo implements it (see contradictions #4) |
| 14 | "the wasm32 module ships the engine core WITHOUT the builtin decoders … Native builds link the real MJPEG + PCM decoders" | README:56-60 | REAL | host.rs:477-486 refusal arm; `wasm32_profile_decoder_seam_refuses_typed` + `decoder_claims_match_the_profile` passed; native host tests decode the real fixture |
| 15 | "the typed render seam, the backend negotiation ladder and the CPU RGBA upload path landed (7.H2) … all headless-proven. The BROWSER halves … are card 7.I1's `[VERIFY]` matrix" | README:63-70 | REAL | loss-ladder + present tests passed; capabilities.rs:280-289 blocked row states the browser present is 7.I1 `[VERIFY]` |
| 16 | "**Network**: the fetch seam + seekability detection + staged source landed (7.H4), proven against the mock double" | README:71-73 | SHALLOW | the modules are mock-proven in their own tests, but "landed" misleads: the host cannot open any network source — open.rs:182-186 refuses http/https, and the e2e harness itself asserts "http open must refuse (the staged fetch lane is not wired to open)" (harness.js:513). The lane is unreachable from every shipped entry point |
| 17 | "**Seek**: refused typed (the seek boundary's host duties land with the H-pillar pump integration)" | README:75-76 | REAL | host.rs owner_of:1837-1839; `unserved_surface_refuses_typed` passed (code UnsupportedForHost, detail `playback.seek`) |
| 18 | "Proven headless-native by `cargo test -p lumen-web` (the SAME message paths the JS glue drives … the mock-fetch playback)" | README:80-84 | SHALLOW | 111 tests passed (baseline cargo-test-workspace.txt:3443; I re-ran 24). But "the mock-fetch playback" does NOT ride the host's message paths — it lives in io module tests the host never invokes (see #16); the parenthetical overstates the evidence for the fetch lane |
| 19 | "Browser-runtime behavior … is `[VERIFY]` — no browser exists in this sandbox, and nothing here claims it was exercised" | README:85-89 | REAL | consistent with `[VERIFY]` labels in host.rs:67-69, capabilities.rs, render/mod.rs; no browser claim found in the crate |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "64 commands / 23 events ride the catalog" (README:6-7) | spec.rs:161-162 ("61 rows + the issue #32 bookmark trio + the w31.1 frame-channel bind") and :698-699 ("23 rows + the issue #42 additive trio") | The catalog: 65 CommandSpec rows, 26 EventSpec rows (counted in-table via awk/rg). The README is stale on both numbers |
| "the fetch seam + seekability detection + staged source landed (7.H4)" (README:71-72) | open.rs:182-186: http answers "network streaming is card 7.H4 (fetch + Range seekability detection)"; harness.js:513: "the staged fetch lane is not wired to open" | The code: the io pillar exists as libraries but the host's open path refuses every network source; nothing dispatches a fetch message |
| "this build's own output lanes remain the headless counting presenter and the null audio device" (README:69-70) | Same bullet, two lines earlier: "the CPU RGBA upload path landed (7.H2)"; host.rs tests ship `PresentFrame` RGBA over the port | Both half-true: the video CONSUMER counts (headless presenter), but the present mailbox + `PresentFrame` message lane DID land and ships frames (test `the_present_path_ships_one_frame_per_pump`). The "remain" sentence — and engine_info's stale feature tag `video.headless-counter-presenter` (host.rs:721) — predate 7.H2 |
| "Serve `worker.js`, `audio-worklet.js` and `lumen_web.wasm` … The page probes … owns the `AudioContext`" (README:38-47) — implies a serving page | `rg "apps/web|worker\.js|lumen_web" ui/src` → only a comment (transport.ts:37); the sole in-repo consumer is the e2e test harness (ui/e2e/web/site/harness.js) | No product page exists: the React UI (ui/) does not instantiate the wasm worker; its canvas video path is the DESKTOP LUF1 frame channel (ui/src/client/frame-channel.ts). The README documents an integration nobody ships yet |

## 4. Fabrication / inflation indicators
- Stale counts, not fabrication: "64 commands / 23 events" were true before the w31.1 frame-channel bind and the issue #42 event trio landed; nobody updated the README.
- Evidence-scope inflation (one instance): folding "the mock-fetch playback" into "the SAME message paths the JS glue drives" (README:80-84) — the fetch lane never rides the host dispatch (proof in #16/#18).
- "zero new dependencies" (README:9) survives only with the Cargo.toml header's own qualification ("adds none beyond the command layer's own serde wire family", apps/web/Cargo.toml:31-34); the bolded absolute is unqualified in the README.
- Notably ABSENT inflation: the decoder-less wasm module, the null audio device, the unwired browser halves, and the refusal of seek are all volunteered by this README — the doc under-claims where most docs in this tree over-claim.

## 5. Matrix rows
N/A — this is a component README, not a feature-matrix chunk.
