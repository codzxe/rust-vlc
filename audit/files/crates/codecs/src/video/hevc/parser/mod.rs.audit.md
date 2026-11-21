# AUDIT: crates/codecs/src/video/hevc/parser/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Lines / functions | 37 / 5 (module declarations: nal, pps, slice, sps, vps) — documentation-only module root |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — the entire parser tree's only consumers are `hevc/decode.rs` (`HevcDecoder`, itself never constructed outside `crates/codecs/tests/hevc_hw_path.rs`/`hevc_parser_corpus.rs`), `hevc/fuzz.rs`, `demux/src/es/fuzz.rs` and corpus tests. The shipped CLI's decoder table has NO hevc arm (apps/cli/src/pipeline_host.rs:241-346); raw `.265` files refuse decode typed (`apps/cli/src/containers/es.rs:463-484`, `cli.codec_unavailable`); baseline `mp4-smoke-open-session-hevc.txt`: Opening → Error |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg "HevcDecoder|hevc::parser" repo-wide (consumers = tests/fuzz only); rg parse_with_buffer across the five submodules (conventions claim); baseline log :630-650 (hevc parser unit tests executed); cross-read of my own sps.rs audit findings against this module's claims |

## 1. One-paragraph verdict
A 37-line module root: five `pub mod` declarations plus an accurate module map
and conventions section. Every claim it makes about its submodules checks out
(verified item by item below), including the honest-subset scoping of
`slice.rs` and the reusable-buffer convention. The one thing the doc cannot
say — and does not — is that the whole tree is unreachable from any shipped
entry point. Zero findings; what was checked and how is in §2. NOTE: the
submodule-level honesty claims that DO fail (sps.rs's "bit-accurate walk" for
scaling lists, the VVC-flag bitstream-restriction read, inter-RPS NumDeltaPocs)
are filed where they live: `audit/files/crates/codecs/src/video/hevc/parser/
sps.rs.audit.md` F-001/F-002/F-003 — this root's own prose does not repeat
those claims, so it does not inherit the verdicts.

## 2. What was checked and how (zero-findings file)
— Module map vs reality: `nal` does own the two-byte header/framing/EPB/
  classification/hvcC/BitReader (baseline :632-641 — its tests executed:
  annexb splits, emulation removal, hvcc field-exact, length-prefixed splits,
  Table 7-1 classification); `vps` owns ProfileTierLevel + HRD walker
  (referenced by sps.rs:10 imports — verified by read of the import);
  `pps`/`sps`/`slice` exist with the described surfaces (parse_with_buffer
  present in vps.rs:318/pps.rs:111 — the "no allocation per NAL" convention
  is implemented, rg).
— "All parsers return typed errors: no panic, no hang, no decode claim":
  supported by the executed fuzz-lite lanes (baseline :630-631
  `empty_and_marker_inputs_survive`, `hostile_wide_inputs_survive` — ok) and
  by my read of sps.rs (all paths typed, all loops bounded). The sps.rs walk
  BUGS produce wrong-but-typed results; the "no panic, no hang" claim itself
  survives.
— "complete NAL units … unless a name says rbsp": consistent with the
  submodule signatures I read (sps.rs:204 takes `nal: &[u8]`, splits the
  two-byte header at 205).
— Dead code: none in this file; the tree-level unreachability is a fact of
  the shipped decoder table, not of this module's contents.
— Dependencies & flags: no cfg gates, no unsafe, no deps; unconditional via
  `crates/codecs/src/video/mod.rs:30-32`.

## 3. Findings
None. (See §2 for the verification list; submodule defects are filed in
sps.rs.audit.md.)

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod nal` | 33 | REAL (code) / UNREACHABLE (lane) | — | consumers: hevc/decode.rs (dead) + tests/fuzz |
| `pub mod pps` | 34 | REAL / UNREACHABLE | — | as above |
| `pub mod slice` | 35 | REAL / UNREACHABLE | — | "minimal honest subset" scoping is accurate |
| `pub mod sps` | 36 | REAL / UNREACHABLE | — | my audited file; 3×S2/3×S3/1×S4 inside |
| `pub mod vps` | 37 | REAL / UNREACHABLE | — | other agent's file (w06 assignment) |
| module docs | 1-31 | REAL | — | every claim verified (§2) |

## 5. Test honesty
No tests in this file; the tree's tests live in the submodules and
`crates/codecs/tests/hevc_parser_corpus.rs` (3 passed, baseline :1109-1117 —
but only over the 0044/0100/0091 corpus lanes; see sps.rs.audit.md §5 for the
shapes those lanes do NOT cover).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Module map (nal/vps/sps/pps/sps/slice contents) | mod.rs:4-22 | REAL | verified per §2 |
| "All parsers return typed errors: no panic, no hang, no decode claim" | mod.rs:30-31 | REAL | fuzz-lite executed ok (:630-631); read of sps.rs |
| "steady-state parsing … performs no allocation per NAL" | mod.rs:29-30 | REAL | parse_with_buffer forms exist (rg) |
| (implicit) the parser serves shipped HEVC playback | — | FALSE by omission | no decoder arm reaches it; HEVC open refuses typed (baseline mp4-smoke-open-session-hevc.txt) — the doc never claims playback, so this is context, not a doc lie |

## 7. Dead / duplicated / unreachable code
The whole tree (this root included) is unreachable from shipped entry points.
Chain of proof in the header row; identical conclusion to my sps.rs audit
(§7 there).

## 8. Dependencies & flags
No feature gates; `pub mod hevc` unconditional (crates/codecs/src/video/mod.rs:30-32),
compiled into every shipped `codecs` consumer; nothing unsafe at this level.

## 9. Open questions for the next agent
- nal.rs/vps.rs/pps.rs/slice.rs owners (wave 6 peers): the module map's
  promises are the contract your file is audited against.
- If a future wave registers `HevcDecoder` in the CLI, this root's
  "typed errors" story stays true but sps.rs F-001/F-002/F-003 turn into
  user-facing rejections — fix order matters.
