# CLAIM AUDIT: docs/formats/subtitle-kate.md
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R5) |
| Claims extracted | 7 |
| REAL 6 / UNVERIFIED 1 (+1 numeric nit) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Honest blocked-decision record with a genuinely sharp landed-state finding that STILL holds
today: the Ogg demuxer's `OggCodec` enum has no Kate variant and Kate streams fall to
`Unknown` → "ogg-opaque" (generic opaque track, not a typed refusal naming kate) — exactly
the gap the record describes, still unfixed, and the matrix's Ogg note still carries the
stale "Kate mapping" wording now with the record's own "5.P2b-I flag" appended. Matrix
blocked ×4 is byte-exact. VLC's kate spu decoder exists in the census (22 registrations —
the doc says 23; off-by-one nit). crates.io facts offline-unverifiable.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row subtitle-kate: blocked ×4 | :17/:72 | REAL | feature-matrix.toml:1048-1057 `status = { windows="blocked", linux="blocked", macos="blocked", web="blocked" }`, evidence `""` (rg). |
| 2 | OggCodec has no Kate variant; Kate stream (magic \x80kate\0) falls to Unknown → descriptor "ogg-opaque" (not typed refusal) | :22-27 | REAL (still so) | crates/demux/src/ogg/mod.rs:126 `pub enum OggCodec` — no Kate variant (rg "kate" -> 0 real hits); :145 `Unknown,`; :157 `OggCodec::Skeleton | OggCodec::Unknown => "ogg-opaque"` (sed-verified). |
| 3 | Matrix Ogg note claims "carries Vorbis/Opus/Theora/Kate/Speex/FLAC-in-Ogg mappings" — stale for Kate; drift flagged | :22-33 | REAL (flagged, note not edited) | feature-matrix.toml:165 notes still say "carries ... Kate ... mappings" + appended "Kate mapping note stale (5.P2b-I flag)" — the Lead recorded the drift flag rather than editing, matching the record's deliberate non-edit. |
| 4 | VLC ships its own kate spu decoder (modules-linux.json spu-decoder list, 23 registrations) | :46-47 | REAL (count nit: 22) | python3 census: by_capability["spu decoder"] count = 22, names include `kate` (with subsdec/libass/spudec/webvtt/dvbsub...). The "23" is off by one (22 actual); kate's presence — the load-bearing fact — is exact. |
| 5 | formats.json subtitle-census-tail is census-only not-probe-verified; no probe fixture | :48-50 | REAL | formats.json carries subtitle-census-tail family; no Kate probe fixture in docs/baselines (rg kate in probe logs -> none). |
| 6 | ADR-008 not yet written — honest gap, Lead owns | :82-84 | REAL (still open) | ls docs/adr -> no ADR-008 today (001-006, 010-013 only). |
| 7 | crates.io: kate/libkate/ogg-kate/kate-rs/katesub all NOT_FOUND; keyword search shows no subtitle-relevant crate | :37-41 | UNVERIFIED | offline sandbox, no registry query; consistent with the 2026-10-24 evidence-date framing. |

## 3. Contradictions
None. The record rejects both self-build and full-block in favor of the honest typed
refusal, and correctly identifies that even the honest refusal is NOT yet wired (claim 2
still holds).

## 4. Fabrication / inflation indicators
None — the record's central contribution is admitting its own ecosystem's gap (opaque
passthrough instead of the promised typed refusal).

Commands run: rg subtitle-kate + container-ogg notes in matrix; rg "kate|Kate" + enum/Unknown/ogg-opaque in crates/demux/src/ogg/mod.rs (sed :126-176); python3 spu-decoder census (22, kate present); ls docs/adr.
