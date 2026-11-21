# AUDIT: crates/demux/src/asf/tests.rs#part3 (lines 1053–1578)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 526 (of 1578; part3) / 3 tests + 5 helpers (+ FIXTURES array tail) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — `#[cfg(test)]`; executed in baseline: all 3 range tests **ok** (`orchrun-test-workspace-nofailfast.log.txt:1566,1567,1953`; demux suite 462/462 at :1489-1955) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "asf::tests::" baseline log` (all ok; 36 `test asf::` total); `sha256sum crates/demux/src/asf/fixtures/{asf_pcm_s16le.wma,asf_stream_broadcast.wmv,wmv_wmv2_wma_av.wmv}` — **pins verified byte-exact against my own hash run**; `sed -n` on every quoted range; `ls crates/demux/src/asf/fixtures/` (11 files) |

## 1. One-paragraph verdict
This chunk is the ASF differential conformance battery: a 11-fixture
ffmpeg-authored family committed IN-TREE (byte-stable, sha256-pinned — I
re-hashed three fixtures myself and they match the pinned table exactly),
walked with exact assertions on the object tree, track table, FPO facts,
WAVEFORMATEX/VIDEOINFOHEADER fields, damage totals, first PTS, stamp
monotonicity and seek-resume; plus a live ffprobe cross-check (codec identity
AND `-count_packets` per-track counts against our media-object counts) and a
broadcast-fixture refusal test where the oracle (ffprobe) itself refuses.
This is the strongest external-anchored test design in the demux crate — it
fixes exactly the silent-skip weakness the WAV corpus suite has (F-001 there)
by committing the fixtures. The two findings are hygiene: the ffprobe
cross-check has a skip-as-pass branch (provably not triggered in the baseline,
because the sibling broadcast test hard-panics without ffprobe and it passed),
and a hand-rolled SHA-256 twin of the integration-suite implementation.

## 2. What actually works (verified)
- All three range tests passed in baseline: `broadcast_fixture_refuses_typed_where_ffprobe_refuses`
  (:1566), `fixture_family_walks_match_the_measured_truth` (:1567),
  `fixture_family_cross_checks_against_live_ffprobe` (:1953).
- **ffprobe presence in the baseline sandbox is PROVEN**: the broadcast test
  spawns ffprobe with `unwrap_or_else(|e| unreachable!("ffprobe spawn"))`
  (:1461) — an ffprobe-less environment panics that test; it shows ok.
  Therefore the cross-check's skip branch (:1329-1332) did not fire and the
  live ffprobe differential genuinely executed.
- sha256 pins verified independently: `sha256sum` on
  `asf_pcm_s16le.wma` → `29c7a469c3627a6f`, `asf_stream_broadcast.wmv` →
  `a9db9175da47fe6c`, `wmv_wmv2_wma_av.wmv` → `5bd8c76fa14e31aa` — all three
  match `expected_sha` (:1494, :1496, :1492) exactly.
- The in-tree fixture dir exists with all 11 files (`ls
  crates/demux/src/asf/fixtures/`), so `fixture_dir()` takes the committed
  branch on every normal run — no ffmpeg dependency for the walks test.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/asf/tests.rs:1329-1332
Code:
```rust
    if Command::new("ffprobe").arg("-version").output().is_err() {
        eprintln!("SKIP: ffprobe absent (asf ffprobe cross-check)");
        return;
    }
```
What is wrong: Skip-as-pass — the live-oracle cross-check returns `Ok` with
zero assertions when ffprobe is absent; the eprintln is captured by cargo for
passing tests, so a green run in an ffprobe-less environment is
indistinguishable from a genuine cross-check pass.
Why it matters: Same evidence-hygiene defect class as wav's corpus skip
(W04-A07 wav/mod.rs#part1 F-001). Here the blast radius is tiny: the sibling
`broadcast_fixture_refuses_typed_where_ffprobe_refuses` test in the same
suite hard-panics without ffprobe (:1461 `unreachable!("ffprobe spawn")`),
so any run of the whole suite without ffprobe is red regardless — the silent
skip can only mislead in a filtered single-test run.
What correct looks like: Match the broadcast test's stance (panic, or mark
the test `#[ignore]` with an explicit env opt-in) so absence is loud
everywhere.
Confidence: HIGH — Verification: read + baseline-log deduction chain (above).

### F-002 [S4]
Where: crates/demux/src/asf/tests.rs:1501-1505 (and 1506-1578)
Code:
```rust
/// Test-only SHA-256 (FIPS 180-4) — the shared `tests/common`
/// implementation's in-crate twin, pinned against the recorded
/// fixture hashes above (which were measured with the system
/// sha256sum this session).
fn sha256_16(bytes: &[u8]) -> String {
```
What is wrong: A second SHA-256 implementation lives in test code, twin of
`tests/common`'s (which the comment itself admits). Crypto code duplicated
across test boundaries is a maintenance hazard; the pin compares only the
first 16 hex characters, so a future edit that breaks the tail of the digest
would still pass.
Why it matters: If either copy is "fixed" without the other, fixture-byte
verification could silently diverge between unit and integration suites.
The current bytes are correct (my independent `sha256sum` run matches), so
this is latent, not active.
What correct looks like: Move the digest helper into a shared dev-dependency
crate (e.g. `testkit`) or compare the full 64-hex digest.
Confidence: HIGH — Verification: read + independent sha256sum cross-check.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FIXTURES` array tail (asf_adpcm_ms entry) | 1053-1065 | REAL | — | Exact measured facts (header_size 486, children 5, fpo/data tuples, wfx incl. cb_size 32, ffprobe_codecs) — the entry style of the whole array (parts 1-2). |
| `fixture_dir` | 1083-1098 | REAL | — | In-tree committed fixtures FIRST (ASF-d decision, documented with the CI run-number rationale — ffmpeg 6.1.x byte drift killed run 43373340224); fallback regenerates into a shared temp dir; None only when ffmpeg absent. The in-tree branch makes the walks test deterministic everywhere. |
| `generate` | 1100-1119 | REAL | — | Reuses existing files; asserts ffmpeg success with stderr on failure. Stale temp files are caught downstream by the sha pin (:1138). |
| `read_file` | 1121-1125 | REAL | — | plain. |
| `fixture_family_walks_match_the_measured_truth` | 1127-1322 | REAL | — | The core differential: sha pin → object tree (header_size, declared==walked children, data offset/size/packet_count, reserved, simple-index presence/absence) → track table (kind/codec/stream number) → FPO facts (file_size, packet count, preroll, min==max packet size, !broadcast, seekable) → wfx/video field-exact → flags_raw==0 → damage total==0 → per-track drain (first PTS exact, monotonic, non-zero counts) → seek+resume on indexed fixtures. Mutations all fail loudly. |
| `fixture_family_cross_checks_against_live_ffprobe` | 1324-1426 | REAL | S4 (F-001) | Live oracle: codec identity per stream, stream count parity, `-count_packets` per-track equality. `unwrap_or(u64::MAX)` on N/A parses makes ffprobe weirdness loud. |
| `broadcast_fixture_refuses_typed_where_ffprobe_refuses` | 1428-1481 | REAL | — | Oracle-parity refusal: asserts ffprobe itself fails on the broadcast form, our content probe MISSES (no false anchor), our walk refuses typed `demux.asf.broadcast-unsupported`. Honest gap documentation. |
| `expected_sha` | 1484-1499 | REAL | — | 11 pins; `unreachable!` on unpinned fixture — new fixtures cannot silently skip pinning. |
| `sha256_16` | 1505-1578 | REAL | S4 (F-002) | Correct FIPS 180-4 (verified by matching system sha256sum on 3 fixtures through the passing baseline asserts); duplication noted. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fixture_family_walks_match_the_measured_truth | YES | Mutation: wrong child walk → declared/walked mismatch; wrong packet parse → data tuple; wrong PTS ladder → first_pts exact assert; wrong damage accounting → total≠0. Every field is pinned to measured numbers. |
| fixture_family_cross_checks_against_live_ffprobe | YES (when ffprobe present — proven present in baseline) | Codec identity + per-track packet counts must equal the live oracle exactly. |
| broadcast_fixture_refuses_typed_where_ffprobe_refuses | YES | Requires: ffprobe fails, our probe misses, our start fails with the exact code. Any acceptance → `unreachable!`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ffmpeg 7.1.5 sha256-pinned fixture differential incl. live ffprobe codec + -count_packets cross-check" | docs/feature-matrix.toml:216 (container-asf-wmv evidence) | REAL | Tests exist, ran ok in baseline (:1567, :1953); pins independently re-verified by me (sha256sum). |
| "36-test asf unit suite in the 454-test demux crate" | docs/feature-matrix.toml:216 | PARTIAL drift | asf count accurate (36 `test asf::` in baseline); "454-test demux crate" is stale — baseline shows 462 (undercount; suite grew after the string was written, not inflated). |
| "Linux-only evidence, CI 3-OS pending" | docs/feature-matrix.toml:216 | honest | Consistent with baseline (all runs Linux). |

## 7. Dead / duplicated / unreachable code
- `sha256_16` duplicates `tests/common`'s SHA-256 (F-002).
- No dead code otherwise in range; the temp-dir regeneration fallback in
  `fixture_dir`/`generate` is unreachable in the committed tree (in-tree dir
  exists) but is a documented stripped-tree path.

## 8. Dependencies & flags
Test-only: std::process::Command (ffmpeg/ffprobe), std::fs, env!(CARGO_MANIFEST_DIR).
No feature gates, no unsafe, no network. The suite depends on the committed
fixtures at `src/asf/fixtures/` (11 files, also audited as separate assignment
rows by A01/A05).

## 9. Open questions for the next agent
- asf/tests.rs parts 1-2 (A02/A06): does any test cover the `header_above_memory_ceiling`
  path's exact ceiling boundary (off-by-one at the cap)? Names suggest yes;
  verify the boundary is asserted, not just above/below.
- The FIXTURES family has no A/V file with MORE than 2 streams and no
  command-media / script-command objects — real-world ASF carries those;
  the corpus plan's gap list should say so if it doesn't.
