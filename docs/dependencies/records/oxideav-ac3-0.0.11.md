# deps/records/oxideav-ac3-0.0.11.md — oxideav-ac3 0.0.11 (purity + differential, card 6.VTA-oxideav-ac3)

> **Class:** P0 validate-then-admit SPIKE (Phase 6 W1). Candidate AC-3
> (Dolby Digital, ATSC A/52) decoder — the Phase-5 exit gate named
> oxideav-ac3 the ac3-decode candidate. This is NOT a wiring card:
> oxideav-ac3 is not added to `crates/codecs`. Everything below was
> measured in this session (2026-10-29) in the build sandbox against
> oxideav-ac3 **0.0.11** (latest release) with a harness-only AC-3
> syncwalker (0x0B77 + A/52 frame-size formula — never production code)
> supplying the frames. Harness: `tools/vta/ac3/` (standalone,
> non-workspace).

STATUS: IN PROGRESS — skeleton committed before measurement (claim
discipline). Numbers filled in as measured; see final commit.

## R1 — Version, language & source

- **oxideav-ac3 0.0.11** (crates.io, published 2026-09-01; release line
  0.0.1 → 0.0.11 over 2026-04-23 → 2026-09-01, 11 releases in ~4.5
  months; 7,142 downloads, ~6,310 recent). License **MIT**. Repo:
  https://github.com/OxideAV/oxideav-ac3 (org: OxideAV — the oxideav
  crate family). MSRV: (measured from manifest below.)

## R2 — Transitive graph

(pending)

## R3/R4 — Build scripts & native links

(pending)

## R5 — Targets

(pending)

## R6 — API surface relevant to the wiring unit

(pending)

## D — Conformance differential (measured)

(pending)

## H — Hostile input (measured)

(pending)

## Verdict

(pending)
