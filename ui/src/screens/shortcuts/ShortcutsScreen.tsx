// /keyboard-shortcuts (Solstice app-shell workstream, 2026-11-08) — the
// grouped keycap REFERENCE, built from REAL keymap data:
//   * `HOTKEY_BINDINGS` (settings/areas/hotkeys/generated/bindings.ts — the
//     generated mirror of keymaps/default.toml, the same source the
//     HotkeysEditor edits), and
//   * the shell-owned variant/fullscreen seed rows (player keymap +
//     variantKeymap [PROPOSAL] seeds), whose labels reuse existing keys.
// Every row shows the action label (binding.labelMsgid) plus .solstice-key
// keycap chips split from the binding's combo spelling. Editing happens in
// Settings → Hotkeys (footer link); this screen is read-only by design.
//
// Solstice v2 craft pass (2026-11-09, brief §2.9/§2.10):
//   * Keycaps render ≥13px with FRIENDLY labels — raw keymap spellings like
//     "BrowserRefresh"/"VolumeMute"/"MediaNext" map to Refresh/Mute/Next and
//     arrows render as glyphs (the artwork-not-copy convention shared with
//     the onboarding tour).
//   * Numbered series (bookmarks) sort NUMERICALLY: 1, 2, 3 … 10, never the
//     lexical 1, 10, 2 (runs of equal-prefix labels compare trailing
//     numbers; everything else keeps the curated keymap order).
//   * The reference is an APP-tier screen: calm backdrop, anchored column
//     (min(1200px, 92vw)), structured cards, one left edge for the search
//     box and the cards, visible warm scrollbar when the list runs tall.
//
// Grouping policy (keymaps sections → the four pre-seeded shortcuts.group.*
// groups): media/playlist/subtitles → playback; navigation/advanced →
// navigation; audio → volume; video/window + the shell variant seeds and
// the platform-variant (*-macos) rows → variants.
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { TextField } from "../../components/form/TextField";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { DEFAULT_KEYMAP } from "../../screens/player/keymap";
import type { KeyBinding } from "../../screens/player/keymap";
import { VARIANT_KEYMAP } from "../../screens/player/variants";
import { HOTKEY_BINDINGS } from "../../screens/settings/areas/hotkeys/generated/bindings";
import type { HotkeyBinding } from "../../screens/settings/areas/hotkeys/generated/bindings";
import "./shortcuts.css";

/** One reference row: an action label + its combo chips. */
export interface ShortcutRow {
  readonly id: string;
  readonly labelKey: MessageKey;
  readonly key: string;
}

/** The four reference groups (the pre-seeded shortcuts.group.* keys). */
export type ShortcutGroupId = "playback" | "navigation" | "volume" | "variants";

/** Group definition: i18n heading + the rows belonging to it. */
export interface ShortcutGroup {
  readonly id: ShortcutGroupId;
  readonly labelKey: MessageKey;
  readonly rows: readonly ShortcutRow[];
}

/** keymaps section → group id (see the module header for the policy). */
const SECTION_TO_GROUP: Readonly<Record<string, ShortcutGroupId>> = {
  media: "playback",
  playlist: "playback",
  subtitles: "playback",
  navigation: "navigation",
  advanced: "navigation",
  audio: "volume",
  video: "variants",
  window: "variants",
};

/** Platform-variant rows (section "<base>-macos") map by their base action. */
function macosSectionGroup(section: string): ShortcutGroupId | undefined {
  const base = section.replace(/-macos$/u, "");
  if (base.startsWith("audio")) return "volume";
  if (base.startsWith("subtitle") || base.startsWith("seek")) return "playback";
  if (base.startsWith("fullscreen")) return "variants";
  return undefined;
}

const groupOfHotkey = (binding: HotkeyBinding): ShortcutGroupId | undefined =>
  binding.section.endsWith("-macos")
    ? macosSectionGroup(binding.section)
    : SECTION_TO_GROUP[binding.section];

/** Labels for the shell-owned seed rows (variant toggles + fullscreen). */
const SHELL_BINDING_LABELS: Readonly<Record<string, MessageKey>> = {
  fullscreen: "hotkeys.fullscreen",
  "exit-fullscreen": "hotkeys.exit-fullscreen",
  "show-controls-fullscreen": "hotkeys.show-controls-fullscreen",
  "miniplayer-toggle": "menu.view.miniplayer",
  "music-mode-toggle": "menu.view.music-mode",
};

function shellRow(binding: KeyBinding): ShortcutRow | null {
  const labelKey = SHELL_BINDING_LABELS[binding.id];
  return labelKey === undefined ? null : { id: binding.id, labelKey, key: binding.key };
}

const GROUP_LABELS: Readonly<Record<ShortcutGroupId, MessageKey>> = {
  playback: "shortcuts.group.playback",
  navigation: "shortcuts.group.navigation",
  volume: "shortcuts.group.volume",
  variants: "shortcuts.group.variants",
};

const GROUP_ORDER: readonly ShortcutGroupId[] = ["playback", "navigation", "volume", "variants"];

/**
 * Keymap spellings → friendly keycap text (brief §2.9: no raw
 * "BrowserRefresh" — media/browser keys get short human labels, arrows and
 * wheel axes render as glyphs). Artwork, not copy: same convention as the
 * onboarding tour's arrow caps. Unknown spellings render verbatim keymap
 * data (never invented).
 */
const KEYCAP_LABELS: Readonly<Record<string, string>> = {
  // arrows + wheel
  Down: "↓",
  Left: "←",
  Right: "→",
  Up: "↑",
  WheelUp: "Wheel ↑",
  WheelDown: "Wheel ↓",
  // browser media keys — spell with spaces so they read as KEY NAMES, not
  // actions ("Play — Browser Refresh" is VLC 3.0.23's real default, and a
  // truncated "Refresh" cap next to "Play" reads as a mislabel)
  BrowserBack: "Browser Back",
  BrowserForward: "Browser Fwd",
  BrowserNext: "Browser Next",
  BrowserPrevious: "Browser Prev",
  BrowserRefresh: "Browser Refresh",
  BrowserStop: "Browser Stop",
  BrowserSearch: "Browser Search",
  BrowserHome: "Browser Home",
  // media keys
  MediaNext: "Media Next",
  MediaPrevious: "Media Prev",
  MediaPrev: "Media Prev",
  MediaPlay: "Media Play",
  MediaPause: "Media Pause",
  MediaPlayPause: "Media Play/Pause",
  MediaStop: "Media Stop",
  MediaRecord: "Media Rec",
  // volume keys
  VolumeMute: "Mute",
  VolumeUp: "Vol +",
  VolumeDown: "Vol −",
  VolumeRaise: "Vol +",
  VolumeLower: "Vol −",
  // long function-key spellings
  PageUp: "PgUp",
  PageDown: "PgDn",
  PrintScreen: "PrtSc",
  ScrollLock: "ScrLk",
  NumLock: "NumLk",
  CapsLock: "Caps",
  Backspace: "Bksp",
  Insert: "Ins",
  Delete: "Del",
  // modifier abbreviations (v2 §2.9: Ctrl/⌥/⇧…)
  Shift: "⇧",
  Alt: "⌥",
  Meta: "⌘",
  Super: "⌘",
};

/** One keycap chip's display text (friendly map ▸ verbatim fallback). */
export function keycapText(spelling: string): string {
  return KEYCAP_LABELS[spelling] ?? spelling;
}

/** Build the full grouped reference from the real data (pure; test-pinnable). */
export function buildShortcutGroups(
  bindings: readonly HotkeyBinding[] = HOTKEY_BINDINGS,
  shellBindings: readonly KeyBinding[] = [...DEFAULT_KEYMAP.values(), ...VARIANT_KEYMAP.values()],
): readonly ShortcutGroup[] {
  const rows: Record<ShortcutGroup["id"], ShortcutRow[]> = {
    playback: [],
    navigation: [],
    volume: [],
    variants: [],
  };
  for (const binding of bindings) {
    if (binding.disabled) continue; // feature-gated rows are not reference rows
    const group = groupOfHotkey(binding);
    if (group === undefined) continue;
    rows[group].push({
      id: binding.id,
      labelKey: binding.labelMsgid as MessageKey,
      key: binding.key,
    });
  }
  for (const binding of shellBindings) {
    const row = shellRow(binding);
    if (row !== null) rows.variants.push(row);
  }
  return GROUP_ORDER.map((id) => ({ id, labelKey: GROUP_LABELS[id], rows: rows[id] }));
}

/** Label with its trailing number stripped ("Play Bookmark 10" → "Play Bookmark"). */
const labelPrefix = (label: string): string => label.replace(/[0-9]+\s*$/u, "").trim();

/** Trailing number of a label, or -1 when it ends in no number. */
const trailingNumber = (label: string): number => {
  const match = /[0-9]+\s*$/u.exec(label);
  return match === null ? -1 : Number(match[0]);
};

/**
 * Numeric-aware ordering for NUMBERED SERIES (brief §2.10: bookmarks 1, 2,
 * 3 … 10, never the lexical 1, 10, 2). Only runs of consecutive rows whose
 * labels share one prefix and end in numbers are reordered — the curated
 * keymap order of everything else is untouched.
 */
export function sortRowsNumericAware(
  rows: readonly ShortcutRow[],
  labelOf: (key: MessageKey) => string,
): readonly ShortcutRow[] {
  const labels = rows.map((row) => labelOf(row.labelKey));
  const result = [...rows];
  let start = 0;
  while (start < result.length) {
    const prefix = labelPrefix(labels[start]!);
    let end = start + 1;
    while (end < labels.length && labelPrefix(labels[end]!) === prefix) end++;
    if (end - start > 1) {
      const run = result
        .slice(start, end)
        .map((row, offset) => ({ row, number: trailingNumber(labels[start + offset]!) }))
        .sort((a, b) => a.number - b.number);
      for (let offset = 0; offset < run.length; offset++) result[start + offset] = run[offset]!.row;
    }
    start = end;
  }
  return result;
}

/** Case-insensitive filter over the localized label, the combo spelling and
 * the FRIENDLY cap text (so searching "mute"/"refresh" finds the row). */
export function filterGroups(
  groups: readonly ShortcutGroup[],
  labelOf: (key: MessageKey) => string,
  query: string,
): readonly ShortcutGroup[] {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) return groups;
  return groups
    .map((group) => ({
      ...group,
      rows: group.rows.filter(
        (row) =>
          labelOf(row.labelKey).toLowerCase().includes(needle) ||
          row.key.toLowerCase().includes(needle) ||
          keyChips(row.key).some((chip) => chip.toLowerCase().includes(needle)),
      ),
    }))
    .filter((group) => group.rows.length > 0);
}

/** Split a combo spelling ("Ctrl+Shift+Right") into friendly keycap texts. */
export function keyChips(combo: string): readonly string[] {
  return combo
    .split("+")
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
    .map((part) => keycapText(part));
}

export function ShortcutsScreen() {
  const m = useMsg();
  const [query, setQuery] = useState("");

  // Solstice v2 (brief §1): APP-tier reference screen — calm backdrop while
  // mounted (cleaned up on unmount), anchored column, no stage theatrics.
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "calm";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

  const groups = useMemo(() => buildShortcutGroups(), []);
  // Bookmarks and other numbered series render in NUMERIC order (1..10).
  const ordered = useMemo(
    () => groups.map((group) => ({ ...group, rows: sortRowsNumericAware(group.rows, m) })),
    [groups, m],
  );
  const visible = useMemo(() => filterGroups(ordered, m, query), [ordered, m, query]);

  return (
    <div className="shortcuts">
      <header className="shortcuts__header">
        <h1 className="shortcuts__title">{m("shortcuts.title")}</h1>
        <p className="shortcuts__subtitle">{m("shortcuts.subtitle")}</p>
        <TextField
          className="shortcuts__search"
          label={m("shortcuts.search.label")}
          placeholder={m("shortcuts.search.placeholder")}
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </header>

      {visible.length === 0 ? (
        <p className="shortcuts__empty">{m("shortcuts.search.empty")}</p>
      ) : (
        <div className="shortcuts__groups">
          {visible.map((group) => (
            <section
              key={group.id}
              className="shortcuts__group"
              aria-labelledby={`shortcuts-group-${group.id}`}
            >
              <h2 id={`shortcuts-group-${group.id}`} className="shortcuts__group-title">
                {m(group.labelKey)}
              </h2>
              <ul className="shortcuts__rows">
                {group.rows.map((row) => (
                  <li key={row.id} className="shortcuts__row" data-binding={row.id}>
                    <span className="shortcuts__action">{m(row.labelKey)}</span>
                    <span className="shortcuts__keys">
                      {keyChips(row.key).map((chip, index) => (
                        <kbd key={`${row.id}-${index}`} className="solstice-key solstice-key--sm shortcuts__keycap">
                          {chip}
                        </kbd>
                      ))}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      )}

      <footer className="shortcuts__footer">
        <p className="shortcuts__hint">{m("shortcuts.edit.hint")}</p>
        <Link className="shortcuts__edit-link" to="/settings/hotkeys">
          {m("settings.area.hotkeys")}
        </Link>
      </footer>
    </div>
  );
}
