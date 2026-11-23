# AUDIT: crates/playlists/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 23 (header 1-6, [package] 8-13, [lints] 15-16, [dependencies] 18-19, [dev-dependencies] 21-23) |
| File verdict | REAL (clean manifest, stale description) |
| Reachable from shipped entry point? | The crate: NO (see m3u.rs audit — only streaming+fuzz depend on it; streaming's own consumption is not entered by lumen-cli). The manifest itself compiles in the workspace and its dep is lockfile-real. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full read; rg (thiserror usage in src, testkit imports in tests, thiserror in Cargo.lock, boundaries.toml) |

## 1. One-paragraph verdict
A minimal, honest manifest: exactly one external dependency (`thiserror = "2"`), and it is genuinely used (derives in error.rs:14, pls.rs:63/88, xspf.rs:70/109) — not a decorative dep. The "dev-dependencies: Intentionally empty" claim is true (no test imports testkit; the fuzz harnesses are inline splitmix64, verified in pls_suite.rs:272-291). Lockfile confirms thiserror 2.0.21 pre-existing (Cargo.lock:1124-1126), matching "already in the workspace lockfile; no new dependency". boundaries.toml's `[crates.playlists]` entry (layer 2, internal_deps [], external_allowlist ["thiserror"]) matches this manifest exactly. The two findings are stale-doc nits: the [package] description sells only "extended/plain M3U parsing and canonical writing" while the crate now contains six format parsers plus the probe/walk module (m3u 894 + pls 446 + xspf 1037 + cue 954 + mpd 2854 + hls 997 + probe 1234 loc), and the header's "no internal deps yet (the queue semantics that will consume this crate land with 4.C2's panel work)" is outdated as a consumer story — a consumer (streaming, card 6.B1a) has existed since the HLS card, even though no shipped binary reaches it.

## 2. What actually works (verified)
— verified by rg: thiserror used at 5 derive sites across src; zero `use testkit` in tests/ and src/fuzz.rs; Cargo.lock thiserror 2.0.21.
— verified by baseline: PACKAGE playlists — 46 lib + 18 m3u_suite + 20 hls_attrs + 14 pls_suite + 14 xspf_suite + 20 cue_suite tests, all green, no dev-deps needed.
— verified by boundaries.toml (L222-227 area): `[crates.playlists] internal_deps = []`, `external_allowlist = ["thiserror"]` — manifest and registry agree.

## 3. Findings
F-001 [S4]
Where: crates/playlists/Cargo.toml:12
Code:
```
description = "Lumen playlist support: extended/plain M3U parsing (dirty-world tolerant, D-1/D-2) and canonical writing with pinned roundtrip stability (card 4.D1)."
```
What is wrong: stale, underselling description — the crate is six parsers (M3U, PLS, XSPF, CUE, MPD, HLS media+master) plus probe/walk, not M3U-only. Same staleness pattern as streaming's manifest description. Anyone triaging the manifest misjudges the crate's surface (and its attack surface: the hand-rolled XML subset in xspf.rs and the 2854-loc MPD parser are non-trivial code a reader would not expect here). Confidence: HIGH — Verification: wc -l of src modules.

F-002 [S4]
Where: crates/playlists/Cargo.toml:3-4
Code:
```
# Layer: application-support, no internal deps yet (the queue semantics that
# will consume this crate land with 4.C2's panel work). Std + thiserror only
```
What is wrong: "no internal deps" is accurate for THIS manifest, but the parenthetical consumer story is stale: `streaming` has depended on playlists since card 6.B1a (crates/streaming/Cargo.toml:42), so the crate's in-tree consumer exists and is not the 4.C2 panel. A reader concludes no one links the crate — true only for shipped binaries, false for the workspace graph. Confidence: HIGH — Verification: streaming manifest read.

Checked and clean (zero-finding duty): thiserror not merely declared but used; dev-deps genuinely empty with the inline-RNG justification verified in the actual test code; no features; no unsafe anywhere in the crate's manifest surface; lints inherit workspace (clippy CLEAN per baseline); publish = false; edition 2024.

## 4. Section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Header comments | 1-6 | REAL | S4 | F-002 stale consumer story |
| [package] | 8-13 | REAL | S4 | F-001 stale description |
| [lints] | 15-16 | REAL | — | workspace inheritance |
| [dependencies] | 18-19 | REAL | — | thiserror 2, used, lockfile-pinned 2.0.21 |
| [dev-dependencies] | 21-23 | REAL | — | empty as claimed, verified |

## 5. Test honesty
N/A (manifest). The suites referenced (m3u/pls/xspf/cue/hls_attrs) are all baseline-green with no dev-deps — consistent with the manifest's claim.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std + thiserror only — the same dependency family settings-schema/localization/plugin-api use" | Cargo.toml:4-6 | REAL | one dep; derives at 5 sites; lockfile 2.0.21 |
| "Intentionally empty [dev-dependencies]: the fuzz-lite harness is a deterministic inline RNG (splitmix64)" | Cargo.toml:21-23 | REAL | pls_suite.rs:272-291 SplitMix64; no testkit imports anywhere in tests/ |
| "no internal deps yet (the queue semantics ... land with 4.C2's panel work)" | Cargo.toml:3-4 | HALF-STALE | manifest-true; consumer story outdated (streaming depends on it since 6.B1a) |
| description (M3U-only scope) | Cargo.toml:12 | STALE | F-001 |

## 7. Dead / duplicated / unreachable code
No dead deps. The crate itself is unreachable from shipped binaries (m3u.rs audit F-001) — a manifest-level fact, not this file's defect.

## 8. Dependencies & flags
thiserror 2 (only external). No internal deps. No features. No dev-deps.

## 9. Open questions for the next agent
- Should the description be rewritten at the 4.C2/6.B-era boundary? Batch it with streaming's stale description (same fix pattern).
- mpd.rs (2854 loc) is the crate's largest surface and feeds DASH — is it covered by dash e2e tests in streaming, and is it reachable anywhere? (Same unreachable-substrate question, one level deeper.)
