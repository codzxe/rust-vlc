# AUDIT: crates/demux/src/asf/guid.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 270 / 15 (9 fns/impls + 15 GUID consts + 3 tests) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — the entire `demux::asf` module is unwired: media-probe's builtin registry registers 14 signatures, none ASF (crates/media-probe/src/signature/mod.rs:132-145); the CLI container dispatch has no `asf` arm (apps/cli/src/containers/mod.rs:57-74 covers adts/aiff/avi/ivf/matroska/flac/hevc/h264/mp2/mp3/mp4/ogg/mpegps/mpegts/wav); rg `asf` outside `crates/demux` → zero references. No `lumen` code path can construct this module's consumers. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `asf` in apps/cli/src/containers/mod.rs (no arm) · rg `registry.register` (14 signatures, no ASF) · rg `asf::|demux::asf` outside crates/demux (zero hits) · rg `asf::` in baseline log (36 tests ok, lines 1555-1590) · GUID byte values hand-verified against the ASF specification set (see §2) · sed -n verification of every quote |

## 1. One-paragraph verdict
The file itself is clean, honest, and correct: all 15 GUID constants carry the exact wire-byte form of the standard ASF object GUIDs (I verified each against the specification's published values — Header `75B22630-…`, Data `75B22636-…`, File Properties `8CABDCA1-…`, Stream Properties `B7DC0791-…`, both stream types `F8699E40-…`/`BC19EFC0-…`, Audio-Spread `BFC3CD50-…`, Ext-Stream-Props `C5F8CBEA-…`, etc.), the observed-vs-`[FORMAT KNOWLEDGE]` split is documented per constant, the LE reader primitives are panic-free on short buffers, and the three in-file tests are real mutation-sensitive assertions that ran green in the baseline. The problem is everything around it: the ASF demuxer this vocabulary serves is dead weight in the shipped product — no probe signature can ever name an ASF file, the container registry has no `asf` arm, and nothing outside `crates/demux` references the module. 36 passing tests exercise a lane no user can reach.

## 2. What actually works (verified)
- GUID table correctness: every constant's wire bytes hand-checked against the published ASF object GUIDs — all 15 exact (e.g. `HEADER` bytes `30 26 B2 75 …` = Data1 LE `75B22630`, matching the render assertion pinned by the test below; `CODEC_LIST` = `86D15240-311D-11D0-A3A4-00A0C90348F6`; the two `[FORMAT KNOWLEDGE]` entries — Content Description `75922EF0-…` and Command stream type `59DACFC0-…` — are also the correct spec values, not invented).
- Tests executed: baseline log 1558-1560 — `asf::guid::tests::header_guid_round_trips_from_wire_bytes`, `known_table_covers_the_verified_set`, `readers_decode_little_endian_and_tolerate_short_buffers` — all ok (demux lib suite 462/0/0).
- Panic-free reads: `Guid::read` uses `saturating_add` + `min(len)`; slice bounds guarded before every index (traced).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/asf/guid.rs:1-270 (the module it serves), evidence at crates/media-probe/src/signature/mod.rs:132-145 and apps/cli/src/containers/mod.rs:57-79
Code:
```
//! ASF GUID constants and the little-endian reader primitives
//! (card 6.ASF-a).
```
```
        registry.register(Box::new(AviSignature), DEFAULT_PRIORITY_BUILTIN);
        registry.register(Box::new(Mp4Signature), DEFAULT_PRIORITY_BUILTIN);
```
What is wrong: The ASF lane is unreachable end-to-end. The probe registry's `with_builtins()` registers 14 signatures — avi, mp4, wav, aiff, ogg, mkv, adts, ts, ps, ivf, mp3, mp2, annexb, flac — no ASF signature exists anywhere in media-probe; and even a hypothetical `asf` format id would hit the container dispatch's default arm (`other => Err(unwired_refusal(other))`, containers/mod.rs:75-79). rg for `asf` outside `crates/demux` returns nothing. The 36 passing asf tests (baseline log 1555-1590) therefore verify a demuxer no shipped entry point can construct.
Why it matters: ~dead code maintained as if shipped: every fixture, test, and line here costs review/compile time and implies a capability ("ASF/WMV/WMA support") the binary does not have. A user opening a `.wmv` gets the probe-stage `unsupported` refusal — correct behavior, but the module's existence invites doc/matrix claims of ASF support that cannot be true of any shipped build.
What correct looks like: Either wire the lane (ASF signature in media-probe + an `asf` arm in containers) or cut the module from the shipped crate behind an explicit not-wired marker; the doc here already says "card 6.ASF-a" (planned), so this is a wiring gap, not a false claim in this file.
Confidence: HIGH — Verification: rg (three independent negative searches) + read of both registries.

### F-002 [S4]
Where: crates/demux/src/asf/guid.rs:184-189 (and 193-203, 207-214)
Code:
```
pub fn u16le(data: &[u8], offset: usize) -> u16 {
    if offset + 2 > data.len() {
        return 0;
    }
    u16::from_le_bytes([data[offset], data[offset + 1]])
}
```
What is wrong: `offset + 2` overflows for `offset > usize::MAX - 2` (debug-build panic; release wrap → a wrong-but-guarded comparison), while `Guid::read` in the same file uses `offset.saturating_add(16)` for the identical caller-bounds-checks contract. The silent-zero-on-short contract is also a caller-discipline hazard: a caller that forgets to bound-check gets a plausible `0` instead of an error — the comment admits the design ("the typed truncation path fires before this is trusted") but the codebase's own panic-free sweep standard (the "8.A1" annotations elsewhere) would use saturating arithmetic uniformly.
Why it matters: Inconsistent defensive posture inside one 270-line file; the zero-return can mask an upstream bounds bug as silent data (a zero GUID/size field) rather than a loud failure.
What correct looks like: `offset.saturating_add(2) > data.len()` (or a `get(..)`-based read); same for u32le/u64le.
Confidence: HIGH — Verification: read; the overflow is unreachable through current callers (offsets come from bounded object sizes) hence S4.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Guid::from_bytes` / `as_bytes` | 29-31, 52-54 | REAL | — | const, trivial |
| `Guid::read` | 36-48 | REAL | — | saturating_add + min; short buffer → zeros; no panic path |
| `Guid::render` | 59-75 | REAL | — | LE Data1/2/3 + verbatim Data4 — matches the documented storage layout; 9 small allocs per call (observability only) |
| `Guid::known_name` | 80-99 | REAL | — | Table lookup, unknown → None (never a guessed name — honest) |
| `Display` | 102-106 | REAL | — | Delegates to render |
| 15 GUID consts | 110-178 | REAL | — | All byte-verified against the ASF spec (§2); honest observed/[FORMAT KNOWLEDGE] split; `EC_NONE` = all-zeros documented as ffmpeg's observed video-SP type |
| `u16le` / `u32le` / `u64le` | 184-214 | REAL | S4 | F-002 overflow-posture inconsistency + silent-zero contract |
| tests (3) | 220-269 | REAL | — | See §5 |

## 5. Test honesty (in-file `mod tests`; executed ok in baseline, log 1558-1560)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `header_guid_round_trips_from_wire_bytes` (221-231) | YES | Wire bytes → const equality → exact render string `75B22630-668E-11CF-A6D9-00AA0062CE6C` → name → Display. Any byte/endianness mutation fails. |
| `known_table_covers_the_verified_set` (234-256) | YES | 12 named entries + unknown → None + all-E render. Dropping a table row fails. |
| `readers_decode_little_endian_and_tolerate_short_buffers` (259-269) | YES | Pins LE values AND the short-buffer-zero contract (incl. the offset-at-tail case `u16le(&data, 7)`). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Storage layout … Data1 is a little-endian DWORD … and the Header GUID appears on disk as `30 26 b2 75 …` and renders as `75B22630-…`" | guid.rs:4-9 | REAL | Byte constants 110-112 + render impl 59-75 + round-trip test ok |
| "the GUIDs marked *observed* were byte-verified against ffmpeg 7.1.5 output this session" | guid.rs:11-13 | REAL-UNPROVEN | Values are spec-correct (§2); the ffmpeg-session verification itself is not re-runnable here (fixtures exist under src/asf/fixtures/, used by asf/tests.rs) |
| "the ones marked `[FORMAT KNOWLEDGE]` … never gate parsing" | guid.rs:14-17 | REAL (as to this file) | Neither const is referenced in guid.rs logic; gating lives in asf/mod.rs (another agent's file — verify there) |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped entry points (F-001) — every item is "dead" at the product level while alive at the test level. No intra-file dead code.

## 8. Dependencies & flags
No deps beyond `std::fmt`. No feature flags, no unsafe, no allocation hazards (render's String is error-path only).

## 9. Open questions for the next agent
- Whoever audits `crates/demux/src/asf/mod.rs`: confirm whether the `[FORMAT KNOWLEDGE]` GUIDs truly never gate parsing, and whether the module's own docs claim any wiring that F-001 refutes.
- The eight asf fixtures in my assignment are consumed only by this unreachable module's tests — their audits will carry the same UNREACHABLE verdict context.
- If a later wave wires the ASF lane (probe + containers arm), F-001 retires and the verdict flips to REAL-UNPROVEN pending an executed open.

## CORRECTION (2026-11-21, appended by W04-A05 during the fixture-family audits)
§6's row for guid.rs:11-13 ("the GUIDs marked *observed* were byte-verified
against ffmpeg 7.1.5 output this session" → REAL-UNPROVEN) and §2's "all 15
GUID constants carry the exact wire-byte form of the standard ASF object
GUIDs" are WRONG for the two error-correction constants. Byte dumps of the
committed fixtures (which I proved ARE ffmpeg 7.1.5 output by regenerating
them byte-exactly from the in-repo recipes):
- every VIDEO SP's EC field is `00 57 fb 20 55 5b cf 11 a8 fd 00 80 5f 5c
  44 2b` = 20FB5700-5B55-11CF-A8FD-00805F5C442B (the spec's no-error-
  correction GUID) — EC_NONE's all-zero value and its "observed … in ffmpeg
  output" comment (guid.rs:168-169) are FALSE;
- every AUDIO SP's EC field is `50 cd c3 bf 8f 61 cf 11 8b b2 00 aa 00 b4
  e2 20` = BFC3CD50-618F-11CF-8BB2-00AA00B4E220 — EC_AUDIO_SPREAD
  (guid.rs:165-167) drops the `00` after `AA` and appends a phantom `C0`.
The other 13 constants match the fixture bytes. known_name()'s EC rows can
never fire on real ffmpeg output. Filed as F-001 [S3] in
audit/files/crates/demux/src/asf/fixtures/asf_mjpeg.asf.audit.md with od
evidence and blast-radius analysis (no parse path branches on the EC type —
no functional impact; S3). File verdict stands (UNREACHABLE); this correction
lowers §2/§6's constant-verification claim from "spec-correct" to "13/15
correct, 2 contradicted by the fixtures".
