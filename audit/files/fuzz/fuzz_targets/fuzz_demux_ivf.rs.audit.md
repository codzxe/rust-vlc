# AUDIT: fuzz/fuzz_targets/fuzz_demux_ivf.rs
| Field | Value |
|---|---|
| Auditor | W16-A08 | Date | 2026-06-14 |
| Lines / functions | 20 / 1 fuzz_target! |
| File verdict | REAL (thin shim; harness verified) — UNREACHABLE as a running fuzz target (campaign not wired) |
| Reachable from shipped entry point? | NO — cargo-fuzz binary; never run in campaign mode |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `fn fuzz_demux_ivf` → crates/demux/src/ivf/fuzz.rs:49 (exists); baseline full_gate.txt:56-58 registry PASS |

Same one-line-shim shape as fuzz_codec_flac: calls the real
`demux::ivf::fuzz::fuzz_demux_ivf(data)` (rg: ivf/fuzz.rs:49); honest
wiring-status comment; registry row README:65; seeds in
fuzz/corpus/fuzz_demux_ivf/ (2 files incl. seed_ivf_av1.ivf — audited in the
corpus data audits). No findings; checked call target, attribute, registry
row, and corpus presence.
