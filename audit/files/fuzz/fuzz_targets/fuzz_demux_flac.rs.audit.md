# AUDIT: fuzz/fuzz_targets/fuzz_demux_flac.rs

| Field | Value |
|---|---|
| Auditor | W16-RR5 (orchestrator-completed; orig W16-A10) |
| Date | 2026-11-24 |
| File verdict | REAL (thin libFuzzer shim; campaign UNWIRED) |
| Reachable from shipped entry point? | NO (fuzz crate non-member; nightly-only if ever wired) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | head (header+body), rg -> fuzz/Cargo.toml (registered target), rg wiring |

## 1. One-paragraph verdict
822-byte libFuzzer shim: one call = codecs::audio::flac::fuzz::
fuzz_demux_flac(data). Header honestly discloses: campaign needs the
Lead's nightly wiring; package compiles on stable; harness proven only
by in-crate deterministic sweeps. Registered in fuzz/Cargo.toml;
consumes the in-crate harness W07 audited (REAL harness, campaign
never run).

## 3. Findings
F-001 [S3] Where: fuzz/fuzz_targets/fuzz_demux_flac.rs:7-9
Code:
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
What is wrong: "fuzz/README.md" says SCAFFOLDED-NOT-WIRED while
audit.yml:325-352 HAS a nightly fuzz job — the repo contradicts itself
on whether any campaign can ever run; no campaign artifact exists.
Confidence: HIGH — Verification: read + rg.
F-002 [S4] Header claims "panic-free by construction" of the delegate —
the delegate's own audit (W07-A05) found stale wiring docs there too.
