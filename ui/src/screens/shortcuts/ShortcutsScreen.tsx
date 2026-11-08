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
// Grouping policy (keymaps sections → the four pre-seeded shortcuts.group.*
// groups): media/playlist/subtitles → playback; navigation/advanced →
// navigation; audio → volume; video/window + the shell variant seeds and
// the platform-variant (*-macos) rows → variants.
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useMemo, useState } from "react";
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

/** Case-insensitive filter over the localized label + the combo spelling. */
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
          row.key.toLowerCase().includes(needle),
      ),
    }))
    .filter((group) => group.rows.length > 0);
}

/** Split a combo spelling ("Ctrl+Shift+Right") into keycap chip texts. */
export function keyChips(combo: string): readonly string[] {
  return combo
    .split("+")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

export function ShortcutsScreen() {
  const m = useMsg();
  const [query, setQuery] = useState("");
  const groups = useMemo(() => buildShortcutGroups(), []);
  const visible = useMemo(() => filterGroups(groups, m, query), [groups, m, query]);

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
              className="shortcuts__group solstice-note solstice-note--flat"
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
                        <kbd key={`${row.id}-${index}`} className="solstice-key solstice-key--sm">
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
