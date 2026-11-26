# w26 verification captures — the committed inventory (GENERATED)

> GENERATED FILE — do not edit by hand. Regenerate:
> `scripts/quality/w26_captures.sh index-gate --write`
> Drift gate (exit 1 when this file disagrees with a fresh scan of
> `docs/waves/w26/**`): `scripts/quality/w26_captures.sh index-gate`

Why this inventory exists (FX8-A06, FIX-PLAN §4 F2.4): the Devil Audit's
process-failure finding #7 (audit/final/DEVIL-AUDIT-REPORT.md:112) —
"Evidence lives in /tmp and .gitignore. w26 verification captures never
committed" — every capture artifact the wave-26 docs cite below lived
OUTSIDE the repo (in /tmp or an agent scratch dir under /home/z) and no
bytes were ever committed.

NO original capture bytes exist to recover. /tmp is recycled and the agent
scratch dirs are gone; nothing in this directory fabricates bytes. This
INDEX is the committed record of WHAT the docs cite and HOW the class is
regenerated — the bytes come back only by running the regen, never by
trusting this list.

## Regen basis — the committed tool, in-tree-only output

- Capture tool: `ui/scripts/dev-shots.mjs` (committed — the wave-26 agent
screenshot tool, w26 contract §4.7/§5). The regen harness
`scripts/quality/w26_captures.sh capture` drives it over the 25-route truth
of `ui/e2e/visual/manifest.ts` (ROUTES) with `--out
docs/waves/w26/captures/regen/` — regenerated captures land IN-TREE, never
/tmp-only.
- The regen needs the UI stack (node + ui/node_modules/playwright + the dev
server on :5173). `scripts/quality/w26_captures.sh probe` is check-only;
`capture` probes first and fails loud (exit 1) when the stack is missing —
it never pretends a regen ran.
- Capture diff: `scripts/quality/w26_captures.sh compare DIR_A [DIR_B]` —
byte-level (md5) comparison of two capture sets.
- The `.gitignore` exceptions `!docs/waves/w26/captures/**` and
`!docs/waves/w26/captures/*.log` keep everything under this directory
committable — including the `console-*.log` artifacts dev-shots.mjs writes
when a state logs console errors, which the global `*.log` rule would
otherwise ignore.
- What the regen produces: the capture CLASS — screenshots of the routed
surface plus console-error logs — not the historical filenames one-for-one
(many rows below are one-off session scripts and reports whose bytes never
existed in the repo; rows of kind shot/log/report under a shots-*/cap-*
session dir are the rows the regen class reproduces).

## The scan rule (what the drift gate re-runs)

Corpus: every file under docs/waves/w26/** except this INDEX (a generated
file never scans itself). A citation is a token rooted at /tmp/ or /home/z/
(agent scratch space) in a conservative charset: balanced {...} brace
expansions, commas and * globs survive; the token stops at whitespace,
backtick, quotes, parens, brackets, angle brackets, pipe and backslash.
Normalization: an unbalanced '{' (prose fragment) cuts the token at the
brace; trailing prose punctuation and slashes are stripped; placeholder
fragments (tokens ending in '-') are dropped; /home/z/my-project (the repo
itself) and /home/z/node_modules (environment, not an artifact) are dropped;
a bare file straight under /home/z counts only when its name says w26. Kind:
known extension of the last segment (png→shot, log/txt→log, json→report,
mjs/js/cjs/py/html/ts→tool), any * or {...} → set, anything else → dir (a
session directory citation; '.e01' in shots-w26.e01 is the session id, not
an extension).

## Summary

- corpus scanned: 174 files under docs/waves/w26/
- citing docs: 107
- distinct cited artifacts: 291
- by kind: shot 21, log 3, report 26, tool 59, dir 117, set 65

## Inventory — 291 cited artifacts, 107 citing docs

Citing docs are paths relative to docs/waves/w26/. No row implies
the bytes exist anywhere — see the honesty note above.

| artifact | kind | cited by |
|---|---|---|
| `/home/z/.w26a09` | dir | logs/w26.a09.md |
| `/home/z/cap-w26.i12/*.mjs` | set | logs/w26.i12.md |
| `/home/z/cap-w26.i12/probe.json` | report | logs/w26.i12.md |
| `/home/z/cap-w26.i12/vision-*.txt` | set | logs/w26.i12.md |
| `/home/z/cap-w26.i15/probe.json` | report | logs/w26.i15.md |
| `/home/z/cap-w26.i15/probe.mjs` | tool | logs/w26.i15.md |
| `/home/z/cap-w26.i15/vision-*.txt` | set | logs/w26.i15.md |
| `/home/z/cap-w26g10` | dir | findings/w26.g10.md |
| `/home/z/cap-w26g10/{global-1,global-dropdown-open,interface-state,*-1024,*-light}.png` | set | findings/w26.g10.md |
| `/home/z/cap-w26h03/capture.mjs` | tool | ratings/w26.h03.md |
| `/home/z/cap-w26h03/{capture,internals,framecomp}.mjs` | set | ratings/w26.h03.md |
| `/home/z/cap-w26h04` | dir | ratings/w26.h04.md |
| `/home/z/cap-w26h04/cap1..8` | dir | ratings/w26.h04.md |
| `/home/z/cap-w26h08/*.json` | set | ratings/w26.h08.md |
| `/home/z/cap-w26h11/probe*.json` | set | ratings/w26.h11.md |
| `/home/z/cap-w26h11/probe*.mjs` | set | ratings/w26.h11.md |
| `/home/z/cap-w26i02` | dir | logs/w26.i02.md |
| `/home/z/cap-w26i03` | dir | logs/w26.i03.md |
| `/home/z/cap-w26i04` | dir | logs/w26.i04.md |
| `/home/z/cap-w26i04/intercept-lib.mjs` | tool | logs/w26.i04.md |
| `/home/z/cap-w26i13` | dir | logs/w26.i13.md |
| `/home/z/cap-w26i14r3/{verify1,verify2,light,anatomy,pitch,inherit,clips,states}.mjs` | set | logs/w26.i14.md |
| `/home/z/cap-w26l01/{onb,onb2,player}.mjs` | set | findings/w26.l01.md |
| `/home/z/cap-w26l04/capture1–7.mjs` | tool | findings/w26.l04.md |
| `/home/z/h02-scratch/capture.mjs` | tool | ratings/w26.h02.md |
| `/home/z/k04-verify/k04-live-verify.mjs` | tool | logs/w26.k04.md |
| `/home/z/probe-w26-e05.mjs` | tool | findings/w26.e05.md |
| `/home/z/probe-w26-k11.mjs` | tool | logs/w26.k11.md |
| `/home/z/scratch-w26k08/measure-k08.mjs` | tool | logs/w26.k08.md |
| `/home/z/shots-w26.f08` | dir | logs/w26.f08.md |
| `/home/z/tmp-f07/w26f07-clamp.mjs` | tool | logs/w26.f07.md |
| `/home/z/tmp-w26g20/capture*.mjs` | set | findings/w26.g20.md |
| `/home/z/tmp-w26h18/capture.mjs` | tool | ratings/w26.h18.md |
| `/home/z/tmp-w26h19/probe{,2,3}.mjs` | set | ratings/w26.h19.md |
| `/home/z/tmp-w26h20/flow.mjs` | tool | ratings/w26.h20.md |
| `/home/z/tmp-w26h20/{flow,probe,probe2,probe3,probe4}.mjs` | set | ratings/w26.h20.md |
| `/home/z/tmp-w26i16` | dir | logs/w26.i16.md |
| `/home/z/tmp-w26i17/probe{,2}.mjs` | set | logs/w26.i17.md |
| `/home/z/tmp-w26i18` | dir | logs/w26.i18.md |
| `/home/z/tmp-w26i18/probe{,2}.mjs` | set | logs/w26.i18.md |
| `/home/z/verify-w26e03.mjs` | tool | findings/w26.e03.md |
| `/home/z/w26a04-*.mjs` | set | logs/w26.a04.md |
| `/home/z/w26c1-{shot,probe*}.mjs` | set | logs/w26.c1.md |
| `/home/z/w26d1-tmp/w26d1-audit.mjs` | tool | logs/w26.d1.md |
| `/home/z/w26d2-clip.mjs` | tool | logs/w26.d2.md |
| `/home/z/w26d2-shots.mjs` | tool | logs/w26.d2.md |
| `/home/z/w26e04-queue-geo.mjs` | tool | findings/w26.e04.md |
| `/home/z/w26e04-shots.mjs` | tool | findings/w26.e04.md |
| `/home/z/w26f08-verify.mjs` | tool | logs/w26.f08.md |
| `/home/z/w26i01-scratch/capture.mjs` | tool | logs/w26.i01.md |
| `/tmp/cap-gfixA/shots` | dir | logs/w26.gfix-A.md |
| `/tmp/cap-gfixB` | dir | logs/w26.gfix-B.md |
| `/tmp/cap-gfixC` | dir | logs/w26.gfix-C.md |
| `/tmp/cap-gfixD` | dir | logs/w26.gfix-D.md |
| `/tmp/cap-gfixE` | dir | logs/w26.gfix-E.md |
| `/tmp/cap-gfixF` | dir | logs/w26.gfix-F.md |
| `/tmp/cap-w26g02` | dir | findings/w26.g02.md |
| `/tmp/cap-w26g03/dom-dump.js` | tool | findings/w26.g03.md |
| `/tmp/cap-w26g03/{dom-dump,dom-1024,font-check}.js` | set | findings/w26.g03.md |
| `/tmp/cap-w26g04` | dir | findings/w26.g04.md |
| `/tmp/cap-w26g06` | dir | findings/w26.g06.md |
| `/tmp/cap-w26g07` | dir | findings/w26.g07.md |
| `/tmp/cap-w26g08` | dir | findings/w26.g08.md |
| `/tmp/cap-w26g08/*.mjs` | set | findings/w26.g08.md |
| `/tmp/cap-w26g08/geom-{1,2,3,4}.txt` | set | findings/w26.g08.md |
| `/tmp/cap-w26g08/shots` | dir | findings/w26.g08.md |
| `/tmp/cap-w26g09` | dir | findings/w26.g09.md |
| `/tmp/cap-w26g11` | dir | findings/w26.g11.md |
| `/tmp/cap-w26g11/playback-1.png` | shot | findings/w26.g11.md |
| `/tmp/cap-w26g12/*.mjs` | set | findings/w26.g12.md |
| `/tmp/cap-w26g13` | dir | findings/w26.g13.md |
| `/tmp/cap-w26g13/extract.log` | log | findings/w26.g13.md |
| `/tmp/cap-w26g14` | dir | findings/w26.g14.md |
| `/tmp/cap-w26g15` | dir | findings/w26.g15.md |
| `/tmp/cap-w26g15/report.json` | report | findings/w26.g15.md |
| `/tmp/cap-w26g16` | dir | findings/w26.g16.md |
| `/tmp/cap-w26g16/shots` | dir | findings/w26.g16.md |
| `/tmp/cap-w26g17` | dir | findings/w26.g17.md |
| `/tmp/cap-w26g18/capture{,2,3,4,5,6}.mjs` | set | findings/w26.g18.md |
| `/tmp/cap-w26g19` | dir | findings/w26.g19.md |
| `/tmp/cap-w26g19/shots/measure-{real,harness}.json` | set | findings/w26.g19.md |
| `/tmp/cap-w26g20` | dir | findings/w26.g20.md |
| `/tmp/cap-w26g20/measurements*.json` | set | findings/w26.g20.md |
| `/tmp/cap-w26h03/report.json` | report | ratings/w26.h03.md |
| `/tmp/cap-w26h06/drive.js` | tool | ratings/w26.h06.md |
| `/tmp/cap-w26h07/capture{,2,3}.mjs` | set | ratings/w26.h07.md |
| `/tmp/cap-w26h09/*.mjs` | set | ratings/w26.h09.md |
| `/tmp/cap-w26h09/video-codecs.png` | shot | ratings/w26.h09.md |
| `/tmp/cap-w26h12` | dir | ratings/w26.h12.md |
| `/tmp/cap-w26h13` | dir | ratings/w26.h13.md |
| `/tmp/cap-w26h13/capture.js` | tool | ratings/w26.h13.md |
| `/tmp/cap-w26h14` | dir | ratings/w26.h14.md |
| `/tmp/cap-w26h15` | dir | ratings/w26.h15.md |
| `/tmp/cap-w26h16/capture3.mjs` | tool | ratings/w26.h16.md |
| `/tmp/cap-w26h17` | dir | ratings/w26.h17.md |
| `/tmp/cap-w26h19/measurements{,2,3}.json` | set | ratings/w26.h19.md |
| `/tmp/cap-w26i02/report.json` | report | logs/w26.i02.md |
| `/tmp/cap-w26i04` | dir | logs/w26.i04.md |
| `/tmp/cap-w26i04/probes.json` | report | logs/w26.i04.md |
| `/tmp/cap-w26i05/capture.mjs` | tool | logs/w26.i05.md |
| `/tmp/cap-w26i05/capture2.mjs` | tool | logs/w26.i05.md |
| `/tmp/cap-w26i06` | dir | logs/w26.i06.md |
| `/tmp/cap-w26i06/live348.js` | tool | logs/w26.i06.md |
| `/tmp/cap-w26i07/shots` | dir | logs/w26.i07.md |
| `/tmp/cap-w26i08` | dir | logs/w26.i08.md |
| `/tmp/cap-w26i08/rhythm.mjs` | tool | logs/w26.i08.md |
| `/tmp/cap-w26i09` | dir | logs/w26.i09.md |
| `/tmp/cap-w26i09/report3.json` | report | logs/w26.i09.md |
| `/tmp/cap-w26i10` | dir | logs/w26.i10.md |
| `/tmp/cap-w26i10/battery.mjs` | tool | logs/w26.i10.md |
| `/tmp/cap-w26i11/shots` | dir | logs/w26.i11.md |
| `/tmp/cap-w26i11/shots/measure-i11.json` | report | logs/w26.i11.md |
| `/tmp/cap-w26i11/{verify,crops,light}.mjs` | set | logs/w26.i11.md |
| `/tmp/cap-w26i16` | dir | logs/w26.i16.md |
| `/tmp/cap-w26i17/measurements{,2}.json` | set | logs/w26.i17.md |
| `/tmp/flow-w26e13.cjs` | tool | findings/w26.e13.md |
| `/tmp/shots` | dir | logs/w26.k03.md |
| `/tmp/shots-w26-k11` | dir | logs/w26.k11.md |
| `/tmp/shots-w26.e01` | dir | findings/w26.e01.md |
| `/tmp/shots-w26.e01/report.json` | report | findings/w26.e01.md |
| `/tmp/shots-w26.e01/step-03-appearance.png` | shot | findings/w26.e01.md |
| `/tmp/shots-w26.e01/step-07-shortcuts.png` | shot | findings/w26.e01.md |
| `/tmp/shots-w26.e01/win-appearance.png` | shot | findings/w26.e01.md |
| `/tmp/shots-w26.e02` | dir | findings/w26.e02.md |
| `/tmp/shots-w26.e03` | dir | findings/w26.e03.md |
| `/tmp/shots-w26.e03/vision-*.txt` | set | findings/w26.e03.md |
| `/tmp/shots-w26.e04` | dir | findings/w26.e04.md |
| `/tmp/shots-w26.e04/customize-anatomy.json` | report | findings/w26.e04.md |
| `/tmp/shots-w26.e04/miniplayer-geometry.json` | report | findings/w26.e04.md |
| `/tmp/shots-w26.e04/player-customize-reachability.json` | report | findings/w26.e04.md |
| `/tmp/shots-w26.e05` | dir | findings/w26.e05.md |
| `/tmp/shots-w26.e05/interface-light.png` | shot | findings/w26.e05.md |
| `/tmp/shots-w26.e05/probe-report.json` | report | findings/w26.e05.md |
| `/tmp/shots-w26.e06` | dir | findings/w26.e06.md |
| `/tmp/shots-w26.e06/state-*.png` | set | findings/w26.e06.md |
| `/tmp/shots-w26.e07` | dir | findings/w26.e07.md |
| `/tmp/shots-w26.e07/1024` | dir | findings/w26.e07.md |
| `/tmp/shots-w26.e08` | dir | findings/w26.e08.md |
| `/tmp/shots-w26.e09/playlist.png` | shot | findings/w26.e09.md |
| `/tmp/shots-w26.e09/{c1-sorting-menu-scaled,c2-view-menu-scaled,c3-thumbnail-card-scaled,c4-media-list-scaled,c5-sub-filename-*,a3-new-playlist-dialog,a1-playlist-seed,b11-folder-*}.png` | set | findings/w26.e09.md |
| `/tmp/shots-w26.e10` | dir | findings/w26.e10.md |
| `/tmp/shots-w26.e11/side-{colors,crop,essential,hdr}.png` | set | findings/w26.e11.md |
| `/tmp/shots-w26.e11/{03-colors-expanded,04-crop-expanded,05-essential-expanded,06e-hdr-open-composite}.png` | set | findings/w26.e11.md |
| `/tmp/shots-w26.e12` | dir | findings/w26.e12.md |
| `/tmp/shots-w26.e12/*.png` | set | findings/w26.e12.md |
| `/tmp/shots-w26.e13` | dir | findings/w26.e13.md |
| `/tmp/shots-w26.e14` | dir | findings/w26.e14.md |
| `/tmp/shots-w26.e14/*-pseudo.png` | set | findings/w26.e14.md |
| `/tmp/shots-w26.e18` | dir | findings/w26.e18.md |
| `/tmp/shots-w26.e18/1024x768/player.png` | shot | findings/w26.e18.md |
| `/tmp/shots-w26.e18/900x700/player.png` | shot | findings/w26.e18.md |
| `/tmp/shots-w26.e19/light` | dir | findings/w26.e19.md |
| `/tmp/shots-w26.e19/light-session` | dir | findings/w26.e19.md |
| `/tmp/shots-w26.e19/light-session/playlist-new-playlist-dialog-light.png` | shot | findings/w26.e19.md |
| `/tmp/shots-w26.e19/switch` | dir | findings/w26.e19.md |
| `/tmp/shots-w26.e19/switch/settings-interface-{dark,light}-live.png` | set | findings/w26.e19.md |
| `/tmp/shots-w26.f06r2` | dir | logs/w26.f06.md |
| `/tmp/shots-w26.f06r2/playlist.png` | shot | logs/w26.f06.md |
| `/tmp/shots-w26.f07` | dir | logs/w26.f07.md |
| `/tmp/shots-w26.i06` | dir | logs/w26.i06.md |
| `/tmp/shots-w26.i08/player.png` | shot | logs/w26.i08.md |
| `/tmp/shots-w26.i12` | dir | logs/w26.i12.md |
| `/tmp/shots-w26.i12*` | set | logs/w26.i12.md |
| `/tmp/shots-w26.i12-1024` | dir | logs/w26.i12.md |
| `/tmp/shots-w26.i12-light` | dir | logs/w26.i12.md |
| `/tmp/shots-w26.i15` | dir | logs/w26.i15.md |
| `/tmp/shots-w26.i15-light` | dir | logs/w26.i15.md |
| `/tmp/shots-w26.i15{,-light}` | set | logs/w26.i15.md |
| `/tmp/shots-w26.i16` | dir | logs/w26.i16.md |
| `/tmp/shots-w26.k03` | dir | logs/w26.k03.md |
| `/tmp/shots-w26.k06/player.png` | shot | logs/w26.k06.md |
| `/tmp/shots-w26.k08` | dir | logs/w26.k08.md |
| `/tmp/shots-w26.k09/bookmarks-{1440,1024}.png` | set | logs/w26.k09.md |
| `/tmp/shots-w26.k17` | dir | logs/w26.k17.md |
| `/tmp/shots-w26a02/player.png` | shot | logs/w26.a02.md |
| `/tmp/shots-w26a04` | dir | logs/w26.a04.md |
| `/tmp/shots-w26a05` | dir | logs/w26.a05.md |
| `/tmp/shots-w26a06` | dir | logs/w26.a06.md |
| `/tmp/shots-w26a07` | dir | logs/w26.a07.md |
| `/tmp/shots-w26a07/player.png` | shot | logs/w26.a07.md |
| `/tmp/shots-w26a09` | dir | logs/w26.a09.md |
| `/tmp/shots-w26a11` | dir | logs/w26.a11.md |
| `/tmp/shots-w26a11-1024` | dir | logs/w26.a11.md |
| `/tmp/shots-w26a11-bypass` | dir | logs/w26.a11.md |
| `/tmp/shots-w26a13/player.png` | shot | logs/w26.a13.md |
| `/tmp/shots-w26a15` | dir | logs/w26.a15.md |
| `/tmp/shots-w26b01` | dir | logs/w26.b01.md |
| `/tmp/shots-w26b01/settings.png` | shot | logs/w26.b01.md |
| `/tmp/shots-w26b13{,-1024}` | set | logs/w26.b13.md |
| `/tmp/shots-w26c1` | dir | logs/w26.c1.md |
| `/tmp/shots-w26c1/*.png` | set | logs/w26.c1.md |
| `/tmp/shots-w26c3` | dir | logs/w26.c3.md |
| `/tmp/shots-w26c3/*.png` | set | logs/w26.c3.md |
| `/tmp/shots-w26c5` | dir | logs/w26.c5.md |
| `/tmp/shots-w26c5/*.png` | set | logs/w26.c5.md |
| `/tmp/shots-w26d1` | dir | logs/w26.d1.md |
| `/tmp/shots-w26d2-1024` | dir | logs/w26.d2.md |
| `/tmp/shots-w26d2-light` | dir | logs/w26.d2.md |
| `/tmp/shots-w26g03` | dir | findings/w26.g03.md |
| `/tmp/shots-w26g03/{player,player-1024,player-light}.png` | set | findings/w26.g03.md |
| `/tmp/shots-w26g07/customize-after-ctxmenu-click.png` | shot | findings/w26.g07.md |
| `/tmp/shots-w26g10` | dir | findings/w26.g10.md |
| `/tmp/shots-w26g10/{global,interface}.png` | set | findings/w26.g10.md |
| `/tmp/shots-w26g11/{privacy,playback}.png` | set | findings/w26.g11.md |
| `/tmp/shots-w26g12` | dir | findings/w26.g12.md |
| `/tmp/shots-w26g13/subtitles.png` | shot | findings/w26.g13.md |
| `/tmp/shots-w26g18` | dir | findings/w26.g18.md |
| `/tmp/shots-w26g20` | dir | findings/w26.g20.md |
| `/tmp/shots-w26gfixB-final` | dir | logs/w26.gfix-B.md |
| `/tmp/shots-w26gfixB2/*-light.png` | set | logs/w26.gfix-B.md |
| `/tmp/shots-w26h03` | dir | ratings/w26.h03.md |
| `/tmp/shots-w26h03/frame-composition-1440.png` | shot | ratings/w26.h03.md |
| `/tmp/shots-w26h04` | dir | ratings/w26.h04.md |
| `/tmp/shots-w26h07/probes{,2}.json` | set | ratings/w26.h07.md |
| `/tmp/shots-w26h08` | dir | ratings/w26.h08.md |
| `/tmp/shots-w26h09` | dir | ratings/w26.h09.md |
| `/tmp/shots-w26h09-1024` | dir | ratings/w26.h09.md |
| `/tmp/shots-w26h09-light` | dir | ratings/w26.h09.md |
| `/tmp/shots-w26h11` | dir | ratings/w26.h11.md |
| `/tmp/shots-w26h13/report.json` | report | ratings/w26.h13.md |
| `/tmp/shots-w26h18` | dir | ratings/w26.h18.md |
| `/tmp/shots-w26h18/measurements.json` | report | ratings/w26.h18.md |
| `/tmp/shots-w26h19*` | set | ratings/w26.h19.md |
| `/tmp/shots-w26h20` | dir | ratings/w26.h20.md |
| `/tmp/shots-w26h20/v{p,m,min,ob}.json` | set | ratings/w26.h20.md |
| `/tmp/shots-w26i01` | dir | logs/w26.i01.md |
| `/tmp/shots-w26i02` | dir | logs/w26.i02.md |
| `/tmp/shots-w26i03` | dir | logs/w26.i03.md |
| `/tmp/shots-w26i05/state{0..8}.png` | set | logs/w26.i05.md |
| `/tmp/shots-w26i10` | dir | logs/w26.i10.md |
| `/tmp/shots-w26i11` | dir | logs/w26.i11.md |
| `/tmp/shots-w26i13` | dir | logs/w26.i13.md |
| `/tmp/shots-w26i14r3*` | set | logs/w26.i14.md |
| `/tmp/shots-w26i17` | dir | logs/w26.i17.md |
| `/tmp/shots-w26i18` | dir | logs/w26.i18.md |
| `/tmp/shots-w26i20` | dir | logs/w26.i20.md |
| `/tmp/verify-w26e06.mjs` | tool | findings/w26.e06.md |
| `/tmp/verify-w26e11*.mjs` | set | findings/w26.e11.md |
| `/tmp/vision-w26b01.txt` | log | logs/w26.b01.md |
| `/tmp/w26c1-harness/*.html` | set | logs/w26.c1.md |
| `/tmp/w26c3-harness/*.html` | set | logs/w26.c3.md |
| `/tmp/w26c5-harness/*.html` | set | logs/w26.c5.md |
| `/tmp/w26e01-keyboard*.mjs` | set | findings/w26.e01.md |
| `/tmp/w26e01-measure.mjs` | tool | findings/w26.e01.md |
| `/tmp/w26e01-onboarding.mjs` | tool | findings/w26.e01.md |
| `/tmp/w26e02-empty.mjs` | tool | findings/w26.e02.md |
| `/tmp/w26e02-explore.mjs` | tool | findings/w26.e02.md |
| `/tmp/w26e02-explore2.mjs` | tool | findings/w26.e02.md |
| `/tmp/w26e02-hover2.mjs` | tool | findings/w26.e02.md |
| `/tmp/w26e02-menuclip.mjs` | tool | findings/w26.e02.md |
| `/tmp/w26e07-eqcard.mjs` | tool | findings/w26.e07.md |
| `/tmp/w26e07-states.mjs` | tool | findings/w26.e07.md |
| `/tmp/w26e09-refs/AddSubtitles_Folder{,-1,-2,-3}.png` | set | findings/w26.e09.md |
| `/tmp/w26e09-work/*.mjs` | set | findings/w26.e09.md |
| `/tmp/w26e12-1024.json` | report | findings/w26.e12.md |
| `/tmp/w26e12-menus.mjs` | tool | findings/w26.e12.md |
| `/tmp/w26e12-metrics3.json` | report | findings/w26.e12.md |
| `/tmp/w26e12-metrics{,2,3,4}.json` | set | findings/w26.e12.md |
| `/tmp/w26e12-pixels.mjs` | tool | findings/w26.e12.md |
| `/tmp/w26e12-shortcuts.json` | report | findings/w26.e12.md |
| `/tmp/w26e14-aria.mjs` | tool | findings/w26.e14.md |
| `/tmp/w26e14-dead-keys.txt` | log | findings/w26.e14.md |
| `/tmp/w26e14-key-coverage.mjs` | tool | findings/w26.e14.md |
| `/tmp/w26e14-orphans2.mjs` | tool | findings/w26.e14.md |
| `/tmp/w26e14-pseudo-sweep.mjs` | tool | findings/w26.e14.md |
| `/tmp/w26e17-audit.cjs` | tool | findings/w26.e17.md |
| `/tmp/w26e17-audit.json` | report | findings/w26.e17.md |
| `/tmp/w26e17-contrast.cjs` | tool | findings/w26.e17.md |
| `/tmp/w26e17-followup.json` | report | findings/w26.e17.md |
| `/tmp/w26e17-light.json` | report | findings/w26.e17.md |
| `/tmp/w26e18-audit.json` | report | findings/w26.e18.md |
| `/tmp/w26e18-audit.mjs` | tool | findings/w26.e18.md |
| `/tmp/w26e18-followup*.mjs` | set | findings/w26.e18.md |
| `/tmp/w26e18-vision/crop-900-controls.png` | shot | findings/w26.e18.md |
| `/tmp/w26e19-*.mjs` | set | findings/w26.e19.md |
| `/tmp/w26e19-audit.json` | report | findings/w26.e19.md |
| `/tmp/w26e19-audit.mjs` | tool | findings/w26.e19.md |
| `/tmp/w26e19-contrast*.mjs` | set | findings/w26.e19.md |
| `/tmp/w26e19-light.json` | report | findings/w26.e19.md |
| `/tmp/w26e19-light.mjs` | tool | findings/w26.e19.md |
| `/tmp/w26e19-live.json` | report | findings/w26.e19.md |
| `/tmp/w26e19-live.mjs` | tool | findings/w26.e19.md |
| `/tmp/w26e19-live2.json` | report | findings/w26.e19.md |
| `/tmp/w26e19-live2.mjs` | tool | findings/w26.e19.md |
| `/tmp/w26e19-onboard3/4.mjs` | tool | findings/w26.e19.md |
| `/tmp/w26e19-scrim.mjs` | tool | findings/w26.e19.md |
| `/tmp/w26e19-switchon.mjs` | tool | findings/w26.e19.md |
| `/tmp/w26f06r2-work/{drive,elements,extras,f6harness}.mjs` | set | logs/w26.f06.md |
| `/tmp/w26h01-clickthrough.mjs` | tool | ratings/w26.h01.md |
| `/tmp/w26i19/contrast.py` | tool | logs/w26.i19.md |
| `/tmp/w26i19/e2e-theme.mjs` | tool | logs/w26.i19.md |
