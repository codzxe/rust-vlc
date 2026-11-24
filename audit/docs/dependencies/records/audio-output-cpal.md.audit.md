# CLAIM AUDIT: docs/dependencies/records/audio-output-cpal.md
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Claims extracted | 12 |
| Verdict counts | TRUE: 4 UNVERIFIED: 8 (5 of them pre-flagged [VERIFY-P1] by the doc itself) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The record's load-bearing claims are TRUE and in-tree verifiable: cpal is NOT
a workspace dependency (no hit in Cargo.toml/Cargo.lock), the admission gate is
really coded (`AUDIO_ADR005_ADMITTED = false` + compile-time assert + typed
`BackendNotAdmitted::CPAL` refusal in crates/audio-output/src/cpal.rs), and the
`AudioBackend` trait surface it describes (enumerate/open/pause/resume +
clock_source + underruns) exists exactly as claimed. The transitive-graph,
license, MSRV, unsafe-count, and build-failure claims (R2-R5, R7-R9) were
produced in a throwaway `/tmp/cpal-spike` that NO LONGER EXISTS and whose
crates are not even in the registry cache — they are unverifiable today and I
mark them UNVERIFIED. The doc pre-declares exactly this posture (throwaway
project, `[VERIFY-P1]` markers on the admission-blocking items), which is the
honest way to do a spike, but it also means the record's quantitative evidence
(1,301 extern "C" functions, per-file unsafe counts, bindgen chain) rests
solely on the author's word.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "cpal is not admitted and not a workspace dependency" | :9 | TRUE | rg cpal in root Cargo.toml + Cargo.lock -> 0 hits |
| 2 | "the flip point is already coded: audio_output::cpal carries the typed BackendNotAdmitted constructor marker + the AUDIO_ADR005_ADMITTED const" | :215-217 | TRUE | crates/audio-output/src/cpal.rs:43 `pub const AUDIO_ADR005_ADMITTED: bool = false;`, :58/:68 `BackendNotAdmitted`/`CPAL`, :98 `pub fn cpal_backend() -> Result<Box<dyn AudioBackend>, BackendNotAdmitted>` |
| 3 | "the AudioBackend trait (enumerate/open/pause/resume + clock_source + underruns)" | :138-139 | TRUE | crates/audio-output/src/device.rs:88 `pub trait AudioBackend: AudioSink` with `enumerate`/`open`; null.rs:123 `clock_source`, :128 `underruns` |
| 4 | "this record ends in a [PROPOSAL], not an allowlist row" | :12-13 | TRUE | :197 `[PROPOSAL — Lead sign-off]`; deny.toml has no cpal row (rg -> 0) |
| 5 | "real cargo output gathered 2026-11-09 in a throwaway /tmp/cpal-spike project" | :7-9 | UNVERIFIED | /tmp/cpal-spike absent (ls -> no such dir); no cpal/alsa/coreaudio/dasp tarballs in ~/.cargo/registry cache — spike evidence was ephemeral and is gone |
| 6 | "License (cargo metadata): Apache-2.0. MSRV: rust-version = 1.70" | :23-24 | UNVERIFIED | needs the spike cargo output; cpal tarball not present locally |
| 7 | "alsa-sys 0.3.1 src/generated.rs: 1,301 extern \"C\" functions" | :112-113 | UNVERIFIED | same — count not re-derivable from this sandbox |
| 8 | "Build attempt (real): cargo build ... exit 101 ... Package alsa was not found" | :125-128 | UNVERIFIED | no cargo binary in this sandbox; alsa dev pkg absent locally corroborates the tenor but not the recorded run |
| 9 | "All inside the deny.toml allow set" (R7) | :149 | TRUE-STRUCTURAL | deny.toml:85-86 allows "MIT","Apache-2.0" — the license classes named |
| 10 | "rodio 0.23.0 ... include cpal ^0.18.2 (crates.io API, fetched 2026-11-09)" | :184-186 | UNVERIFIED | no network; not re-fetchable |
| 11 | "cpal 0.15.3 ... 0.18.2 is current, released 2026-08-16" | :155-156 | UNVERIFIED | crates.io data, no network access |
| 12 | "never a panic, never a fake success" (the seam's contract) | cpal.rs doc | TRUE | cpal.rs:98-107: `cpal_backend()` returns Err(BackendNotAdmitted::CPAL) — typed refusal; no panic path read |

## 3. Contradictions
None found. The record is consistent with the tree state (not-admitted) and
with crates/audio-output's gate module.

## 4. Fabrication / inflation indicators
None visible — but note the STRUCTURAL evidence limitation: every quantitative
claim (graph, unsafe counts, MSRV, licenses) came from a deleted throwaway
project. The honest markers (`[VERIFY-P1]`, "throwaway", "nothing here asserts
device behavior") argue against fabrication; the numbers remain trust-based
until a spike is re-run.

## 5. Matrix rows
N/A — dependency record, not a matrix chunk.

## 6. Reachability note (per spec §7)
`cpal_backend()` reachable from shipped code? crates/audio-output/src/lib.rs:136
`pub use cpal::{...}` — the module is part of the shipped audio-output crate
(workspace member), and the refusal arm IS the shipped behavior (the seam).
The REAL cpal adapter does not exist (not admitted). The gate tests
(`the_gate_is_recorded_closed`, `the_constructor_refuses_with_the_typed_marker`)
passed in the orchestrator baseline (cargo-test-workspace: audio-output ok).
So: seam REAL + executed; adapter ABSENT by design.
