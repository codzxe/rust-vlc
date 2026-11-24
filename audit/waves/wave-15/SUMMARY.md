# WAVE 15 SUMMARY — ui/src

| Field | Value |
|---|---|
| Agents | A01, A09, A10 complete; A02-A08 timed out at ~90% -> recovery R1-R3; 100% coverage |
| Files audited | 434, 100% covered |
| Findings | S0:0 S1:0 S2:7+ S3:~40 S4:~60 |
| Tree purity | clean |

## THE UI ANSWER (master question)
REAL wiring EXISTS, through the single lumen_dispatch IPC: transport
controls, settings writes (config.set/get/reset-setting, keys verified
engine-side), playlist play-item, frame-channel bind, capability probes,
OS window controls. Hand-rolled bridge (no @tauri-apps/api). BUT: zero
executed end-to-end evidence anywhere (tauri app unbuildable; browser
lane runs DemoTransport). The browser build is an honestly-badged demo;
the desktop build is the real target — and it cannot compile.

## Key findings
1. [CONFIRMED, both directions] The canvas EXISTS: MediaSurface.tsx:174/
   :227 real putImageData behind __TAURI_INTERNALS__; KNOWN-LIMITS.md:52
   "no <canvas> exists in ui/src" is FALSE/stale; the browser build is
   genuinely canvas-less (pinned by PlayerScreen.test:68). The master
   prompt's pin was stale in one direction, right in the other.
2. [S2] Browser lane renders fake data as real: DemoTransport seeds fake
   volumes ("Lumen demo media"/"Field Kit") rendered as real rows on
   /discovery; settings fall back to localStorage (W15-A01).
3. [S2] playlistsStore SEEDs 3 fake playlists ("Cool Songs", fake
   episodes) presented as user data; MiniPlayer "+" lists unlabeled demo
   clips (W15-A09).
4. [S2] config-settings.ts: failed settings writes silently lost
   (optimistic snapshot never reconciled; onWriteResult wired nowhere) —
   "never swallowed" contract false (W15-A01).
5. [S2] EQ UI: 10-band equalizer UI vs engine's 3-band Eq3Band;
   audio.set-eq command absent; faders interactive-but-local-only at
   forced opacity:1 (W15-A01/R1/A09).
6. [Dead-surface census] subtitles settings 14/14 staged; effects+
   library 100% staged; file menu 7/12 rows disabled; ~8 of 141 hotkey
   bindings execute; visualizer cosmetic (no FFT source); 3 toolbar dead
   tiles (record/pip/tone-mapping absent from wire union, errors
   swallowed, UNDISCLOSED); ColorsTool 13/13 inert (staged-labeled);
   convert lifecycle unreachable; 5/6 menu rows disabled on discovery.
7. [S3] hardcoded requestId 1/2 violates doc 05's monotonic anti-replay
   contract (W15-A10); radiogroup tabindex holes; barrel bloat (5/18,
   14/18, 9/17 exports dead); dead form-Checkbox island with 5 green
   tests + snapshot.
8. [DATA LITTER] ui/.vitest/json/output.json: committed ad-hoc test run
   from a FOREIGN checkout (/home/z/my-project/rust-vlc path leak),
   success:false, zero consumers (W15-R3).
9. [Verified good] generated settings screens: mutation-honest drift
   tests, 8 invariants, i18n fails-closed; i18n zero-overlap claim TRUE
   (15 non-UI keys in toml vs hand-written TS catalog; two disjoint
   systems); disabled-with-reason grammar pervasive and truthful.
