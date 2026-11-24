# AUDIT: fuzz/fuzz_targets/fuzz_playlist_cue.rs
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 20 lines / 1 fuzz_target! macro |
| File verdict | REAL-UNPROVEN (thin shim; harness fn exists; compiles-on-stable claimed; no campaign) |
| Reachable from shipped entry point? | NO — fuzz lane only; campaign unwired |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg harness existence (exit 0); fuzz_registry_check.sh RAN -> CLEAN (exit 0) |

## 1. One-paragraph verdict
Identical honest shape to fuzz_demux_es.rs: a zero-logic shim forwarding bytes to the real
harness `playlists::fuzz::fuzz_playlist_cue` — verified to exist at
crates/playlists/src/fuzz.rs:50 (`pub fn fuzz_playlist_cue(data: &[u8])`). Registry gate
ran clean (35 targets). The seed corpus (fuzz/corpus/fuzz_playlist_cue/seed_cue_plain.cue,
244 bytes — audited separately) is present and gate-checked.

## 2. What actually works (verified)
- `rg -n "fn fuzz_playlist_cue" crates/playlists/src/fuzz.rs` -> line 50 (exit 0).
- `bash scripts/fuzz_registry_check.sh` -> CLEAN, 35 targets, corpora present (exit 0).
- Baseline: playlists PACKAGE exit_code=0 (cargo-test-workspace.txt:4802-4803) — the
  in-crate deterministic cue sweeps ran and passed.

## 3. Findings
None specific. Same systemic (honest) status: campaign unwired pending the Lead decision.

## 4. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "panic-free by construction" (harness) | header line 7 | REAL-UNPROVEN | playlists package tests pass in baseline; harness body not read by me |
| compiles on stable | fuzz/README.md:31-33 | REAL-UNPROVEN | no cargo in sandbox |

## 5. Open questions
None.
