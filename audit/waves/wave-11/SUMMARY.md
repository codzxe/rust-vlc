# WAVE 11 SUMMARY — config, settings, locales, hotkeys, themes

| Field | Value |
|---|---|
| Agents | A01, A03–A10 complete; A02 max-turns timeout -> completion run |
| Files audited | 131, 100% covered |
| Findings | S0:2 S1:0 S2:10 S3:~50 S4:~150 |
| Tree purity | clean |

## THE SETTINGS ANSWER
- Round-trip is REAL in-process only: `setting set` validates via embedded
  schema and stores into a session-local BTreeMap that only `setting get`
  reads. NO persistence exists anywhere — doc 07 §4.1 "persisted settings"
  FALSE; UI persistence is a localStorage MOCK nothing reads back
  (W11-A05/A07).
- The ENTIRE localization plane is unwired: crates/localization (real,
  tested ICU-subset) has zero dependents; en-US.toml ships 16 keys, the UI
  uses its own hand-written TS catalog (1000+ keys, ZERO overlap, own
  l10n-check gate); locale picker hardcodes ["en-US"]; doc 07's
  l10n-audit gate does not exist; 121 settings msgids ALL unresolved
  (W11-A08/A09/A04).
- Hotkeys: 150 bindings in keymaps/default.toml, only 7 wired (hardcoded
  excerpt seeds); 143/150 wired to nothing; all 8 macOS overrides dead;
  18/36 command rows target commands the CLI refuses; frame-step: 3
  bindings -> playback.step-frame refused by BOTH hosts (W11-A07,
  confirms Wave 3).
- Theme tokens: real pipeline, WCAG ratios recompute exactly, but
  emit_rust's 247-line output has zero consumers; focus ring pinned
  orange under 8/9 accents vs doc "focus = accent.primary" (W11-A06).
- product-config semantic layer: zero production callers; the accent
  check Wave 1 found missing is CONFIRMED missing AND inexpressible;
  the tree violates today (#4f8cff vs #d37918) (W11-A10).

## Worst findings
1. [S0, EXECUTED] settings-help.txt and settings-docs.md advertise 121
   `--<area>-<key>` CLI flags; the parser knows 4 flags; every settings
   flag exits 64 "unknown command" (W11-A04 executed --video-hue 50;
   W11-A10 executed --audio-volume-step; W11-A09 confirmed + phantom
   --generate-manifest).
2. [S2] schema.rs: unresolvable/single-level $ref silently DISABLES
   validation — "drift fails loudly" only for `pattern` (W11-A09).
3. [S2] emit.rs prints flags the parser rejects (same S0 family).
4. [S2] keymap loading pipeline "not landed" — 143/150 dead bindings
   (W11-A07).
5. [S2] settings.toml catalog: keys declared vs consumed — help text
   consumed by --help only; settings-help/doc drift-free but describe a
   CLI surface that does not exist (W11-A02 SHALLOW + A04/A10 S0s).
6. [S3 cluster] merge.rs blame mis-attribution; T16 tests absent (doc
   claims named tests T1-T16, 9 exist); plural rules pre-CLDR-42 (fr/pt
   zero wrong); 39/40 product-config fixtures stale forks.
