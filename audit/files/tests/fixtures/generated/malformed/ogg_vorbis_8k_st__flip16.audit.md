# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__flip16
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (malformed Ogg/Vorbis derivative, NO extension) |
| Size / sha256-prefix | 3245 bytes / 981c9ecea7e314ae (verified locally) |
| File verdict | REAL (consumed) — not hash-pinned; invisible to compat walker |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | python3 sha256, header sniff (`OggS\x00\x02...vnr` = vorbis BOS), rg consumers |

## 1. Verdict
Real malformed Ogg derivative of `ogg_vorbis_8k_st.ogg` (3245-byte parent:
BOS + headers page + one EOS page with 14 audio packets; 16 seeded header
bit-flips applied). Consumed by crates/demux/tests/ogg_fixtures.rs twice
(pin table + fuzz-lite survival sweep). Same two integrity gaps as its aiff
sibling: absent from MANIFEST.toml, and no extension → skipped by the
compat-matrix walker.

## 2. What actually works (verified)
- Consumer 1 — pinned outcome (verbatim, crates/demux/tests/ogg_fixtures.rs:536-545):
```
    Derivative {
        name: "flip16",
        bytes: 3_107,
        packets: 16,
        reasons: &["ogg: page CRC mismatch"],
        terminal: "",
        fails_at_start: false,
        damage: 58,
```
- Consumer 2 — fuzz survival (ogg_fixtures.rs:616): `fixtures.push(format!("generated/malformed/ogg_vorbis_8k_st__{}", d.name));` then `demux::ogg::fuzz::fuzz_demux_ogg(&bytes)` — the flip16 bytes are actually fed through the fuzz-lite harness.
- Parent provenance: docs/worklog/archive-2026-10.md:8587 records the ffmpeg-authored ogg family and its 9 derivatives (trunc0..4, flip1/4/16, garbage0).

## 3. Findings
F-001 [S3] No extension → compat-matrix walker (tests/compat/src/corpus.rs:131-137 extension filter) never collects it; malformed-ogg coverage is test-only, not matrix-visible. Same root cause as aiff sibling.
F-002 [S3] Not in tests/fixtures/generated/MANIFEST.toml — no hash pin, no `xtask fixtures --check` drift gate for these bytes. Confidence: HIGH (read both files).
