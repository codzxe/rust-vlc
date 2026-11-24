# CLAIM AUDIT: docs/04-codec-format-roadmap.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 4 / REAL-UNPROVEN 2 / SHALLOW 0 / STUB 0 / PAPER-ONLY 1 / FALSE 1 / UNREACHABLE 0 | |
| Internal contradictions found | 2 (gap rows vs landed codecs) |

## 1. Blunt verdict
An honest, well-researched planning doc whose biggest gap rows died of success: the
"largest single-format hole" (H.264 software decode, "no pure-Rust decoder exists") is
FALSE on audit day — rust_h264 0.6.1 is pinned, feature-gated, and shipped with its own
dependency admission record. HEVC and Opus rows drifted the same way. The doc was never
re-scored after the admissions landed, so a reader today would re-plan solved problems.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "53 format records" (per-format record set) | §6 area | REAL | ls docs/formats/ -> 53 files |
| 2 | "**H.264 software decode** (no pure-Rust decoder exists)" | :541 | FALSE (by drift) | Cargo.lock:769 name="rust_h264"; crates/codecs/Cargo.toml:174 `rust_h264 = { version = "=0.6.1", optional = true }`; sw.rs present; docs/dependencies/records/rust_h264-0.6.1.md admission record |
| 3 | HEVC: "no pure-Rust decoder found (crates.io … NOT_FOUND) … software decode" | :258 | SHALLOW (drifted) | in-tree hevc parser/decode/hw modules exist (crates/codecs/src/video/hevc/{parser,decode,hw}); matrix row audited REAL by W19-A10 (video-hevc.md) |
| 4 | Opus in-work/blocked lane | §3 area | SHALLOW (drifted) | crates/codecs/Cargo.toml:79 `codec-opus = ["dep:ruopus"]` — ruopus landed |
| 5 | fuzz-target registry (fuzz target per parser) | §6/§2-5 | REAL | crates/demux/src/{mp4,avi,mkv,ts,ogg,wav,adts,ps,asf,ivf,aiff,es}/fuzz.rs all present |
| 6 | crates.io API + tarball purity verifications "2026-10-19" | header notes | UNVERIFIED | offline; evidence not committed |
| 7 | status vocabulary + P0-P3 priority scheme | §1 | REAL | used consistently across 53 records and matrix |
| 8 | "XL self-build" H.264 recommendation | :541 | PAPER-ONLY (superseded) | third-party crate chosen instead (see #2) — supersession not recorded in doc |
| 9 | forbidden-bindings list (openh264/libdav1d/libass …) | §3 notes | REAL-UNPROVEN | no C-wrapping dep in Cargo.toml spot-checks; full sweep not run |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "no pure-Rust H.264 decoder exists" | Cargo.toml:174 + Cargo.lock:769 + sw.rs | rust_h264 landed 2026-11-15 (admission record in-tree) — doc row FALSE |
| HEVC row: software decode as gap | codecs/src/video/hevc/decode.rs present | in-tree software HEVC decode exists — row stale |

## 4. Findings
F-001 [S3] Where: docs/04-codec-format-roadmap.md:541.
Code (verbatim): "| 1 | **H.264 software decode (no pure-Rust decoder exists)** | Largest
single-format hole; ~half of real-world video `[VERIFY]` | … `h264.software_decode =
blocked` |"
What is wrong: the doc's #1 gap is closed; the gap register was not re-scored after the
rust_h264 admission. Why it matters: the doc steers wave planning — stale gap rows
misallocate future waves to solved problems. What correct looks like: re-scored register
row pointing at the admission record. Confidence: HIGH — Verification: read + rg.
F-002 [S4] Same drift class for HEVC (:258) and the Opus lane (ruopus landed): rows
still describe the pre-landing world. Confidence: HIGH.

## 5. Fabrication / inflation indicators
None — the doc's own [VERIFY]/[VERIFY-P0] markers (24+) are honest; the defect is
staleness, not invention.

## 6. Prior-audit reconciliation
Dead W19-A08's audit (old tree) found the same drift ("FALSE-by-drift (rust_h264 landed
2026-11-15); CLI 'landed:' list contradicts engine-info baseline") — re-verified here
with the Cargo.toml/Cargo.lock/sw.rs evidence chain.
