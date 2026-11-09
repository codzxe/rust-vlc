// Card 4.C1 — keymap-data-driven shortcuts seam (doc 05 §8.1, doc 07 §7.3).
//
// Keymap seeds live in keymaps/default.toml at repo root (binding-id keyed
// inline tables: `fullscreen = { key = "F", recipe = "fullscreen-toggle",
// mode = "player-focused", … }`). The keymap LOADING pipeline has not landed
// (doc 05 §8.1 vs MP §4 path mismatch is open question Q4 in
// docs/inventory/ui-inventory.md §9) — [PROPOSAL] adaptation, recorded in
// ui/NEXT-STEPS.md: this module is the full consumption side (parser +
// matcher + shell recipe dispatch), seeded from a verbatim excerpt of the
// three shell-relevant bindings from keymaps/default.toml lines 154-156.
// When the loader lands, swap DEFAULT_KEYMAP_TOML for the loaded source.
//
// The shell claims ONLY plain-key bindings in mode "player-focused"; bindings
// with modifiers (e.g. "Cmd+F") are parsed but not matched here — modifier
// dispatch and full command routing are card 4.C2's command layer.

import { parseTomlSubset } from "./layout";

export interface KeyBinding {
  readonly id: string;
  /** Raw keymap spelling, e.g. "F", "Esc", "Cmd+F". */
  readonly key: string;
  readonly command?: string;
  readonly recipe?: string;
  readonly uiAction?: string;
  readonly mode?: string;
  readonly when?: string;
}

/** Parse a keymap TOML source (any sections; binding rows are inline tables). */
export function parseKeymap(src: string): ReadonlyMap<string, KeyBinding> {
  const root = parseTomlSubset(src);
  const bindings = new Map<string, KeyBinding>();
  const walk = (table: Record<string, unknown>, section: string): void => {
    for (const [key, value] of Object.entries(table)) {
      if (typeof value !== "object" || value === null || Array.isArray(value)) continue;
      const row = value as Record<string, unknown>;
      if (typeof row["key"] !== "string") continue; // not a binding row
      if (bindings.has(key)) {
        throw new Error(`keymap: duplicate binding id "${key}" (section [${section}])`);
      }
      const binding: KeyBinding = {
        id: key,
        key: row["key"],
        command: typeof row["command"] === "string" ? row["command"] : undefined,
        recipe: typeof row["recipe"] === "string" ? row["recipe"] : undefined,
        uiAction: typeof row["ui_action"] === "string" ? row["ui_action"] : undefined,
        mode: typeof row["mode"] === "string" ? row["mode"] : undefined,
        when: typeof row["when"] === "string" ? row["when"] : undefined,
      };
      bindings.set(key, binding);
    }
  };
  for (const [name, section] of Object.entries(root)) {
    if (typeof section === "object" && section !== null && !Array.isArray(section)) {
      walk(section as Record<string, unknown>, name);
    }
  }
  return bindings;
}

/**
 * [PROPOSAL] Shell keymap seed — verbatim excerpt of the fullscreen-related
 * bindings from keymaps/default.toml (lines 154-156, source annotations
 * dropped; see that file for full provenance). Full keymap arrives with the
 * loader (see header note).
 */
export const DEFAULT_KEYMAP_TOML = `# [PROPOSAL] excerpt of keymaps/default.toml (card 4.C1 shell seam).
[player-fullscreen]
fullscreen = { key = "F", recipe = "fullscreen-toggle", mode = "player-focused" }
exit-fullscreen = { key = "Esc", recipe = "exit-fullscreen-close-dialog", mode = "player-focused" }
show-controls-fullscreen = { key = "I", ui_action = "show-controls-fullscreen", mode = "player-focused" }
`;

/** The parsed seed (module-load; a bad seed is a programmer error). */
export const DEFAULT_KEYMAP: ReadonlyMap<string, KeyBinding> = parseKeymap(DEFAULT_KEYMAP_TOML);

/**
 * [PROPOSAL] Dialog-opening seed — modifier-combo rows this screen
 * implements as dialogs, copied VERBATIM (key/ui_action/mode/when) from
 * keymaps/default.toml (provenance annotations live in that file). The full
 * keymap arrives with the loader (see DEFAULT_KEYMAP_TOML's header note);
 * these rows move into it unchanged when it lands.
 */
export const DIALOG_KEYMAP_TOML = `# [PROPOSAL] excerpt of keymaps/default.toml dialog rows (issue #32).
[player-dialogs]
jump-to-time = { key = "Ctrl+T", ui_action = "goto-time", mode = "player-focused", when = "has-media" }
bookmarks-dialog = { key = "Ctrl+B", ui_action = "bookmarks", mode = "player-focused" }
`;

/** The parsed dialog seed (module-load; a bad seed is a programmer error). */
export const DIALOG_KEYMAP: ReadonlyMap<string, KeyBinding> =
  parseKeymap(DIALOG_KEYMAP_TOML);

/** A normalized key spec: lowercased DOM key + claimed modifiers. */
export interface NormalizedKey {
  readonly key: string;
  readonly modifiers: readonly string[];
}

/**
 * Normalize a keymap spelling to a DOM KeyboardEvent shape.
 * "F" → {key:"f"}, "Esc" → {key:"escape"}, "Cmd+F" → {key:"f",modifiers:["meta"]}.
 * Returns null for unparseable spellings (never guessed).
 */
export function normalizeBindingKey(raw: string): NormalizedKey | null {
  const parts = raw.split("+").map((p) => p.trim()).filter(Boolean);
  if (parts.length === 0) return null;
  const claimed: string[] = [];
  for (const part of parts.slice(0, -1)) {
    switch (part.toLowerCase()) {
      case "cmd":
      case "meta":
      case "super":
        claimed.push("meta");
        break;
      case "ctrl":
        claimed.push("control");
        break;
      case "alt":
      case "option":
        claimed.push("alt");
        break;
      case "shift":
        claimed.push("shift");
        break;
      default:
        return null; // unknown modifier — don't guess
    }
  }
  let key = (parts[parts.length - 1] ?? "").toLowerCase();
  if (key === "esc") key = "escape"; // keymap spelling vs DOM KeyboardEvent.key
  if (key.length === 0) return null;
  return { key, modifiers: claimed };
}

/** The minimal event shape needed for matching. */
export interface MatchableKeyboardEvent {
  readonly key: string;
  readonly metaKey: boolean;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
}

/**
 * Match a DOM key event against bindings — data-driven, not hardcoded. Only
 * plain-key bindings (no modifiers) in mode "player-focused" are claimed by
 * the shell; everything else is left for 4.C2's dispatcher.
 */
export function matchBindings(
  bindings: ReadonlyMap<string, KeyBinding>,
  event: MatchableKeyboardEvent,
): readonly KeyBinding[] {
  const hits: KeyBinding[] = [];
  const anyModifier = event.metaKey || event.ctrlKey || event.altKey || event.shiftKey;
  for (const binding of bindings.values()) {
    if (binding.mode !== undefined && binding.mode !== "player-focused") continue;
    const normalized = normalizeBindingKey(binding.key);
    if (normalized === null || normalized.modifiers.length > 0) continue;
    if (normalized.key !== event.key.toLowerCase()) continue;
    if (anyModifier) continue; // a plain-key binding never fires on combos
    hits.push(binding);
  }
  return hits;
}

/** The modifiers a DOM event holds, in normalizeBindingKey's spelling. */
function pressedModifiers(event: MatchableKeyboardEvent): readonly string[] {
  const pressed: string[] = [];
  if (event.metaKey) pressed.push("meta");
  if (event.ctrlKey) pressed.push("control");
  if (event.altKey) pressed.push("alt");
  if (event.shiftKey) pressed.push("shift");
  return pressed;
}

/** Same-modifier test (order-insensitive; spellings are unique per key). */
function sameModifiers(
  claimed: readonly string[],
  pressed: readonly string[],
): boolean {
  if (claimed.length !== pressed.length) return false;
  const rest = [...pressed];
  for (const modifier of claimed) {
    const index = rest.indexOf(modifier);
    if (index === -1) return false;
    rest.splice(index, 1);
  }
  return true;
}

/**
 * The modifier-combo twin of [`matchBindings`] (issue #32): matches ONLY
 * bindings whose key spec carries modifiers, and only when the event's
 * modifier set matches EXACTLY — "Ctrl+T" fires on Ctrl+T alone, never on
 * Ctrl+Shift+T or a bare "t". `when` gating (e.g. "has-media") is the
 * caller's truth to evaluate — the matcher has no media state.
 */
export function matchModifierBindings(
  bindings: ReadonlyMap<string, KeyBinding>,
  event: MatchableKeyboardEvent,
): readonly KeyBinding[] {
  const hits: KeyBinding[] = [];
  const pressed = pressedModifiers(event);
  for (const binding of bindings.values()) {
    if (binding.mode !== undefined && binding.mode !== "player-focused") continue;
    const normalized = normalizeBindingKey(binding.key);
    if (normalized === null || normalized.modifiers.length === 0) continue;
    if (normalized.key !== event.key.toLowerCase()) continue;
    if (!sameModifiers(normalized.modifiers, pressed)) continue;
    hits.push(binding);
  }
  return hits;
}

/**
 * The recipe/ui_action names the shell itself implements (doc 05 §2.7 —
 * `video.set-fullscreen` is target:shell). Values come from keymap DATA;
 * this table only declares which recipes are shell-owned. Everything else is
 * forwarded to the onBinding seam for card 4.C2's command dispatch.
 */
/** The recipes/ui-actions the shell itself implements (see shellActionFor).
 * Card 4.F2 added the variant-toggle recipes — UI-owned layout state per
 * doc 08 §4, so no engine command exists; the seed rows live in
 * variants/variantKeymap.ts [PROPOSAL] until keymaps/default.toml carries
 * them. */
type ShellActionKind =
  | "toggle-fullscreen"
  | "exit-fullscreen"
  | "show-controls"
  | "toggle-miniplayer"
  | "toggle-music-mode";

export type PlayerShellAction =
  | { readonly kind: ShellActionKind }
  | { readonly kind: "forward"; readonly binding: KeyBinding };

const SHELL_RECIPES: Readonly<Record<string, ShellActionKind>> = {
  "fullscreen-toggle": "toggle-fullscreen",
  "exit-fullscreen-close-dialog": "exit-fullscreen",
  "miniplayer-toggle": "toggle-miniplayer",
  "music-mode-toggle": "toggle-music-mode",
};
const SHELL_UI_ACTIONS: Readonly<Record<string, ShellActionKind>> = {
  "show-controls-fullscreen": "show-controls",
};

/** Map a matched binding to what the shell does with it. */
export function shellActionFor(binding: KeyBinding): PlayerShellAction {
  if (binding.recipe !== undefined) {
    const kind = SHELL_RECIPES[binding.recipe];
    if (kind !== undefined) return { kind };
  }
  if (binding.uiAction !== undefined) {
    const kind = SHELL_UI_ACTIONS[binding.uiAction];
    if (kind !== undefined) return { kind };
  }
  return { kind: "forward", binding };
}
