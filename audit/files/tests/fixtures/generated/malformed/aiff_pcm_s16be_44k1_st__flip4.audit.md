# AUDIT: tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__flip4
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| File verdict | REAL (fixture consumed by a pinned-outcome demux test) — but OUTSIDE the MANIFEST drift gate (see MANIFEST audit) |
| Reachable from shipped entry point? | NO — test fixture; consumer: crates/demux/tests/wav_aiff_fixtures.rs AIFF-family table (path via format!; family header at L498) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | sha256 batch (audit/runs/W16-A08.fingerprints.txt); rg consumer sweep |

Binary fixture, 35,334 bytes, sha256 888afc3707477255623bbbb944358849aeb160c6e5fe98f6dccc0d525e700beb, `file`: IFF AIFF audio (no extension).
F-001 [S4]: byte-verified derivation (python3): flip4 vs parent
aiff_pcm_s16be_44k1_st.aiff (35,334 B, same length) differs at exactly 3 byte
offsets — 18, 20, 108 — with XOR deltas 0x33, 0x64, 0x02 (multi-bit, i.e.
several seeded bit flips landed on the same bytes; the wav_aiff header calls
them "seeded single-bit XORs at the recorded sites", L38-39 — the recorded
sites are real, the "single-bit" wording is loose). NOT in MANIFEST.toml —
outside the drift gate, `cargo xtask fixtures --check` cannot catch silent
corruption of this file.
Test honesty: the AIFF derivative table pins terminal ("demux.aiff.truncated" etc.) and damage expectations per derivative; missing/mutated fixture → test failure. Baseline passed (no wav_aiff_fixtures failures in cargo-test-workspace.txt).
Verdict: REAL adversarial fixture (bit-flip inside the AIFF COMM/SSND header surface).
