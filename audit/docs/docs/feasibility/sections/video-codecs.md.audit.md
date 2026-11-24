# CLAIM AUDIT: docs/feasibility/sections/video-codecs.md
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 16 |
| REAL 7 / REAL-UNPROVEN 3 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 6 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The most honest document audited so far in this wave: every in-repo checkable fact checks
out exactly (VLC census numbers, ADR numbering conflict, HEVC store-id drift, rav1d
asm-off config, decoder-reporting types), external web/crates.io facts are self-flagged
`[RV]`-by-author or `[VERIFY]`, and its proposals (conformance-gated adoption of
rust_h264 / rusty_vp9 / rav1d) were actually implemented as feature-gated optional
dependencies after the fact. Its one unverified-at-the-time wasm32 leg later resolved
NEGATIVE in reality (rav1d does not build on wasm32) — the doc never claimed it would.
No fabricated citations, no overclaims. Findings below are cross-reference notes, not
defects in the doc.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`video decoder` capability = 14 registrations, `hw decoder` = 3 (`vaapi` 100, `vaapi_drm` 0, `vdpau_avcodec` 100)" | :9-10 | REAL | python3 over docs/inventory/modules-linux.json: video decoder count=14, hw decoder count=3; vaapi score 100, vaapi_drm 0, vdpau_avcodec 100 — exact match |
| 2 | "`avcodec` ffmpeghw scores 10001 (the highest decoder score in the entire census) and `dav1d` 10000" | :12-13 | REAL | python3: max score across all 56 capabilities = 10001 (video decoder/avcodec); dav1d=10000 — exact match |
| 3 | doc 04 §4.2 "no production-grade pure-Rust H.264 decoder" "no longer literally true" | :32-34 | REAL | docs/04-codec-format-roadmap.md:272 contains the quoted sentence "no production-grade pure-Rust H.264" — quote accurate; the "no longer true" half rests on 2026-10 crates.io state (external) |
| 4 | rav1d compliant config `default-features = false, features = ["bitdepth_8","bitdepth_16"]` | :258-259 | REAL | crates/codecs/Cargo.toml:125 `rav1d = { version = "=1.1.0", optional = true, default-features = false, features = ["bitdepth_8", "bitdepth_16"] }` — adopted verbatim; Cargo.lock:674 rav1d present |
| 5 | "build.rs is a no-op without `asm`" (§3.1 finding 4) | :261-267 | REAL-UNPROVEN | tarball-inspection claim by author; consistent with repo's asm-off adoption but I did not download/extract the tarball (no network) |
| 6 | "`cc`/`nasm-rs` … unconditional build-dependencies … declared-but-inert" | :277-286 | REAL-UNPROVEN | same class of author-verified claim; no in-repo way to confirm without building rav1d (cargo-build-budget) |
| 7 | wasm32 rav1d leg "[VERIFY: Phase 5 build matrix]" | :287-292, :314 | REAL | flagged unknown by the doc; resolved negative later: crates/codecs/Cargo.toml:62 "rav1d does not compile for wasm32-unknown-unknown (39 errors, measured" and apps/web/Cargo.toml:113-114 "rav1d … deliberately ABSENT" — the doc's caution was warranted, and it never claimed wasm support |
| 8 | ADR numbering conflict "doc 01 and ROADMAP disagree on 008/009" | :409-412 | REAL | docs/01-architecture.md:1502-1503 (ADR-008=optical disc, ADR-009=WASM threading); docs/ROADMAP.md:161-164 now shows reconciliation note "former ROADMAP identities 'WebCodecs-as-hw-analog' / 'DVD/Blu-ray per-title' were unregistered" — F-12 was accurate and was fixed |
| 9 | HEVC store id "old deep-link id `9N4WGH0Z6VHM` is dead (HTTP 410), current id `9nmzlz57r3t7`" | :68 (§1 row 8) | REAL | docs/inventory/platform-windows.md:184-185 contains both ids and the 410 note — cross-reference intact |
| 10 | "system.get-engine-info → decoders[].backend" reporting rule | :231-232, :422 | REAL | crates/commands/src/catalog/types.rs:553-563 `DecoderInfo { name, kind, hardware: bool, backend: Option<String>, active }` — fields exist as claimed |
| 11 | F-2/F-5 candidate adoption tracks → later real code | :579, :582 | REAL | crates/codecs/Cargo.toml:73 `codec-h264-sw = ["dep:rust_h264"]`, :81 `codec-vp9 = ["dep:rusty_vp9"]`, :174 `rust_h264 = { version = "=0.6.1" … }`, :145 `rusty_vp9 = "=0.1.3"` — proposals became gated deps (with Lead-admission comments citing the records) |
| 12 | W3C registry "registers exactly five video codec strings: av01/avc1+avc3/hev1+hvc1/vp8/vp09" | :374-375 | UNVERIFIED | external web fact; no network access in audit sandbox |
| 13 | MDN/BCD browser floors (Chrome 94+, Firefox 130+ desktop-only, Safari 16.4+) | :379-392 | UNVERIFIED | external; doc marks provenance (MDN/BCD fetched 2026-10-19) |
| 14 | crates.io crate facts (rusty_h264-decoder 0.16.1 bit-exact 35/35, rust_h264 174 byte-exact tests, rusty_vp9 315/315 vectors) | :110-114, :463-468 | UNVERIFIED | self-reported conformance, explicitly labeled "conformance numbers self-reported" by the doc itself (:133-134) |
| 15 | x264/openh264 maintainer history (Wikipedia, Cisco README) | :166-172 | UNVERIFIED | external sources |
| 16 | "rav1d 1.1.0 stays the only compliant AV1 software path" | :297-298 | REAL-UNPROVEN | consistent with Cargo.lock (only rav1d for AV1) but the exclusion of alternatives is an external-crate-survey claim |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none found) — the doc self-qualifies every uncertain item with [VERIFY]/[PROPOSAL]/[RV] | — | — |

Cross-doc note (not a contradiction): §3.2 P5.1 planned a wasm32 build leg "where
feasible"; apps/web/Cargo.toml:20 later measured 39 compile errors — reality disagreed,
the doc's own closing rule (":593-595 when reality disagrees with a row, the row loses")
was applied by the codebase, not the doc.

## 4. Fabrication / inflation indicators
- None. Negative facts (dead store link HTTP 410, probe negatives, "none of them earned
  trust yet", "not yet independently audited" quoted from the candidate project itself)
  are recorded prominently. Download counts are quoted with dates and marked
  self-reported. This is the anti-fabrication posture the audit wants.

## 5. Matrix rows
N/A — feasibility investigation doc; the matrix rows it *predicts* (`av1.software =
partial`, `h264.hw = partial/hw-only`, `vp9.hw = partial`) belong to
docs/feature-matrix.toml chunk rows-170-182 (audited separately by this agent).

## 6. Reachability
Doc-only artifact feeding ADR-001/008/010 decisions; its adoption proposals demonstrably
reached Cargo.toml (claims 4, 11). No code to reach.

## 7. Open questions for the next agent
- Verify claim 5/6 (rav1d build.rs asm gate, cc/nasm-rs inertness) if a no-C-toolchain
  sandbox build is ever cheap: `cargo build -p codecs --features codec-av1`.
- The shipped default build still reports decoders `mjpeg, pcm_s16le` only (baseline
  rc-smoke engine-info) — the feasibility doc's P0 codec paths (av1/h264/vp9 gates) are
  all OFF in the default lane; cross-check matrix rows 170-182 against which features
  the release lane enables.
