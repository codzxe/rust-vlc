# CLAIM AUDIT: docs/formats/disc-dvd-video.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R5) |
| Claims extracted | 7 |
| REAL 5 / TRUE-at-time-since-fixed 1 / UNVERIFIED 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Honest blocked-decision record whose checkable claims verify: matrix blocked ×3 + web
unsupported is byte-exact; the MPEG-PS parser it names as v1's DVD surface exists; the VLC
oracle modules (dvdnav/dvdread) are in the census; ADR-008 remains unwritten today exactly
as the record says (its "Lead writes ADR-008" next step is still open). One claim inverts
in the honest direction: the record flagged the inventory-ref `formats.json#disc-family`
as pointing at a nonexistent key — the key EXISTS today (post-record addition; dating is
impossible without git), i.e. the drift it flagged was subsequently healed. crates.io facts
are offline-unverifiable.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row platform-disc-dvd: blocked ×3 + web unsupported | :17-18/:74 | REAL | feature-matrix.toml:2524-2532 `status = { windows="blocked", linux="blocked", macos="blocked", web="unsupported" }` (rg/sed-verified). |
| 2 | inventory_ref formats.json#disc-family points at a key that does not exist — drift flagged for the Lead | :47-51 | TRUE-at-time-since-fixed | python3 scan today: `"family": "disc-family"` EXISTS in docs/inventory/formats.json (with cdda/vcd/dvdnav module entries) — the flagged drift was healed post-record (addition undatable, no git). |
| 3 | VOB/PS demux ships: MPEG-PS parser in crates/demux/src/ps; matrix row container-mpeg-ps | :24-27 | REAL | ls crates/demux/src -> ps present; formats.json family "mpegps" (container, demux) exists; doc's "mpeg-ps" spelling is a nit — the entry is real. |
| 4 | VobSub subtitles row exists (vobsub 0.2.3 CC0 harden path [VERIFY]) | :29-30 | REAL | feature-matrix.toml:1035 `id = "subtitle-vobsub"` status implemented ×3 — the recorded harden path landed as implemented. |
| 5 | ADR-008 (optical disc scope) is not yet written; record does not close it | :52-56/:85-87 | REAL (still open) | ls docs/adr -> ADR-001..006, 010..013 — no ADR-008 today; the record's honesty about the open ADR is accurate on audit day. |
| 6 | VLC oracle: dvdnav + dvdread access_demux modules (modules-linux.json census) | :45-46 | REAL | audit/runs/W18-A04.modules-linux.json: `"name": "dvdnav"` (:474/:2575) and `"name": "dvdread"` (:2591) — present in the saved census. |
| 7 | crates.io: dvdnav/dvdread/libdvdcss/ifo... all NOT_FOUND; `dvd` = terminal screensaver toy | :38-44 | UNVERIFIED | offline sandbox, no registry query; the E1-forbidden-class reasoning (C-library bindings) is policy-consistent either way. |

## 3. Contradictions
None. The blocked verdict is consistent across matrix row, ADR-008 absence, and the
documented v1 fallback surface (VOB file demux).

## 4. Fabrication / inflation indicators
None — the record shrinks its own scope ("this record does not close it") and flags its
own inventory drift rather than papering over it.

Commands run: rg platform-disc-dvd + status/inventory_ref + subtitle-vobsub in matrix; ls crates/demux/src/ps, docs/adr; python3 scans of formats.json (disc-family/mpegps/spudec contexts); rg dvdnav/dvdread in modules-linux.json.
