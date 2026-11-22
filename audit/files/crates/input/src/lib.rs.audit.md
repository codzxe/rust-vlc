# AUDIT: crates/input/src/lib.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 76 / 0 functions (crate root: 7 module decls, 6 re-export lines) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen` bin open path: run.rs → apps/cli/src/demux_host.rs (rg confirms `input::` usage) → this crate's `FileAccess`/`BufferedReader`/`resolve_file` (module files owned by other agents). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `cat` of lib.rs + crates/input/Cargo.toml (exit 0); `rg` for `input::` in apps/cli/src, crates/demux/src, crates/media-probe/src; `rg resolve_origin` callers. No cargo (adaptation). |

## 1. One-paragraph verdict

A crate root whose entire body is module declarations, cfg-gated re-exports, and an unusually specific design doc. Nothing executable lives here; the honest audit is of the doc claims versus the gating. The one substantive claim I could verify locally — the `contract` module is test/conformance-only — is true at the manifest level (`contract = []` feature, `#[cfg(any(test, feature = "contract"))]` gating at both the `pub mod` and the re-export). The rest of the doc's claims (truncation-as-typed-error, bounded buffer, UTF-8 paths, no-registry-hook) point into sibling files owned by other agents and are recorded as open questions, not verdicts.

## 2. What actually works (verified)

- Gating consistency: `#[cfg(any(test, feature = "contract"))] pub mod contract;` and the matching gated `pub use contract::{...}` (lib.rs:65-68 region) — release binaries compile none of it; Cargo.toml's `[features] contract = []` confirms no default-on. Verified by reading both files.
- Reachability of the crate from the shipped CLI: `rg "input::"` hits apps/cli/src/parse.rs and apps/cli/src/demux_host.rs — the open path consumes this crate.

## 3. Findings

### F-001 [S4] The crate doc records five design claims whose proofs live in unaudited-here sibling files
Where: crates/input/src/lib.rs:43-61 (doc block "Design decisions (recorded for review)")
What is wrong: nothing in this file — but the claims ("Truncation is a typed error, never a silent EOF"; "buffer allocated once and never grows"; "requests above a hard ceiling are refused"; "Paths are UTF-8"; "No registry hook yet") are assertions about buffered.rs/file.rs/uri.rs, which this audit does not cover. Recorded so the sibling-file agents see the claim ledger entries owed.
Confidence: HIGH that the claims are unverified here — Verification: read.

## 4. Per-function table

Not applicable — no functions, traits, or impls in this file. (Module inventory: `buffered`, `contract` (gated), `error`, `file`, `seekable`, `uri`; re-exports `BufferedReader`, `DEFAULT_BUFFER_BYTES`, `MAX_BUFFER_BYTES`, `MAX_READ_BYTES`, `ScriptSource`/`SyntheticStream` (gated), `FileAccess`, `FileSource`, `FileStream`, `RawRead`, `Seekable`, `resolve_file`.)

## 5. Test honesty

No tests in this file.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| contract module is test/conformance-only, release binaries carry none of it | lib.rs:25-28 + Cargo.toml features | REAL | cfg gating at mod + re-export; feature not default |
| Network protocols rejected typed here, belong to streaming | lib.rs:4-7 | UNVERIFIED (uri.rs — other agent) | — |
| Truncation is a typed error | lib.rs:48-50 | UNVERIFIED (buffered.rs) | flagged F-001 |
| Bounded reader: fixed capacity, bypass, hard ceiling | lib.rs:17-24 | UNVERIFIED (buffered.rs) | flagged F-001 |

## 7. Dead / duplicated / unreachable code

None in this file. (`MAX_READ_BYTES` re-export — consumed by demux_host or buffered callers; not traced further.)

## 8. Dependencies & flags

media-types + plugin-api only; std-only per C5 (comment), consistent with Cargo.toml `[dependencies]`; testkit dev-only. No unsafe in this file (whole crate claims none — UNVERIFIED for siblings).

## 9. Open questions for the next agent

- buffered.rs: is the read-ahead window truly never grown, and is the hard ceiling refusal wired (lib.rs's claim)?
- uri.rs: does `resolve_file` reject non-`file:` schemes typed (the "never silently mis-opened" claim)?
