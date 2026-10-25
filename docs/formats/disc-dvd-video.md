# Format record — DVD-Video (menus/titles/angles navigation, IFO/VOB/CSS)

Status: **blocked (v1, and absent a policy change blocked v2+) —
decision record (card 5.P2b-I part B)**, per feasibility subsystems
§7.3 μADR-C: **no E2 path exists** — no modern OS ships DVD navigation
as a service (Windows DirectShow DVD Navigator legacy/deprecated,
macOS DVDPlayback deprecated, Linux none `[VERIFY status — feasibility
μADR-C marks]`), and libdvdnav/libdvdread/libdvdcss are third-party C
libraries — a forbidden-class exception requiring a **policy change,
not an audit pass** (AGENTS/MP §14; doc 11 territory). CSS
descrambling adds regional circumvention-law exposure (ADR-010 input).

- Date of evidence: **2026-10-24** (crates.io API, this card).
- Supersedes-cites: doc 04 §2 MPEG-PS paragraph ("DVD-Video navigation
  (IFO/VOB menus) is a separate, C-library-bound problem — ADR
  territory per §14, not in this roadmap's v1 scope") + §8.6 item 9,
  feasibility subsystems §7.3 μADR-C (2026-10-19), feature-matrix row
  `platform-disc-dvd` (blocked ×3 + web unsupported).

## Parse / Decode / Encode

Nothing implemented for navigation, and none scheduled. **What v1 ships
instead (already-decided surface, per μADR-C and existing rows):**
(1) **VOB/PS file demux** — MPEG-PS parser exists in tree
(`crates/demux/src/ps`; matrix row `container-mpeg-ps`, doc 04 §2
"MPEG-PS … DVD-VOB private streams (AC-3/DTS/LPCM) exposed") so a
user opening a `VTS_01_1.VOB` from a mounted or unencrypted disc gets
streams + typed decode statuses (AC-3/DTS pending 5.K4-I's outcomes);
(2) **VobSub subtitles** — `vobsub` 0.2.3 (CC0) harden path recorded
under `subtitle-vobsub`'s row (deps are 2017-era — fork-level harden
`[VERIFY]`); (3) honest "disc navigation not supported" behavior.
Optional μADR-C extra, **not** scheduled: S-sized IFO *metadata*
parsing (durations/chapters for the media-info panel) with an explicit
no-navigation claim — recorded as `[PROPOSAL]`, owned by a future card.

## Evidence (what was checked, 2026-10-24)

- **crates.io direct names:** `dvdnav`, `dvdread`, `libdvdcss`,
  `dvdnav-rs`, `ifo`, `ifo-parser`, `dvd-video` — **all NOT_FOUND**.
  `dvd` EXISTS but is a **terminal DVD-screensaver toy** (0.0.0,
  2024-11-27, "classic dvd screensaver ported to a terminal") —
  name-collision disambiguation recorded. The feasibility μADR-C's
  `dvdcss` 0.1.2 (2021, Rust bindings **to libdvdcss C**) stands:
  E1-class third-party C, forbidden.
- **VLC handling (oracle):** `dvdnav` + `dvdread` access_demux modules
  (modules-linux.json census); formats.json has no dedicated
  dvd-navigation family (the `platform-disc-dvd` row's
  `inventory_ref = formats.json#disc-family` points at a key that does
  not exist in the file — honest inventory-ref drift, flagged for the
  Lead; the closest real entries are `mpeg-ps` (VOB payload, probed)
  and `subtitle-census-tail` (spudec)).
- **ADR-008 gap (honest):** both the matrix row and μADR-C assign
  scope ownership to "ADR-008 (optical disc scope)" — **ADR-008 is not
  yet written** (docs/adr contains 001–006 + 011 only). The decision
  is *recorded* here and in μADR-C, but the formal ADR remains open
  Lead work; this record does not close it.

## Bit depths / layouts

N/A (blocked). VOB payload formats are MPEG-PS/MPEG-2/AC-3/LPCM —
those families' records own the details.

## Tracks / Seeking / Timestamps

N/A for navigation. VOB-file seeking rides the MPEG-PS parser's
existing semantics.

## Corrupt-input behavior

N/A (no DVD parser ships).

## Limits / Platforms

Matrix: blocked ×3 + web `unsupported` (no disc access in browsers —
doc 09). Unchanged by this card.

## Corpus

None — no disc lane in corpus-v4 (physical-media oracle work was out
of Phase-0 scope); any future navigation work would need a C-V6-class
disc-image acquisition plan with its own legal review.

## Results

None (no code). Next step: **Lead writes ADR-008 (optical disc scope)
with ADR-010 legal input — the μADR-C text in feasibility subsystems
§7.3 is its draft brief; until then blocked stands and the v1 surface
is the VOB-file path above.** No crates.io recheck is meaningful
(navigation is C-library-bound, not crate-availability-bound).
