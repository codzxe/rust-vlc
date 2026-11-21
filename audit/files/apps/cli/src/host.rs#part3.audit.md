# AUDIT: apps/cli/src/host.rs#part3 (lines 1293–1940)
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2026-11-21 |
| Lines / functions | 648 / 26 (5 CliHost methods, 12 free fns, 2 test modules with 9 items) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` main → run.rs grammar → dispatch.rs arms → `host.media_info` dispatch.rs:133, `host.duration` :135, `host.current_time` :134, `host.setting_get` :149, `host.setting_set` :152, `crate::host::engine_info` :140, `crate::host::command_listings` :144; free helpers called from host.rs parts 1–2 at :303, :656-668, :885, :864, :984, :1108, :1139) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `rg -n "^impl |^fn |^mod " apps/cli/src/host.rs` (structure map; part1=1-646 A09, part2=647-1292 A08, part3=1293-1940 mine); `rg -n "<helper>" apps/cli/src/*.rs` for every function (all have live callers, listed above); `rg -n '"[a-z0-9_]+"' apps/cli/src/pipeline_host.rs` (12+ real decoder arms vs engine_info's 2); `cat audit/runs/baseline/lumen-engine-info.txt` (hard-coded values are the shipped output); `cat audit/runs/baseline/gate-lint_literals.log.txt gate-check_identity.log.txt` (both FAIL on host.rs:1799); `rg -n "lumen_cli|audio_limb|backpressure" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (tests ok :2868-2872); `sed -n '585,595p' crates/demux/src/mkv/header.rs` + `sed -n '210,216p' apps/cli/src/containers/mkv.rs` (default_duration_ns has no zero guard); `sed -n '950,970p' crates/demux/src/avi/mod.rs` (SeekSupport::None exists); `sed -n` on every quoted range (verbatim verified) |

## 1. One-paragraph verdict
Chunk boundaries: part3 spans host.rs:1293-1940 per the assignments.tsv size split (A09 part1=646, A08 part2=646, A10 part3=648); `media_info`'s signature and guard (1283-1292) fall in part2 — I audited its `MediaInfo` construction (1293-1314) which is inside my range and note the overlap. The chunk is genuine, reachable, and executed: the class-S snapshot answers (`media_info`, `duration`, `current_time`), the settings seam (`setting_get`/`setting_set`), the wire-mapping helpers, `engine_info`, `command_listings`, and two real test modules all have live callers and passed in the orchestrator's no-fail-fast run (:2868-2872). The defects: `engine_info` is the known standing lie — a frozen first-vertical-era capability block that under-reports the real 12+ decoder registry and marks its two listed decoders `active: false` (baseline CORRECTION v2 already burned Wave-0 on trusting it); `media_info` hard-codes `seekable: true`/`is_live: false` and `current_time` fabricates a full-range `buffered` array; and line 1799 is the single site keeping BOTH repo identity gates red (baseline #13/#14).

## 2. What actually works (verified)
- All 5 tests of my two test modules → `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2868-2872` (`host::audio_limb_seam_tests::*`, `host::session_backpressure_tests::*`) — the backpressure pair exercises the real `open_session` over a generated 6-second WAV and asserts exact ring-fill bounds and wall-clock sample counts.
- `lumen engine-info` executed (baseline `lumen-engine-info.txt`: version 0.1.3, host cli, features/decoders strings) and `commands` listing (`command_surface.rs::commands_lists_the_whole_catalog ... ok`, no-failfast log) — the two free functions in this chunk run in the shipped binary.
- Settings seam end-to-end: `run.rs::settings_get_set_through_the_grammar` + `dispatch.rs::settings_seam_validates_through_the_schema` ok in the lumen_cli lib run (no-failfast :2769-2790 region, 129 tests).

## 3. Findings
### F-001 [S2]
Where: apps/cli/src/host.rs:1631-1661
Code:
```
pub fn engine_info() -> EngineInfo {
    EngineInfo {
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        catalog_version: "1".to_string(),
        protocol_version: "unwired (control-protocol is a later card)".to_string(),
        host: "cli".to_string(),
        features: vec![
            "first-vertical:avi-mjpeg-pcm".to_string(),
```
(continuing through)
```
        decoders: vec![
            DecoderInfo {
                name: "mjpeg".to_string(),
                kind: TrackKind::Video,
                hardware: false,
                backend: None,
                active: false,
            },
```
What is wrong: The entire capability block is a hard-coded literal frozen at the first-vertical era. The real provider registry (`apps/cli/src/pipeline_host.rs:242-346`) matches mjpeg, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, av1/av01, vp9, dnxhd, h264/avc1 (+ PCM) — engine_info names exactly two (`mjpeg`, `pcm_s16le`) and stamps both `active: false`. `features` claims only `first-vertical:avi-mjpeg-pcm` while the same binary plays MP4 H.264 4:2:0, ADTS AAC, Ogg Vorbis/Opus, MKV, TS (baseline CORRECTION v2 + container_conformance 19/19). `protocol_version` carries the stale prose "control-protocol is a later card" — crates/control-protocol exists and apps/control-server ships it (`apps/control-server/src/main.rs:21-23` imports `control_protocol::PROTOCOL_VERSION`). `sessions` is unconditionally empty (the function takes no state).
Why it matters: `lumen engine-info` is the shipped capability surface; its output is false in both directions (under-reports 12+ decoders, marks the two it names inactive). Baseline history proves the blast radius: Wave-0 trusted this output and headlined "no H.264 decoder in this build" — wrong (CORRECTION v2). Any script or user probing capability today gets the same lie.
What correct looks like: Derive `decoders` from the live `HostDecoderProvider` registry, `features` from the real shipped lanes, `protocol_version` from `control_protocol::PROTOCOL_VERSION` (or an honest "n/a" for the CLI surface), and drop or populate `sessions` from host state.
Confidence: HIGH — Verification: read + baseline cites (lumen-engine-info.txt; INDEX CORRECTION v2 "Standing defects"; pipeline_host.rs arms counted by rg).

### F-002 [S3]
Where: apps/cli/src/host.rs:1307-1308 (same class at 1351-1357)
Code:
```
                seekable: true,
                is_live: false,
```
and
```
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
            None => Vec::new(),
        };
```
What is wrong: `media.get-info` hard-codes `seekable: true` and `is_live: false` regardless of the demuxer's actual `SeekSupport` (AVI honestly reports `SeekSupport::None` when sticky/not-started — crates/demux/src/avi/mod.rs:956-964; a mid-session damaged session keeps its facts, so media_info answers seekable on a demuxer that answers None), and the http lane (`open_http_source`, RangedResource with linear-scan no-range fallback) is a live-ish stream reported `is_live: false` with a fabricated full-range `buffered` `[0, duration]`.
Why it matters: A UI or script gating seek/live/buffering on `media.get-info` gets wrong answers for exactly the degraded inputs where the answer matters (damaged index, no-range HTTP). The truthful source (`SelectDemuxer::seekability()` forwarding the inner demuxer) exists and is one call away.
What correct looks like: `seekable: matches!(seekability, SeekSupport::Fast{..} | SeekSupport::SlowLinear)`, `is_live` derived from the source lane, `buffered` either omitted for files or derived from the ranged resource's cached windows.
Confidence: HIGH — Verification: read (SeekSupport::None producer verified at avi/mod.rs:956-964; http lane verified at demux_host.rs:422-436 region).

### F-003 [S3]
Where: apps/cli/src/host.rs:1798-1799
Code:
```
        let path = std::env::temp_dir().join(format!(
            "lumen-backpressure-{}-{}s.wav",
```
What is wrong: The literal embeds the product identity token and carries no `// lumen-allow-literal:` annotation. This single site is the ONLY violation keeping both repo identity gates red: baseline #13 `cargo xtask lint-literals` → "FAIL — 1 violation(s): apps/cli/src/host.rs:1799: identity — identity(identity.name)" and baseline #14 `cargo xtask check-identity` → same (gate-lint_literals.log.txt / gate-check_identity.log.txt, EXIT=1 each).
Why it matters: Two shipped gates fail on main because of this line. The repo's own convention for exactly this shape exists — `crates/testkit/src/lib.rs:475` is allow-listed as "cfg(test) scratch filename — test-only, never a shipped surface" — so the fix is a one-line annotation (or dropping the token from the scratch name).
What correct looks like: `// lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface` above line 1799 (mirroring testkit lib.rs:475), or a token-free name.
Confidence: HIGH — Verification: read + baseline gate logs #13/#14 (both FAIL on exactly this line).

### F-004 [S3]
Where: apps/cli/src/host.rs:1452
Code:
```
    if let Some((width, height)) = fact.dimensions {
        info.video = Some(commands::catalog::types::VideoTrackInfo {
            width,
            height,
            frame_rate: fact.frame_ns.map(|ns| (1.0e9 / ns as f64) as f32),
```
What is wrong: No zero guard on `frame_ns`. `1.0e9 / 0.0` is `inf` in Rust (no panic), so a track fact carrying `frame_ns = Some(0)` reports `frame_rate: inf` in `media.get-info`. The MKV lane can mint that fact: `default_duration_ns` is read verbatim from the EBML uint with no zero check (crates/demux/src/mkv/header.rs:589-591) and passed through unguarded (apps/cli/src/containers/mkv.rs:212-214) — a malformed `DefaultDuration = 0` TrackEntry is parseable.
Why it matters: Garbage-in produces garbage-out on the user-visible info surface instead of a typed refusal or an honest `None` — the repo's own convention elsewhere ("honest no, never a guess").
What correct looks like: `.filter(|ns| *ns > 0)` at the harvest (mkv.rs) and/or `frame_ns.filter(|ns| *ns > 0).map(...)` here.
Confidence: MEDIUM — Verification: read (zero-guard absence verified at both producers; the inf path is Rust float semantics; no fixture exercises a 0-cadence MKV, so end-to-end UNVERIFIED).

### F-005 [S4]
Where: apps/cli/src/host.rs:1775-1781
Code:
```
    /// Write a `seconds`-long 44.1 kHz mono s16le WAV to a unique
    /// temp path and answer it.
    fn long_wav(seconds: u32) -> PathBuf {
        use std::io::Write as _;
        let rate: u32 = 44_100;
        let frames = u64::from(seconds) * u64::from(rate);
        let data_len = frames * 2 * 2; // stereo s16le
```
What is wrong: The doc comment says "mono", the code writes stereo (channels=2, block align 4, `data_len = frames * 2 * 2` with its own "stereo s16le" note). The distinction matters to the test's own arithmetic — the park bound's "worst single packet burst (16 384 frames of 16-bit stereo at the 64 KiB packetizer cap)" comment at :1855-1857 assumes stereo.
Why it matters: A reader tuning the fixture (e.g., to mono to halve the burst) would follow the doc comment and silently invalidate the burst bound.
What correct looks like: Fix the doc comment to "stereo".
Confidence: HIGH — Verification: read (test passes with stereo; sample_count counts frames).

### F-006 [S4]
Where: apps/cli/src/host.rs:1565-1591 (and 1404)
Code:
```
        DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}")),
```
and
```
        toml::Value::Table(_) => SettingValue::String("(table)".to_string()),
```
What is wrong: Two lossy conversions in the settings seam: (a) a `default_ref` default is reported as the literal marker string `-> product.toml <path>` (the `[PROPOSAL]` product-config join never landed); (b) a TOML table default stringifies to `"(table)"`. Additionally `setting_set` stores into the in-memory `self.settings` BTreeMap (host.rs:1404) whose only reader is `setting_get` (host.rs:1372) — no persistence, no playback consumer; `CommandResult::SettingApplied` implies more than validate-and-store does.
Why it matters: The schema's SettingType vocabulary has no table type, so (b) is latent; (a) leaks an internal marker into `config.get-setting` output. The validate-only scope is honestly doc-commented ("validate + store one setting") so no false doc claim found — hygiene debt, and a `[PROPOSAL]` the repo owes.
What correct looks like: Resolve `DefaultValue::Ref` against product.toml at build time; represent table defaults as an error or structured value; rename/document the result as "validated and stored for this process" or wire persistence.
Confidence: HIGH — Verification: read (self.settings readers grepped: only :1372/:1404).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `CliHost::media_info` | 1283-1314 (my range: 1293-1314) | REAL | S3 | NotReady typed when no facts; hard-coded seekable/is_live/metadata/chapters (F-002); `source` fallback `PathBuf::new()` placeholder unreachable in practice (facts imply source set). Reachable dispatch.rs:133. |
| `CliHost::duration` | 1317-1333 | REAL | — | Honest Known/Unknown from facts; NotReady typed. dispatch.rs:135. |
| `CliHost::current_time` | 1335-1363 | REAL | S3 | Fraction 0.0 on unknown duration (honest fallback); fabricated full-range `buffered` (F-002); `position_us().max(0)`. dispatch.rs:134. |
| `CliHost::setting_get` | 1366-1387 | REAL | — | Schema lookup with typed InvalidArgument; stored value or default; meta carries type/restart/flag. dispatch.rs:149. |
| `CliHost::setting_set` | 1390-1409 | REAL | S4 | Validates via schema then stores in-memory (F-006); no consumer beyond setting_get. dispatch.rs:152. |
| `session_state_as_wire` | 1416-1418 | REAL | — | Delegation shim to `SessionState::as_wire` (single implementation rule); callers :303, :656, :1471. |
| `is_seek_terminal` | 1424-1429 | REAL | — | Pure matches! on TimingSeekCompleted/Failed; caller :1139. |
| `track_info_wire_host` | 1433-1435 | REAL | — | Delegation shim to playback_engine::track_info_wire; caller :668. |
| `enrich_track_info` | 1441-1466 | REAL | S3 | Folds TrackFact geometry into wire row; `channels.min(u16::MAX as u32) as u16` safe; inf frame_rate on frame_ns=0 (F-004); caller :667. |
| `rejection_wire` | 1469-1533 | REAL | — | Exhaustive 10-arm mapper of RejectionReason → CommandError; Internal/Control for caller-bug arms is the right severity split; caller :885 (part2). |
| `session_error_wire` | 1536-1560 | REAL | — | 5-arm SessionError mapper with detail codes (`session.no-decodable-track`, `inbox-capacity`); callers :864, :984, :1108 (part2). |
| `default_value` | 1565-1571 | REAL | S4 | Ref-marker string for product.toml defaults (F-006); caller :1374. |
| `toml_to_setting_value` | 1573-1591 | REAL | S4 | Lossy table/"(table)" and non-string array items (F-006); caller :1568. |
| `setting_value_to_toml` | 1593-1611 | REAL | — | Number→Integer under fract==0 && abs<9e15 (cast-safe); caller :1401. |
| `setting_type_name` | 1613-1627 | REAL | — | Exhaustive 9-arm type naming; caller :1377. |
| `engine_info` | 1631-1661 | REAL | S2 | Frozen literal capability block (F-001); executed (lumen-engine-info.txt). |
| `command_listings` | 1665-1676 | REAL | — | Maps catalog_data; since="1"/None/empty matches documented v1 namespace (command.rs:680-686 docs); executed. |
| `fixture` (test) | 1691-1695 | REAL | — | Path join helper. |
| `vorbis_session_opens_with_an_audio_limb` | 1702-1718 | REAL | — | Real fixture, real open_session; mono-vs-stereo-default discrimination is a genuine proof; ok no-failfast :2870. |
| `pcm_session_keeps_its_audio_limb` | 1724-1739 | REAL | — | Committed fixture; ok :2869. |
| `in_band_config_audio_stays_limb_less` | 1746-1757 | REAL | — | ADTS in-band lane stays limb-less; ok :2868. |
| `long_wav` (test) | 1777-1806 | REAL | S4 | Real WAV bytes; doc "mono" vs code stereo (F-005); identity-literal gate violation (F-003). |
| `playing_host` (test) | 1810-1823 | REAL | — | Open+settle+assert Playing harness. |
| `settle_loop_parks_instead_of_fatalling_the_ring` | 1836-1877 | REAL | — | The GUI regression proof: park bound `AUDIO_PARK_FILL + 16_384`, no fatal, exact 6 s play-out; ok :2871. Would fail if parking regressed (fill bound + state assertions). |
| `wall_pump_renders_only_what_the_wall_owes` | 1888-1939 | REAL | — | Exact quantum arithmetic (1470/44_100), pause renders nothing; ok :2872. Would fail on a whole-ring drain. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| vorbis_session_opens_with_an_audio_limb | YES | Asserts device present AND channels==1 (the stereo default would answer 2 — the geometry flow is pinned, not the fallback). |
| pcm_session_keeps_its_audio_limb | YES | device present + channels==2 from the pcm pair. |
| in_band_config_audio_stays_limb_less | YES(weakly) | Asserts device.is_none() — pins the honest absence; would fail only if a wrong fallback appeared. |
| settle_loop_parks_instead_of_fatalling_the_ring | YES | State ≠ Error, no fatal, fill ≤ park bound, fill < RING_FRAMES, exact end position 5.99-6.01 s. |
| wall_pump_renders_only_what_the_wall_owes | YES | Exact sample counts (1_470 then 44_100) and position window — a whole-ring drain or clock drift breaks them. |
All five executed: no-failfast log :2868-2872. Note: they are absent from baseline runs #3/#4 (those aborted at other packages before lumen_cli's lib tests ran — see adts.rs audit for the section analysis).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| engine-info reports the build's decoders/features | baseline INDEX CORRECTION v2 "Standing defects" (already flags this site) | FALSE (output) | host.rs:1637-1658 literals vs pipeline_host.rs:242-346 registry (12+ arms); h264 4:2:0 MP4 plays (INDEX corrected smoke matrix). |
| "protocol_version: unwired (control-protocol is a later card)" | host.rs:1635 string | FALSE (stale) | crates/control-protocol shipped; apps/control-server/src/main.rs:21-23 imports `control_protocol::PROTOCOL_VERSION`. |
| media.get-info seekable/is_live reflect the media | doc 05 class-S shape (implied by field names) | SHALLOW | Hard-coded true/false (F-002); SeekSupport::None exists (avi/mod.rs:956-964). |
| settings seam "validate + store one setting" | host.rs:1389 doc comment | REAL | Exactly what the code does — honest scope; the wider "applied" implication is F-006. |
| command listings "since 1, empty requires" | commands/catalog/command.rs:680-686 | REAL | Matches documented v1 namespace; CommandSpec carries no since/deprecated/requires to lose. |
| backpressure test doc "the GUI bug needs a media whose decode-ahead can exceed 131 072 frames" | host.rs:1762-1767 | REAL | 6 s × 44.1 kHz = 264 600 frames > 2× RING_FRAMES; test asserts fill < RING_FRAMES. |

## 7. Dead / duplicated / unreachable code
None in this chunk. Every free function has a live caller (per-function table); both test modules are cfg(test) and executed. `media_info`'s `unwrap_or_else` empty-path source fallback (:1294-1298) is defensive dead-in-practice but harmless.

## 8. Dependencies & flags
No feature gates in this chunk — all code compiles into the shipped `lumen` binary unconditionally. External deps touched: settings-schema (runtime::schema_meta/validate_value), toml (value conversion), playback_engine (SessionState/WireEvent/track_info_wire/RejectionReason), commands catalog types. Tests use testkit::Must and write to std::env::temp_dir() (never the repo).

## 9. Open questions for the next agent
- engine_info (F-001) needs an owner decision: derive from the provider registry or delete the decoders/features fields — the fix spans pipeline_host (registry enumeration) and this file.
- mkv.rs owner: add the `default_duration_ns > 0` filter at the harvest (F-004's producer side).
- dispatch.rs/run.rs owners: does any UI surface consume `buffered`/`seekable` from media.get-info (F-002 blast radius), and does anything expect `SettingApplied` to persist?
