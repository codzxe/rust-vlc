# AUDIT: crates/playback-engine/src/pipeline.rs#part1 (lines 1-667)
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 667 / 28 items (2 traits +5 methods, 3 enums +3 Display/Error impls, 4 structs +2 Defaults, 5 consts, 11 fns/methods, module docs) |
| File verdict | REAL (chunk part1: type/policy layer; executed via baseline) |
| Reachable from shipped entry point? | YES (call chain: `lumen` open path → apps/cli/src/host.rs:971 `queue_plan_from_product_limits` → apps/cli/src/demux_host.rs:89 `QueuePlan::from_product_limits` (pipeline.rs:325); seams implemented at apps/cli/src/pipeline_host.rs:235 (`HostDecoderProvider`) and :1228 (`CliFrameConsumer`)) |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:1 |
| Commands run | `rg -n "from_product_limits"` (callers: cli host only + tests) · `rg -n "impl DecoderProvider|impl FrameConsumer"` (prod impls in apps/cli/pipeline_host.rs, apps/web/host.rs) · `sed -n '300,345p' crates/packet-queue/src/limits.rs` + `od -c` (watermark predicate: low = all-axes) · `rg -n "queue_packets"` repo-wide (config wiring) · `rg -n "engine-queue" apps/cli/src/` → EXIT=1 (flag absent) · `rg -n "with_subtitle"` → only definition site · baseline log greps (see §2) |

## 1. One-paragraph verdict
Part1 is the pipeline's contract/policy layer: the `DecoderProvider`/`FrameConsumer` seams, the `QueuePlan` watermark policy, `TickBudget`, `TickReport`, and the `Limb` state record. The code is genuine, internally consistent, and unusually well-cross-referenced to design docs; the schema-bound constants are pinned against the committed schema by a real test that reads the file and asserts equality. The one substantial defect is a wiring lie at the product boundary: `from_product_limits` takes "product-shaped" integers, but the only shipped caller feeds it hard-coded mirror constants — product.toml's `limits.queue_packets_*` is parsed by the `product-config` crate and never reaches the engine, the UI setting `engine.queue-packets-audio/video` has no Rust consumer, and the generated settings docs advertise a CLI flag `--engine-queue-packets-audio` that does not exist. Second-order: the hard-coded 1 MiB/4 MiB byte caps make the schema's upper packet-cap range (up to 8192) unreachable for real-world packet sizes.

## 2. What actually works (verified)
— `queue_policy.rs` suite (7 tests incl. `from_product_limits_accepts_the_exact_boundaries`, `schema_constants_pin_the_engine_copies`, `thresholds_make_packets_the_starvation_axis`) ran and passed in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3953-3958` (suite ok; whole-run totals 4534/7/49 per that log's footer).
— Constants match the schema: `product.schema.json:192-193` carries `"queue_packets_audio": { "type": "integer", "minimum": 64, "maximum": 8192 }` / video 64..4096; engine constants (pipeline.rs:244-250) are 64/8192/4096 — verified by reading both files.
— Watermark degeneration claim verified: `crates/packet-queue/src/limits.rs:323-330` computes `low = usage.packets <= marks.low.packets && usage.bytes <= marks.low.bytes && usage.duration.0 <= marks.low.duration.0`; setting low.bytes/low.duration to their caps (pipeline.rs:411-412) makes those conjuncts tautological, so low is packets-only — the comment's claim is true.
— `WiredKind` has exactly Audio/Video/Subtitle variants (`crates/playback-engine/src/tracks.rs:156-162`), so `for_kind` (pipeline.rs:382-388) is total as documented.
— Baseline doctest for packet-queue ran ok (`orchrun-...log.txt:6451`); `packet_queue` unittests 17 tests at :3534-3536, `playback_engine` unittests 83 tests at :3740-3742.

## 3. Findings
### F-001 [S2]
Where: crates/playback-engine/src/pipeline.rs:309-328 (contract), evidence: apps/cli/src/demux_host.rs:64-73, apps/cli/src/host.rs:971-974, product.toml:203-204, schemas/generated/settings-docs.md:71
Code (pipeline.rs:318-320):
```
    /// `QueuePlan::default()` does not change when a host injects a
    /// plan: the library default (200/300 packets) and the product
    /// posture (1024/512) are different layers, visible by diff.
```
Code (apps/cli/src/demux_host.rs:71-73):
```
pub const PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024;
/// The video half of the same mirror.
pub const PRODUCT_QUEUE_PACKETS_VIDEO: u64 = 512;
```
Code (apps/cli/src/host.rs:971-974):
```
        config.queues = crate::demux_host::queue_plan_from_product_limits(
            crate::demux_host::PRODUCT_QUEUE_PACKETS_AUDIO,
            crate::demux_host::PRODUCT_QUEUE_PACKETS_VIDEO,
        )?;
```
What is wrong: `from_product_limits` is documented as consuming "the `limits.queue_packets_audio` / `queue_packets_video` values product.toml carries" — but the shipped CLI passes hard-coded mirror constants (1024/512), not values read from product.toml (`product.toml:203-204` declares them; `crates/product-config/src/model.rs:176` parses them; nothing forwards them to playback). Worse, `schemas/generated/settings-docs.md:71` exposes UI setting `engine.queue-packets-audio` ("-> product.toml limits.queue_packets_audio", range 64..8192) with CLI flag `--engine-queue-packets-audio` — `rg -n "engine-queue" apps/cli/src/` returns no matches (EXIT=1): the flag does not exist, and `queue_packets` has no consumer in apps/desktop-tauri, apps/desktop, or crates/settings-schema (only the generated ui/src schema.ts references it).
Why it matters: a user who tunes queue packet caps via product.toml, the settings UI, or the documented flag gets zero effect — the engine silently runs the hard-coded posture. This is "config declared but never read / UI settings wired to nothing" at the exact API whose doc promises the product join.
What correct looks like: the open path loads `limits.queue_packets_audio/video` through product-config (the crate already parses them) and passes those to `from_product_limits`; the settings-docs flag either exists in the CLI parser or is removed from generated docs.
Confidence: HIGH — Verification: read all five cited sites; rg exhaustions for `engine-queue` (apps/cli/src EXIT=1) and `queue_packets` consumers (only product-config parse + generated UI schema).

### F-002 [S3]
Where: crates/playback-engine/src/pipeline.rs:296-307
Code:
```
    pub fn with_subtitle(
        audio: QueueLimits,
        video: QueueLimits,
        subtitle: QueueLimits,
    ) -> QueuePlan {
```
What is wrong: `QueuePlan::with_subtitle` has zero callers in the entire repo (rg over crates/, apps/, tests/ matches only the definition at pipeline.rs:296).
Why it matters: dead public API presented as "card 3.A3's track-switching shape" — the subtitle lane in practice is only ever installed via `default_subtitle_limits()` inside `new`/`from_product_limits`; no shipped or test path ever sets explicit subtitle caps.
What correct looks like: either wire it into the track-switch/config path that needs explicit subtitle caps or demote it to the private module surface.
Confidence: HIGH — Verification: `rg -n "with_subtitle" crates/ apps/ tests/` → single hit (definition).

### F-003 [S3]
Where: crates/playback-engine/src/pipeline.rs:356-366
Code:
```
        Ok(QueuePlan {
            audio: QueueLimits {
                max_packets: nonzero(audio_packets as usize),
                max_bytes: nonzero(1 << 20),
                max_duration: media_types::Nanoseconds(1_000_000_000),
            },
```
What is wrong: the byte caps are hard-coded (audio 1 MiB, video 4 MiB) while the packet axis is configurable 64..8192. A queue configured at the schema ceiling of 8192 audio packets can never hold them: 1 MiB / 8192 = 128 bytes average packet before the byte cap binds. Real audio packets (AAC ~300-1500 B, PCM frames larger) bind the byte axis far below the packet cap the host selected.
Why it matters: the schema range advertises tuning headroom that the byte axis silently nullifies; a host raising `queue_packets_audio` to buffer more gets no more buffering (pushes are refused on the byte cap first). It is documented as `[PROPOSAL: until the Lead ratifies the byte-axis keys]`, which makes it honest debt, not fraud — but the two-axis interaction is a behavioral surprise the docs do not state.
What correct looks like: derive byte caps proportionally to the configured packet cap, or document the effective envelope (packets × avg-size vs fixed MiB) in the schema/settings help.
Confidence: HIGH — Verification: read; byte axis enforcement confirmed in packet-queue (`binding_axis` tests in baseline log :3537).

### F-004 [S4]
Where: crates/playback-engine/src/pipeline.rs:391-392 (doc) vs :401-405 (code)
Code (pipeline.rs:401-402):
```
        let high = QueueUsage {
            packets: limits.max_packets.get() - (limits.max_packets.get() / 4),
```
What is wrong: the doc says "high at three quarters of every cap", but `cap - cap/4` equals 3/4 only when cap is a multiple of 4 (e.g. 65 → 49 = 75.4%). All shipped caps (200/300/128/1024/512) land exact today, so no behavioral drift — a doc-precision nit. The pinning test `thresholds_make_packets_the_starvation_axis` (queue_policy.rs:312-337) asserts the same expression, so the doc and code cannot silently diverge further.
Why it matters: none today; the wording would mislead for a cap like 65.
What correct looks like: "high at cap minus a quarter (integer)".
Confidence: HIGH — Verification: read; arithmetic checked.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-99 | REAL | — | coherent tick/watermark/hold model; behavior claims verified where checkable (see §2); refs to doc 03 §4/§5 are design cross-refs |
| `trait DecoderProvider::decoder_for` | 120-133 | REAL | — | seam; prod impl apps/cli/src/pipeline_host.rs:235; Err = fatal typed selection failure (documented) |
| `trait FrameConsumer::accept` | 140-147 | REAL | — | prod impl CliFrameConsumer (pipeline_host.rs:1228), WebFrameConsumer (apps/web/src/host.rs:346) |
| `FrameConsumer::ready_for` (default) | 167-170 | REAL | — | `let _ = kind; true` — documented ungated default; ignored param is the contract, not a stub |
| `enum QueuePlanKind` + `Display` | 174-189 | REAL | — | trivial, correct |
| `enum QueuePlanError` + `Display` + `Error` | 194-238 | REAL | — | typed, names axis+bound; test `queue_plan_errors_display_the_axis_and_bound` passed in baseline |
| `QUEUE_PACKETS_FLOOR/_CEILING` consts | 244-250 | REAL | — | pinned to product.schema.json:192-193 by `schema_constants_pin_the_engine_copies` (test reads the schema file and asserts equality) |
| `DEFAULT_STARVE_DEBOUNCE_TICKS = 2` | 257 | REAL | — | not configurable; `[PROPOSAL: config key awaits Lead ratification]` — nothing declared in any config, so no dead-key defect |
| `QueuePlan::new` | 284-291 | REAL | — | test-only callers (7 test files); production goes through `from_product_limits` |
| `QueuePlan::with_subtitle` | 296-307 | REAL | S3 | F-002: zero callers |
| `QueuePlan::from_product_limits` | 325-370 | REAL | S2/S3 | validation correct + boundary-tested; wiring gap F-001; byte caps F-003; `audio_packets as usize` safe: validated ≤8192 before cast |
| `QueuePlan::for_kind` | 382-388 | REAL | — | total over WiredKind (verified 3 variants); test passed |
| `QueuePlan::thresholds` | 400-415 | REAL | S4 | F-004 rounding nit; low-bytes/low-duration never-binding verified against packet-queue evaluate (:323-330); test passed |
| `default_subtitle_limits` | 420-426 | REAL | — | const, PROPOSAL figures documented |
| `Default for QueuePlan` | 433-442 | REAL | — | 200/300 via packet-queue defaults (verified AUDIO_DEFAULT_PACKETS=200, VIDEO_DEFAULT_PACKETS=300, limits.rs:40,43) |
| `nonzero` | 448-453 | REAL | — | 0→`NonZeroUsize::MIN` fallback unreachable at all call sites (literals ≥128 or validated ≥64); documented |
| `TickBudget` + `Default` (64/16/8) | 458-477 | REAL | — | figures `[PROPOSAL: measured in Phase 3 benches]` — unbenchmarked but honest |
| `TickReport` struct | 484-545 | REAL | — | every field consumed (session.rs:800-802, :864, :2458, :2490; pipeline.rs parts 2-5 populate them — rg verified each field) |
| `Limb` struct | 547-627 | REAL | — | 13 pub + 3 private fields; private `last_frame`/`last_video_end_ns`/`conceal_failures` consumed in later chunks (pipeline.rs:2370+, 2495) — verified by rg |
| `VIDEO_CONCEALMENT_BUDGET = 1000` | 647 | REAL | — | shared both lanes, D-2 design documented; enforcement lives in parts 3-4 |
| `per_frame_damage` doc | 649-667 | — | — | CHUNK BOUNDARY: part1 ends inside this item's doc comment; fn signature at :668 belongs to part2 (A08) |

## 5. Test honesty (queue_policy.rs — the suite covering part1's API; passed in baseline :3953-3958)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `from_product_limits_accepts_in_range_and_rejects_typed` | YES | seeded domain sweep asserts accept/reject per schema range with typed variants |
| `from_product_limits_accepts_the_exact_boundaries` | YES | 64/8192/4096 edge values asserted (off-by-one would fail) |
| `schema_constants_pin_the_engine_copies` | YES | reads committed product.schema.json, extracts min/max, asserts equality with engine consts — drift in either direction fails it |
| `for_kind_is_exhaustive_and_honest` | YES | exhaustive match — adding a WiredKind variant without a lane breaks compilation, test asserts no silent audio fallback |
| `default_keeps_the_doc03_library_figures` | YES | asserts 200/300 packet defaults against packet-queue constants |
| `thresholds_make_packets_the_starvation_axis` | YES | asserts low.bytes==cap, low.duration==cap (never-binding), high==cap−cap/4, low<high — reverting the packets-only degeneration fails it |
| `queue_plan_errors_display_the_axis_and_bound` | YES | Display strings name axis+bound |
| Caveat | — | all seven are policy-shape tests; none execute a real queue tick (that coverage lives in seek_flow/session_flow, other agents' chunks) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the `limits.queue_packets_audio`/`video` values product.toml carries" feed `from_product_limits` | pipeline.rs:310-311 | FALSE (as wired) | host.rs:971-974 passes constants; product.toml:203-204 values never read by CLI (F-001) |
| constants duplicate product.schema.json:192-193, pinned by test | pipeline.rs:241-243 | REAL | schema read + test passed (baseline :3953+) |
| low watermark degenerates to packets-only | pipeline.rs:392-398 | REAL | packet-queue limits.rs:323-330 all-axes-low predicate + caps-at-low marks |
| high at "three quarters of every cap" | pipeline.rs:391 | REAL (rounded) | F-004: exact only for multiples of 4 |
| default 200/300 packets, 1 MiB/4 MiB | pipeline.rs:429-432 | REAL | packet-queue limits.rs:40,43 (200/300) + Default impl byte args |
| library default vs product posture "visible by diff" | pipeline.rs:318-320 | REAL | 200/300 (Default) vs 1024/512 (demux_host consts) |

## 7. Dead / duplicated / unreachable code
- `QueuePlan::with_subtitle` (296-307): zero callers — F-002.
- `QueuePlan::new`: reachable only from tests; retained as library API (acceptable, noted).
- No unreachable-from-CLI items: every other pub item has a production or test consumer (rg-verified).

## 8. Dependencies & flags
- Imports only workspace crates (codec-api, media-types, packet-queue, plugin-api) + std — matches boundaries. No feature gates on part1's items (plain items compiled in the default build that baseline #1 built clean).

## 9. Open questions for the next agent
- Part2 (A08): does `per_frame_damage` (:668+) match the doc's kind list (Corrupt, Decoder{Bitstream}) exactly, and does the concealment budget actually reset on Ok as the `conceal_failures` doc (:621-626) promises?
- Whoever owns apps/cli + product.toml: F-001's join — the `[PROPOSAL]` in demux_host.rs:70 says "when the product-config load lands"; check whether any wave found a landed loader that I missed (my rg says no).
- settings-schema owner: `engine.queue-packets-audio` UI row + `--engine-queue-packets-audio` flag — dead (F-001 evidence); settings-docs.md is generated, so the generator source needs the same fix.

## Chunk boundary note
Part1 = lines 1-667 exactly; line 667 is the last doc-comment line of `per_frame_damage`, whose `fn` starts at :668 → part2. No overlap claimed; Limb private fields consumed at :2370/:2495 were only located, not audited (they belong to parts 3-4).
