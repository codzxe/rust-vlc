# AUDIT: fuzz/fuzz_targets/fuzz_playlist_pls.rs
| Field | Value |
|---|---|
| Auditor | W16-A08 | Date | 2026-06-14 |
| Lines / functions | 20 / 1 fuzz_target! |
| File verdict | REAL (thin shim; harness verified) — UNREACHABLE as a running fuzz target (campaign not wired) |
| Reachable from shipped entry point? | NO — cargo-fuzz binary; never run in campaign mode |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `fn fuzz_playlist_pls` → crates/playlists/src/fuzz.rs:40 (exists); baseline full_gate.txt:56-58 registry PASS |

Same one-line-shim shape: calls `playlists::fuzz::fuzz_playlist_pls(data)`
(rg: fuzz.rs:40); honest wiring-status comment; registry row README:85;
seed fuzz/corpus/fuzz_playlist_pls/seed_basic.pls (data-audited). No
findings; checked call target, attribute, registry row, corpus presence.
