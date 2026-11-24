# CLAIM AUDIT: docs/dependencies/records/audio-output-cpal.md
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Claims extracted | 14 |
| Verdict counts | TRUE: 8 (2 of them structural) UNVERIFIED: 6 (4 pre-flagged [VERIFY-P1] by the doc itself) |
| Internal contradictions found | 0 |

Note: W17-A10's earlier copy of this ledger sits at
audit/docs/dependencies/records/audio-output-cpal.md.audit.md (missing the
`docs/` path segment); this is the canonical audit/docs/docs/<path>.audit.md
recovery copy, re-verified by W17-R1.

## 1. Blunt verdict
The load-bearing claims are TRUE and in-tree verifiable: cpal is NOT in the
workspace (0 hits in root Cargo.toml/Cargo.lock; crates/audio-output has
path-only deps), the admission gate is really coded (`AUDIO_ADR005_ADMITTED =
false` + compile-time assert + typed `BackendNotAdmitted::CPAL` refusal in
crates/audio-output/src/cpal.rs), and flipping the const alone breaks the
build — the assert at cpal.rs:48-52 is the mechanism, so "the flip point is
already coded" is not rhetoric. The quantitative spike evidence (R2-R5
graphs, unsafe counts, 1,301 extern "C", exit-101 build failure) came from a
throwaway `/tmp/cpal-spike` that no longer exists and whose tarballs are not
in the registry cache — unverifiable today, exactly as the record itself
pre-declares. Zero fabrications found; zero contradictions.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "cpal is not admitted and not a workspace dependency" | :9 | TRUE | `rg -i cpal Cargo.toml Cargo.lock` -> 0 hits; crates/audio-output/Cargo.toml:32-41 deps = 3 path crates only |
| 2 | "this record ends in a `[PROPOSAL]`, not an allowlist row" | :12-13 | TRUE | :197 `[PROPOSAL — Lead sign-off]`; rg cpal in deny.toml -> no row (allow list is license classes only) |
| 3 | "the flip point is already coded ... BackendNotAdmitted constructor marker + the AUDIO_ADR005_ADMITTED const" | :215-217 | TRUE | crates/audio-output/src/cpal.rs:43 `pub const AUDIO_ADR005_ADMITTED: bool = false;`; :68 `pub const CPAL: BackendNotAdmitted`; :98-107 `cpal_backend() -> ... return Err(BackendNotAdmitted::CPAL)`; flip-breaks-build mechanism at :48-52 `const _: () = assert!(!AUDIO_ADR005_ADMITTED, ...)` |
| 4 | "the implemented AudioBackend trait (enumerate/open/pause/resume + clock_source + underruns)" | :138-139 | TRUE | crates/audio-output/src/device.rs:88 trait, :91 enumerate, :103 open, :114 pause, :121 resume, :127 clock_source, :131 underruns |
| 5 | "real cargo output gathered 2026-11-09 in a throwaway /tmp/cpal-spike project" | :7-9 | UNVERIFIED | `ls /tmp/cpal-spike` -> No such file or directory; no cpal/alsa/coreaudio/dasp tarballs in ~/.cargo/registry cache — spike evidence is gone |
| 6 | "License (cargo metadata): Apache-2.0. MSRV: rust-version = 1.70" | :23-24 | UNVERIFIED | needs the spike cargo output; cpal tarball not present locally |
| 7 | "alsa-sys 0.3.1 src/generated.rs: 1,301 extern \"C\" functions" | :112-113 | UNVERIFIED | count not re-derivable from this sandbox (no tarball, no cargo) |
| 8 | "Build attempt (real): cargo build ... exit 101 ... Package alsa was not found" | :125-128 | UNVERIFIED (run) / environment corroborated | no cargo in audit sandbox; but `pkg-config --exists alsa` -> exit 1 and no /usr/lib/x86_64-linux-gnu/pkgconfig/alsa.pc — the sandbox really has no ALSA dev package, and crates/audio-output/Cargo.toml:14-15 independently records "alsa-sys/pkg-config probe fails, exit 101" |
| 9 | "All inside the deny.toml allow set" (R7) | :149 | TRUE-structural | deny.toml:85-86 allow list contains "MIT","Apache-2.0" — covers every license the R7 row names |
| 10 | "rodio ... normal dependencies include cpal ^0.18.2 (crates.io API, fetched 2026-11-09)" | :184-186 | UNVERIFIED | no network access in audit sandbox |
| 11 | "0.18.2 is current, released 2026-08-16 per the P0 record" | :155-156 | UNVERIFIED (date) / P0 record exists | docs/dependencies/records/cpal-0.18.2.md exists (card-2.C3 P0 verification record); release date not checkable offline |
| 12 | "never a panic, never a fake success" (the seam contract, cpal.rs header) | :9-10 | TRUE | cpal.rs:98-107 refusal arm returns the typed Err; the unreachable! line sits behind the false const |
| 13 | "our render_into is the callback body a cpal adapter would install" | :138-140 | TRUE | crates/audio-output/src/render.rs:89 `pub fn render_into(`; :127 "calls [`render_into`] directly" |
| 14 | "the sandbox ships no alsa.pc/ALSA dev package ... no audio device" | :127-131 | TRUE (this sandbox too) | pkg-config alsa -> exit 1; no alsa.pc; consistent with the STUB-state cross-ref (null backend + typed stubs are the only impls: null.rs:433, stub.rs:87) |

## 3. Contradictions
None. The record is consistent with the tree: not-admitted state, gate module,
null backend + typed stubs only (crates/audio-output/src/null.rs:433
`impl AudioBackend for NullBackend`; stub.rs:87 `impl AudioBackend for
UnsupportedBackend`), and the P0 sibling record exists.

## 4. Fabrication / inflation indicators
None visible. The doc pre-declares its evidence posture (throwaway project,
`[VERIFY-P1]` markers, "Nothing ran against a real audio device"), which is
the honest spike shape. The quantitative claims (graphs, unsafe counts,
exit 101) remain trust-based until a spike is re-run — flagged as UNVERIFIED
above, not as fabrication.

## 5. Matrix rows
N/A — dependency record, not a matrix chunk.

## 6. Reachability + test honesty (spec §7)
`audio-output` is a shipped workspace member (baseline cargo-test-workspace:
PACKAGE audio-output exit_code=0). The gate tests
`the_gate_is_recorded_closed` / `the_constructor_refuses_with_the_typed_marker`
(cpal.rs:115/:128) are real assertions on the const + refusal arm — flipping
the const would break both the compile-time assert and the first test; they
would fail if the gate rotted. REAL, baseline-executed.
