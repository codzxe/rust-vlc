# AUDIT: apps/web/src/capabilities.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-24 |
| Lines / functions | 440 / 14 (9 prod items, 5 tests) |
| File verdict | SHALLOW (real data snapshot honestly wired into the handshake, but the decoder advertisement and one status row overstate what the host can actually exercise) |
| Reachable from shipped entry point? | YES — `capabilities()` called at protocol.rs:313 (worker handshake reply) and host.rs:837/951/1023 (boot snapshot); wasm module built by scripts/rc-smoke.sh:130 (`cargo build -p lumen-web --target wasm32-unknown-unknown --locked`, rc-smoke PASS per baseline) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `sed -n` quote verification; `rg -c 'dnxhd' crates/demux/src/` (exit 1 — zero); `rg -n 'http' apps/web/src/open.rs` (typed refusal at :182); baseline cite cargo-test-workspace.txt:3351-3355 (5/5 tests ok); `rg -rln 'FetchRequest|StagedSource'` callers; `rg 'builtin-codecs' .github/workflows docs/web/size-report.md` |

## 1. One-paragraph verdict
The snapshot mechanism is real and honestly wired: 5/5 unit tests pass in the baseline, the data rides the worker handshake, and the shipped default wasm module truthfully reports zero decoders. But this "honest capability report" fails its own truth rule twice. First, the `codec-dnx` row advertises `"dnxhd"` (line 220) — the Wave 6 flag persists: no demuxer in the repo mints a `dnxhd` descriptor (`rg -c 'dnxhd' crates/demux/src/` → exit 1), and the web host's ONLY open path is AVI, whose `StreamFormat::codec_descriptor()` can emit nothing but `mjpeg`/`pcm_*` (demux/src/avi/header.rs:217-221) — the advertised decoder is unreachable by every openable file. Second, the `http-streaming` row says `NotStarted` while a complete fetch+Range+seekability+staged-source lane sits unwired in the same crate (apps/web/src/io/), and the file's own vocabulary defines `NotStarted` as "not yet started".

## 2. What actually works (verified)
— verified by `audit/runs/baseline/cargo-test-workspace.txt:3351-3355` -> 5/5 `capabilities::tests` ok. Default-module honesty verified: rc-smoke.sh:130-139 builds lumen-web wasm32 with no features (decoder-less; baseline rc-smoke web-ui-build/browser-load PASS). Mirror discipline verified: host.rs:574-695 dispatch rows match the feature-gated descriptor list codec-for-codec (mjpeg/pcm_/adpcm_/flac/mp3-mp1-mp2/vorbis/opus/aac-aac-lc/alac/vp9/dnxhd).

## 3. Findings
F-001 [S2]
Where: apps/web/src/capabilities.rs:193-220
Code:
```
        let mut set = vec![
            "mjpeg".to_string(),
            "pcm_s16le".to_string(),
            "adpcm_*".to_string(),
        ];
        #[cfg(feature = "codec-flac")]
        set.push("flac".to_string());
```
(through)
```
        #[cfg(feature = "codec-dnx")]
        set.push("dnxhd".to_string());
```
What is wrong: The field is documented "The registry descriptors this build can actually decode" (line 108-109), but the web host's only open path is AVI (open.rs:238 refuses every other container), and the AVI demuxer can mint only `mjpeg` and `pcm_*` track codecs (crates/demux/src/avi/header.rs:217-221 — `StreamFormat::MotionJpeg => "mjpeg"`, `StreamFormat::Pcm { sample, .. } => sample.descriptor()`). No demuxer anywhere in crates/demux emits `dnxhd` (`rg -c 'dnxhd' crates/demux/src/` → zero matches). Therefore in any `builtin-codecs*` build, ten of the twelve advertised descriptors (adpcm_*, flac, mp3/mp1/mp2, vorbis, aac/aac-lc, alac, opus, vp9, dnxhd) are unreachable through every user-facing open — a user on such a build reading the snapshot believes MP3/DNxHD playback exists; opening any such file answers "no web-host wiring" or "no decoder lane" before the advertised decoder is ever consulted. The `dnxhd` row is the exact Wave 6 flag (wave-06/SUMMARY.md:27-28: "apps/web capabilities advertise it anyway (W06-A09)") — still present, still misleading. Why it matters: this file IS the anti-hype mechanism ("Nothing here is aspirational", line 5), and it ships aspirational rows in the codec-carrying lane measured in docs/web/size-report.md:37 (`external-all` includes dnx). What correct looks like: advertise only descriptors the open path can mint (mjpeg + pcm_*), or gate the extra rows behind a container-lane that actually produces them. Confidence: HIGH — Verification: read + rg (dnxhd zero in demux; AVI descriptor vocabulary read).
F-002 [S3]
Where: apps/web/src/capabilities.rs:272-278
Code:
```
        BlockedCapability {
            capability: "http-streaming".to_string(),
            status: CapabilityStatus::NotStarted,
            reason: "fetch + Range seekability detection is card 7.H4 \
                     (doc 09 §6.1)"
                .to_string(),
        },
```
What is wrong: The status is false by this file's own vocabulary: `NotStarted` = "Planned work (named card), not yet started" (line 75-76), but card 7.H4's fetch+Range+seekability work HAS started and largely landed — in this very crate: apps/web/src/io/fetch_port.rs (typed fetch seam), io/seekability.rs (Range probe detection), io/staged_source.rs (bounded staged SourceStream), all real and mock-tested. What is actually missing is the wiring (open.rs:182-186 refuses `http`/`https` with the same stale "is card 7.H4" text) and the Worker JS glue (zero occurrences of fetch glue in ui/ or apps/web non-Rust sources). The honest status per this file's own vocabulary is `Blocked` ("A substrate exists but the engine lane has not landed"). Why it matters: the snapshot is the product's truth surface; it tells users nothing exists when a complete unwired lane exists — the exact inversion of its purpose, and it will mislead the next agent auditing "what's left". What correct looks like: `Blocked` with a reason naming "lane landed in io/, open-path wiring + JS glue pending". Confidence: HIGH — Verification: read + rg.
F-003 [S3]
Where: apps/web/src/capabilities.rs:107-110 and 196
Code:
```
    /// The registry descriptors this build can actually decode
    /// (empty on wasm32 — see the module docs).
    pub decoders: Vec<String>,
```
What is wrong: Two accuracy defects in the native floor: (a) the native list is hard-coded `["mjpeg", "pcm_s16le"]` (line 158) but the dispatch it mirrors accepts every `pcm_*` descriptor (`codec.starts_with("pcm_")`, host.rs:593) and the AVI demuxer mints pcm_u8/pcm_s24le/pcm_s32le/pcm_f32le/pcm_f64le (header.rs:151-156) — a native web host can decode five more PCM flavors than it reports; (b) `"adpcm_*"` is a wildcard pattern, not a registry descriptor — the real registry names are `adpcm_ima_wav`/`adpcm_ms` (demux/src/wav/format.rs:207-208). Under-claiming is the safe direction, but a field defined as "the registry descriptors" containing a glob is a vocabulary lie. Confidence: HIGH — Verification: read + rg.
F-004 [S4]
Where: apps/web/src/capabilities.rs:324-330
Code:
```
        renderer: "unprobed (the page has not reported renderer facts \
                   yet — the boot handshake carries the probe result)"
            .to_string(),
```
What is wrong: `capabilities()` ALWAYS returns the static "unprobed" renderer string — the only constructor never takes probe facts, so every snapshot built after the handshake still says "unprobed" unless the caller overwrites the field. host.rs:837 builds the boot snapshot; whether the page's later probe facts ever replace this string is host.rs's business — from this file's side the field is a constant, weakening the "active backend reported from the page's probe facts" claim in the field docs (lines 98-102). Confidence: MEDIUM — Verification: read (host.rs probe-fact plumbing not fully traced: UNVERIFIED).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Threading / as_str | 35-52 | REAL | — | honest single-mode enum; pooled not claimed |
| BlockedCapability / CapabilityStatus / as_str | 54-89 | REAL | — | vocabulary defined here; misused at :274 (F-002) |
| HostCapabilities / blocked_row | 91-122 | REAL | — | linear find; used by open.rs refusals |
| TargetProfile / build_profile | 124-144 | REAL | — | cfg!-based; correct |
| decoder_descriptors | 156-161 | REAL | S2 | native under-lists pcm_*, wasm32 list → F-001/F-003 |
| wasm32_builtin_descriptors | 172-225 | REAL | S2/F-001 | feature-gated mirror of host.rs dispatch; unreachable-through-open rows |
| capabilities | 229-334 | REAL | S3 | 9 blocked rows + wasm decoder-gap row; http-streaming row stale (F-002); renderer constant (F-004) |
| 5 unit tests | 336-440 | REAL | — | exact assertions incl. cfg'd branches |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| threading_is_single_on_both_profiles | YES | asserts enum + wire string |
| decoder_claims_match_the_profile | YES | asserts exact native list; wasm list per feature cfg; blocked-row status + reason substrings |
| doc_09_table_rows_are_typed_data | PARTIAL | asserts presence + non-empty reason only — cannot catch a wrong status (F-002) or unreachable decoders (F-001) |
| blocked_row_lookup_answers_by_name | YES | hit + miss paths |
| snapshot_serializes | WEAK | two substring checks only |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing here is aspirational: every field names the build's real state" | module doc :3-8 | FALSE for decoders/status rows | F-001 (dnxhd+9 descriptors unopenable), F-002 (NotStarted for landed lane) |
| "decoders: the registry descriptors this build can actually decode" | :108-109 | FALSE in codec builds | AVI-only open path mints mjpeg/pcm_* only |
| "the wasm32 profile ships decoder-less … snapshot says so" | module doc :20-26 | REAL | default build: empty list; rc-smoke builds featureless |
| "kept in one place so the snapshot and the provider cannot drift" | :146-148 | REAL | host.rs dispatch mirrors descriptor list exactly (verified row by row) |
| http-streaming "NotStarted … not yet started" | :272-278 + vocabulary :75-76 | FALSE | io/ lane landed; only open wiring + JS glue missing |

## 7. Dead / duplicated / unreachable code
Nothing dead in-file. Cross-file: the descriptors advertised under codec-* features (F-001) are unreachable through the only open path; `builtin-codecs-external` measurement lane (size-report.md:37) exists solely to size dead-in-practice decoders.

## 8. Dependencies & flags
serde/serde_json only. Feature gates `builtin-codecs`, `codec-flac/mp3/vorbis/aac/alac/opus/vp9/dnx` — none in default; shipped wasm module enables none (rc-smoke.sh:130). `adpcm_*` floor rides `builtin-codecs`.

## 9. Open questions for the next agent
- CORRECTION (self-audit): an earlier draft of this section alleged an `ln.wasm` vs `lumen_web.wasm` artifact-name mismatch in ui/e2e — that was the auditor's own `rg -rln` flag typo (`-r ln` replaces matches with "ln"), not a repo defect. Re-verified clean: ui/e2e/web/matrix.config.json:10 `"wasmArtifact": "wasm32-unknown-unknown/release/lumen_web.wasm"` and web-matrix.spec.ts:92 `endsWith("lumen_web.wasm")` are consistent with ui/scripts/web-matrix-run.mjs:461. No mismatch exists.
- When 7.H4 wiring lands, remember the http-streaming row must flip to REAL and open.rs:182's refusal must go.
