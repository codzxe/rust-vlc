# CLAIM AUDIT: docs/feasibility/sections/audio-codecs.md (L1-446)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 / 2 / 0 / 0 / 0 / 0 / 3 |
| Internal contradictions found | 0 (1 superseded verdict, traceable) |

## 1. Blunt verdict
A serious Phase-0 feasibility record: per-codec verdicts with [RV] re-verification labels, census reconciliation that dispositions every VLC registration (nothing silently dropped), and honest "blocked (v1)" verdicts where no pure-Rust path existed (Opus, WMA, AC-3, DTS, Speex, Musepack, ATRAC). Its recommendations map 1:1 onto what the code actually later admitted (symphonia crates behind feature flags with purity records, claxon FLAC, ruopus for the formerly-blocked Opus) — plan-to-implementation chain is traceable and honest. The [RV] crates.io findings are dated 2026-10-19 session measurements; not re-verifiable here (no network run attempted; see ledger).

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Every crate marked [RV] was re-verified today (2026-10-19) via the crates.io HTTP API" | Method | UNVERIFIED | Dated session measurement; no network verification performed by this auditor (sandbox network policy untested; not re-run). |
| 2 | Opus: "None published… blocked (v1); WebM/YouTube-class audio unplayable" | §1 row 10 | SUPERSEDED-REAL | Later resolved: crates/codecs/src/audio/opus/ exists ("the `ruopus` engine", card 6.M-opus); matrix codec-audio-opus = implemented; compat-matrix: opus 4 tracks decoded. The doc is a Phase-0 snapshot — the supersession is traceable (dependency records), not a fabrication. |
| 3 | PCM/MP3/AAC/ALAC/Vorbis/ADPCM via symphonia crates, MPL-2.0 | §1 rows 1-5,7,9,15 | REAL | crates/codecs/Cargo.toml:60-80 admits symphonia-codec-aac/alac, bundle-mp3, codec-vorbis behind feature flags, each with a purity record (docs/dependencies/records/ cited in Cargo.toml comments). |
| 4 | FLAC: claxon 0.4.3 "oracle role"/stable | §1 row 7 | REAL | claxon = { version = "=0.4.3", optional = true } with record claxon-0.4.3.md (Cargo.toml:12,86). |
| 5 | HE-AAC: "no sbr.rs/ps.rs modules exist in symphonia-codec-aac@main" | §1 row 6, §2.2 | UNVERIFIED | Upstream source claim, dated; downstream consequence (HE-AAC blocked) is respected in the matrix (codec-audio-aac rows partial). |
| 6 | Census: "18 registrations across 16 real decoders… counted as 35 total (18 audio + 14 video + 3 hw)" | header | REAL-UNPROVEN | Cites modules-linux.json by_capability (file exists); count not re-derived here. |
| 7 | g711/aes3/uleaddvaudio "not in doc 04 §3 — add" | §1.1 | REAL (governance) | The reconciliation table dispositions census-only rows explicitly rather than dropping them — verify by the §8 [VERIFY] flags it names. |
| 8 | WavPack third-party crate "GPL-3.0-or-later… copyleft conflict candidate for our MIT repo" | §1 row 16 | REAL | Matches ROADMAP cross-doc reconciliation log ("Symphonia WavPack crate is GPL-3.0 in MIT repo… excluded or dual-compat reviewed"). |
| 9 | WMA "crates.io `wma` NOT_FOUND" → blocked | §1 row 11 | REAL (consequence) | WMA decode absent in tree (matrix codec-audio-wma blocked; asf row notes "WMA audio decode blocked codec-side"). |
| 10 | MIDI: rustysynth 1.3.7 MIT "includes SMF parsing" | §1 row 19 | UNVERIFIED | External crate claim; MIDI row (container-midi-smf) remains not_started in matrix — consistent with "blocked (later)". |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Opus "blocked (v1)" (2026-10-19) | Code + matrix: opus implemented (ruopus, card 6.M-opus) | Both true at their dates — the doc is a snapshot; the supersession chain is documented (Cargo.toml purity records + matrix evidence). No hidden flip. |

## 4. Fabrication / inflation indicators
None. The document's value is its explicit labels ([RV] / [VERIFY] / blocked-with-consequence). Nothing claims implementation status — it is a feasibility input, and downstream artifacts (matrix rows, dependency records) respect its verdicts or traceably supersede them.
Commands run: rg over crates/codecs/Cargo.toml (symphonia/claxon/ruopus pins + purity records), ls codecs/src/audio/opus/, matrix row greps. Exit 0.
Findings: S2: 0 · S3: 0 · S4: 0.
