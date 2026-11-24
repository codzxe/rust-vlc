# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__flip16.mp4
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (class-B seeded header bit-flip, k=16) |
| Size / sha256 | 424452 bytes / 63a746dead5ccbe006d0073f5374750c443a97d7f7e9b7eba9030f51abe2340c (verified locally) |
| File verdict | REAL (generated, hash-locked, multiply consumed) |
| Reachable from shipped entry point? | N/A — test asset; consumed by 4+ test binaries |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 (match), `file`-style header sniff, rg consumers |

## 1. Verdict
Real, machine-generated class-B mutation fixture. First 32 bytes show a
corrupted `ftyp` box (`\x00\x00\x00\x1cft9pisom...` — 'y' of "ftyp"
bit-flipped to '9'), i.e. exactly the "seeded header bit-flips within the
family header extent" the ledger promises. sha256 and size match
MANIFEST.toml exactly, so `cargo xtask fixtures --check` actually pins
these bytes.

## 2. What actually works (verified)
- python3 hashlib -> `63a746dead5ccbe0...` == MANIFEST.toml:218 sha256, size 424452 == `size_bytes = 424_452`.
- Consumers (rg): crates/media-probe/tests/fixtures_matrix.rs:468,548 (asserts `Expected::Unknown` verdict); tests/integration/first-vertical/tests/recovery_matrix.rs:809; compat-matrix harness walks `malformed_root` (tests/compat/src/corpus.rs:106) with `.mp4` in media_extensions; expectations/malformed/mp4_mjpeg_pcm__flip16.mp4.toml exists.
- MANIFEST quote (tests/fixtures/generated/MANIFEST.toml:214-219):
```
path = "generated/malformed/mp4_mjpeg_pcm__flip16.mp4"
class = "B"
op = "flip16"
parent = "generated/valid/mp4_mjpeg_pcm.mp4"
sha256 = "63a746dead5ccbe006d0073f5374750c443a97d7f7e9b7eba9030f51abe2340c"
```
- Generator: tools/xtask/src/cmd/fixtures.rs (in-tree, deterministic; CORPUS.md: "No third-party media ... authored in-tree").

## 3. Findings
None. Fixture is honest: generated, pinned, and consumed by at least three
test suites plus the compat-matrix walker. Matrix evidence: docs/corpus/
compat-matrix-20261110.raw-rows.jsonl does NOT name this file (only valid
rows are named there) — the malformed rows are walked generically; not a
defect of the fixture.
