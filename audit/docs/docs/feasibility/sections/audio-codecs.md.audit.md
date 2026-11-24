# CLAIM AUDIT: docs/feasibility/sections/audio-codecs.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED-net 3 (folded below) / UNREACHABLE 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A dated (2026-10-19) snapshot whose verdicts map 1:1 onto today's shipped Cargo.toml:
every "software-decode" row corresponds to a pinned crate with a purity record, and the
two "blocked" rows that later unblocked (Opus) carry a traceable supersession trail
(Lead admission + dependency record). The VLC census figure it quotes reconciles exactly
with the committed inventory JSON. The only unverifiable layer is the crates.io [RV]
network evidence itself, which the doc honestly labels with its method and date.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "`audio decoder` capability = 18 registrations across 16 real decoders" | :9-10 | REAL | modules-linux.json by_capability['audio decoder'].count = 18 (python3); 16 real after deduping 2 ddummy registrations |
| 2 | PCM row: `symphonia-codec-pcm` software-decode | §1 row 1 | REAL (superseded-in-part) | codecs crate ships unpacked-PCM natively (Cargo.toml description:36 "unpacked-PCM audio decoder"); symphonia family pinned |
| 3 | MP3/AAC/ALAC/Vorbis rows: symphonia crates, software-decode | §1 rows 2,4,8,9 | REAL | crates/codecs/Cargo.toml:60,61,78,80 — codec-mp3/aac/alac/vorbis = dep:symphonia-* (each with a purity record in docs/dependencies/records/) |
| 4 | FLAC row: symphonia-bundle-flac + `claxon` oracle | §1 row 7 | REAL (supersession) | shipped engine is claxon: `codec-flac = ["dep:claxon"]` + `claxon = { version = "=0.4.3" }` (:68,86) + purity record claxon-0.4.3.md |
| 5 | Opus row: "**None published.** ... blocked (v1)" | §1 row 10 | REAL-then-superseded | `ruopus = { version = "=0.1.2" }` (Cargo.toml:137) + records/ruopus-0.1.2.md + 6.M-opus Lead admission — the doc's date-stamped verdict was true; supersession traceable in-tree |
| 6 | forbidden C bindings (minimp3/opus/audiopus) | §1 rows 2,10 | REAL | none of those crates appear in codecs Cargo.toml; C-binding discipline held |
| 7 | [RV] crates.io API re-verification "2026-10-19" | Method § | UNVERIFIED | offline today; the doc's own [RV]/[VERIFY] labeling is honest |
| 8 | "Purity designations remain 'verified promising, not approved'" | :24-26 | REAL | dependency records exist for every shipped dep cited; approval workflow traceable |

## 3. Contradictions
None. The doc is explicitly dated; the two rows whose world changed (Opus, and FLAC's
engine choice) have in-tree supersession trails rather than silent drift.

## 4. Findings
F-001 [S4] The "blocked (v1)" verdict for Opus is stale vs shipped state (ruopus landed
2026-11-15-class admissions) — but unlike doc 04's gap register, this doc's header dates
every verdict and the supersession is one rg away. Confidence: HIGH — Verification: read+rg.

## 5. Prior-audit reconciliation
Dead W19-A09's audit (old tree) — "verdicts map 1:1 to later admissions (symphonia
feature-gated w/ purity records, claxon, ruopus for formerly-blocked Opus — traceable
supersession); [RV] claims UNVERIFIED; no findings" — independently reconfirmed, plus my
new exact reconciliation of the 18-registration census figure.
