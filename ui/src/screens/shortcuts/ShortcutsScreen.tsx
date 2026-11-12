// /keyboard-shortcuts — the W26 Figma keyboard-shortcuts window
// (design/figma/Keyboard Shortcuts.svg): a floating 1048×800 r14 #191919@95%
// dialog over the app (same window grammar as the Settings frames), built
// from REAL keymap data:
//   * `HOTKEY_BINDINGS` (settings/areas/hotkeys/generated/bindings.ts — the
//     generated mirror of keymaps/default.toml, the same source the
//     HotkeysEditor edits), and
//   * the shell-owned variant/fullscreen seed rows (player keymap +
//     variantKeymap [PROPOSAL] seeds), whose labels reuse existing keys.
//
// w26.i17 (h19 shame #2 — dialog semantics): the window wore dialog chrome
// but was a bare route page — no role="dialog", no focus trap, and Tab
// walked all 146 pencil links before reaching Cancel. The window now rides
// the shared <Dialog> primitive (same as /open-network-stream): scrim +
// role=dialog + aria-modal + the surface focus trap (Tab wraps to the
// window's first focusable — the semantics ui/e2e/a11y.spec.ts encodes) +
// Escape (yielding to an open popup: the trap defers while a listbox/menu
// is open inside) + scrim-click close. Initial focus lands on the search
// field (the first field), not the trap's default ✕ glyph — same h19 #4
// fix as the netstream dialog.
//
// Frame anatomy (measured from the SVG, x/y in the 1048×800 artboard):
//   * top bar — 96px #090909 strip: "Keyboard Shortcuts" title #8F8F8F ≈14px
//     at (20,20), ✕ 10×10 at (1018,20); below it the search row y53: search
//     pill 820×34 r6 #151515 (magnifier 15×15, placeholder #404040) + field
//     filter 200×34 r6 #151515 ("All Fields" #969696, chevron #8F8F8F);
//   * content — column headers "Action" #6B6B6B left / "Hotkey" / "Global
//     Hotkey" centered over their columns, hairline rule, then sections
//     (16px #969696 headers) of rows: action label #C6C6C6 left, then TWO
//     270×33 r6 #090909 keycaps (hotkey + global hotkey) with the combo
//     centered #C6C6C6 and an edit pencil #404040 at the right inset;
//   * footer — Cancel 100×32 r6 (Button secondary grammar) left, "Save"
//     100×32 r6 #371F06 + #D37918 at 50% (disabled — this reference screen
//     is read-only by design) right.
//
// Data honesty: the keymaps model has no global-hotkey bindings (VLC 3.0.23
// ships every `global-key-*` empty — the frame's global column is empty for
// the same reason), so the Global Hotkey keycaps render empty with their
// pencil affordance, exactly like the frame. The pencil on the Hotkey column
// links to Settings → Hotkeys (the editing surface); editing does NOT happen
// here. Search keeps the exact pre-rebuild contract (label, raw combo and
// friendly cap text) and gains the frame's field-scope dropdown
// (All Fields / Action / Hotkey; "All Fields" = the previous behavior).
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { Icon } from "../../components/icon/Icon";
import { Dropdown } from "../../components/menu/Dropdown";
import type { DropdownOption } from "../../components/menu/Dropdown";
import { Dialog } from "../../components/surface";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b16";
import { DEFAULT_KEYMAP } from "../../screens/player/keymap";
import type { KeyBinding } from "../../screens/player/keymap";
import { VARIANT_KEYMAP } from "../../screens/player/variants";
import { HOTKEY_BINDINGS } from "../../screens/settings/areas/hotkeys/generated/bindings";
import type { HotkeyBinding } from "../../screens/settings/areas/hotkeys/generated/bindings";
import "./shortcuts.css";

// ---------------------------------------------------------------------------
// Lane i18n seam (contract §4.5) — copy with no catalog entry yet
// ---------------------------------------------------------------------------

/** Keys owned by this lane (registered into messages.ts by the Lead). */
type LaneKey = keyof typeof laneKeys;
const laneCatalog: Readonly<Record<string, string>> = laneKeys;

/**
 * useShortcutsMsg — `useMsg` extended with this lane's pre-registration keys.
 * Lane keys win while they are unregistered; once the Lead merges them into
 * the catalog both paths return the same string. Every user-visible string
 * flows through here — no literals (doc 07 §6.5).
 */
function useShortcutsMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}

// ---------------------------------------------------------------------------
// Reference data (unchanged grouping policy + friendly keycap map)
// ---------------------------------------------------------------------------

/** One reference row: an action label + its combo. */
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
 * Keymap spellings → friendly keycap text (no raw "BrowserRefresh" — media/
 * browser keys get short human labels, arrows and wheel axes render as
 * glyphs). Artwork, not copy: same convention as the onboarding tour's
 * arrow caps. Unknown spellings render verbatim keymap data (never
 * invented).
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
  // modifier abbreviations
  Shift: "⇧",
  Alt: "⌥",
  Meta: "⌘",
  Super: "⌘",
};

/** One keycap chip's display text (friendly map ▸ verbatim fallback). */
export function keycapText(spelling: string): string {
  return KEYCAP_LABELS[spelling] ?? spelling;
}

/** Split a combo spelling ("Ctrl+Shift+Right") into friendly keycap texts. */
export function keyChips(combo: string): readonly string[] {
  return combo
    .split("+")
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
    .map((part) => keycapText(part));
}

/**
 * The keycap's display text (Keyboard Shortcuts.svg Frame 162: ONE 270×33
 * keycap per binding with the whole combo centered — "Space", "esc" — not a
 * chip run): friendly chips joined back into a combo spelling.
 */
export function keycapCombo(combo: string): string {
  return keyChips(combo).join("+");
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
 * Numeric-aware ordering for NUMBERED SERIES (bookmarks 1, 2, 3 … 10, never
 * the lexical 1, 10, 2). Only runs of consecutive rows whose labels share
 * one prefix and end in numbers are reordered — the curated keymap order of
 * everything else is untouched.
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

/** Where the search looks (the frame's "All Fields" filter dropdown). */
export type ShortcutFilterScope = "all" | "action" | "hotkey";

/** Case-insensitive match of one row under a scope (label / key / caps). */
function rowMatches(
  row: ShortcutRow,
  labelOf: (key: MessageKey) => string,
  needle: string,
  scope: ShortcutFilterScope,
): boolean {
  const action = labelOf(row.labelKey).toLowerCase().includes(needle);
  if (scope === "action") return action;
  const hotkey =
    row.key.toLowerCase().includes(needle) ||
    keyChips(row.key).some((chip) => chip.toLowerCase().includes(needle));
  return scope === "hotkey" ? hotkey : action || hotkey;
}

/**
 * Filter the groups by query (case-insensitive). Default scope "all" keeps
 * the exact pre-rebuild contract: localized label, raw combo spelling AND
 * the friendly cap text (so searching "mute"/"refresh" finds the row).
 */
export function filterGroups(
  groups: readonly ShortcutGroup[],
  labelOf: (key: MessageKey) => string,
  query: string,
  scope: ShortcutFilterScope = "all",
): readonly ShortcutGroup[] {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) return groups;
  return groups
    .map((group) => ({
      ...group,
      rows: group.rows.filter((row) => rowMatches(row, labelOf, needle, scope)),
    }))
    .filter((group) => group.rows.length > 0);
}

/** Search-scope options (Keyboard Shortcuts.svg Frame 124 "All Fields"). */
const FILTER_OPTIONS: DropdownOption<MessageKey | LaneKey>[] = [
  { value: "all", labelKey: "shortcuts.filter.all" },
  { value: "action", labelKey: "settings.hotkeys.column.action" },
  { value: "hotkey", labelKey: "shortcuts.column.hotkey" },
];

export function ShortcutsScreen() {
  const m = useMsg();
  const t = useShortcutsMsg();
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<ShortcutFilterScope>("all");
  const titleId = useId();
  const searchRef = useRef<HTMLInputElement | null>(null);

  // Every dismissal (Escape / scrim / ✕ / Cancel) is the same exit the
  // pre-rebuild window shipped: back to the player home. Escape itself is
  // owned by the Dialog's focus trap (yields while a popup is open — the
  // filter dropdown's listbox handles its own Escape); the w26.d1
  // window-level keydown listener is gone (it would have deferred to its
  // own role="dialog" forever).
  const exit = (): void => {
    navigate("/player");
  };

  // w26.i17 (h19 #4): initial focus belongs on the FIRST FIELD (the search
  // input — the frame's rest state), not the trap's first-focusable ✕ glyph
  // with its painted ring. Runs after the child Dialog's trap effect.
  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  const groups = useMemo(() => buildShortcutGroups(), []);
  // Bookmarks and other numbered series render in NUMERIC order (1..10).
  const ordered = useMemo(
    () => groups.map((group) => ({ ...group, rows: sortRowsNumericAware(group.rows, m) })),
    [groups, m],
  );
  const visible = useMemo(() => filterGroups(ordered, m, query, scope), [ordered, m, query, scope]);

  const editLabel = (label: string): string => `${label} — ${m("shortcuts.edit.hint")}`;

  return (
    <Dialog open onClose={exit} labelledBy={titleId} className="shortcuts-dialog" closeOnOutsideClick>
      <div className="shortcuts" data-testid="shortcuts-window">
        <header className="shortcuts__topbar">
          <div className="shortcuts__titlerow">
            <h2 id={titleId} className="shortcuts__title">
              {m("shortcuts.title")}
            </h2>
            <button
              type="button"
              className="shortcuts__close"
              aria-label={t("shortcuts.close")}
              title={t("shortcuts.close")}
              onClick={exit}
            >
              <Icon name="close" size={24} />
            </button>
          </div>
          <div className="shortcuts__filters">
            <label className="shortcuts__search">
              <Icon name="search" size={24} className="shortcuts__search-icon" />
              <input
                ref={searchRef}
                type="search"
                value={query}
                placeholder={m("shortcuts.search.placeholder")}
                aria-label={m("shortcuts.search.label")}
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>
            <Dropdown
              className="shortcuts__filter"
              options={FILTER_OPTIONS}
              value={scope}
              onChange={(value) => setScope(value as ShortcutFilterScope)}
              label={t("shortcuts.filter.label")}
              resolveLabel={(key) => t(key)}
            />
          </div>
        </header>

        {visible.length === 0 ? (
          <div className="shortcuts__body">
            <p className="shortcuts__empty">{m("shortcuts.search.empty")}</p>
          </div>
        ) : (
          <div className="shortcuts__body">
            <div className="shortcuts__head">
              <div className="shortcuts__headrow">
                <span className="shortcuts__headcell">{m("settings.hotkeys.column.action")}</span>
                <span className="shortcuts__headcell">{t("shortcuts.column.hotkey")}</span>
                <span className="shortcuts__headcell">{t("shortcuts.column.global")}</span>
              </div>
              <hr className="shortcuts__rule" />
            </div>

            {visible.map((group, index) => (
              <section
                key={group.id}
                className="shortcuts__group"
                aria-labelledby={`shortcuts-group-${group.id}`}
              >
                <h3 id={`shortcuts-group-${group.id}`} className="shortcuts__group-title">
                  {m(group.labelKey)}
                </h3>
                <ul className="shortcuts__rows">
                  {group.rows.map((row) => (
                    <li key={row.id} className="shortcuts__row" data-binding={row.id}>
                      <span className="shortcuts__action">{m(row.labelKey)}</span>
                      <div className="shortcuts__keycap">
                        <span className="shortcuts__combo">{keycapCombo(row.key)}</span>
                        <Link
                          to="/settings/hotkeys"
                          className="shortcuts__edit"
                          aria-label={editLabel(m(row.labelKey))}
                        >
                          <Icon name="edit" size={24} />
                        </Link>
                      </div>
                      {/* Global hotkey: no binding in the keymaps data (VLC
                          ships global-key-* empty) — the frame's empty
                          keycap with its pencil affordance, decorative. */}
                      <div className="shortcuts__keycap shortcuts__keycap--global">
                        <span className="shortcuts__edit shortcuts__edit--idle" aria-hidden="true">
                          <Icon name="edit" size={24} />
                        </span>
                      </div>
                    </li>
                  ))}
                </ul>
                {index < visible.length - 1 ? <hr className="shortcuts__rule" /> : null}
              </section>
            ))}
          </div>
        )}

        <footer className="shortcuts__footer">
          <Button variant="default" className="shortcuts__cancel" onClick={exit}>
            {m("open.cancel")}
          </Button>
          <Button variant="tinted" className="shortcuts__save" disabled>
            {t("shortcuts.save")}
          </Button>
        </footer>
      </div>
    </Dialog>
  );
}
