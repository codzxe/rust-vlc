// Card 4.C1 — player layout-config consumption (doc 07 §7.1/§7.2).
//
// Doc 07 §7: "which panels, menus, toolbars and buttons exist, and their
// order, come from layout config". The production source is
// `config/layouts/*.toml` (`schema = "layout.v1"`, validated by
// schemas/layout.schema.json). That config system has NOT landed yet (no
// config/ directory at repo root as of this card) — [PROPOSAL] adaptation,
// recorded in ui/NEXT-STEPS.md:
//   * consumption side is fully built here: a TOML-subset parser for the
//     doc 07 §7.2 sketch format ([[region]] / [[menu]] / [[menu.item]] array
//     tables, inline tables, scalars),
//   * the default layout ships as a TOML *string* constant parsed by that
//     same parser, so swapping the source for a real config/layouts/main.toml
//     later changes nothing downstream,
//   * parse failures are loud (LayoutParseError), matching doc 07 §3.5's
//     "actionable diagnostic, never a silent fallback" philosophy; only a
//     MISSING/empty source falls back to the default layout.
//
// Layouts decide existence and order — never pixels (§7.1). All styling here
// is classnames + semantic tokens in player.css.

import { isMessageKey } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";

/** Error with the offending detail — honest diagnostics, no silent default. */
export class LayoutParseError extends Error {
  constructor(message: string) {
    super(`layout config: ${message}`);
    this.name = "LayoutParseError";
  }
}

/** Region anchors (doc 08 player anatomy: top bar, canvas, control bar, side panels). */
export type LayoutAnchor = "top" | "bottom" | "left" | "right" | "center";

/**
 * Condition tokens (doc 07 §7.3 — same grammar as keymaps `when`). Card
 * 4.F2 added the variant-scoped tokens "miniplayer" / "music-mode": a
 * region carrying them is visible only in that player variant (doc 08 §4
 * lists layout mode as UI-owned state; variant layouts in
 * variants/variantLayout.ts consume them).
 */
export type LayoutConditionToken =
  | "has-media"
  | "has-video"
  | "fullscreen"
  | "has-chapters"
  | "miniplayer"
  | "music-mode";

/**
 * Runtime values for the condition tokens above. The variant flags are
 * optional for backward compatibility with pre-4.F2 callers; absent means
 * "not in that variant" (honest default — the full layout).
 */
export interface LayoutConditionContext {
  readonly hasMedia: boolean;
  readonly hasVideo: boolean;
  readonly fullscreen: boolean;
  readonly hasChapters: boolean;
  /** True while the player runs the miniplayer variant (4.F2). */
  readonly miniplayer?: boolean;
  /** True while the player runs the music variant (4.F2). */
  readonly musicMode?: boolean;
}

/** A declared UI region (doc 07 §7.2 `[[region]]`). */
export interface LayoutRegion {
  readonly id: string;
  readonly anchor: LayoutAnchor;
  readonly labelKey: MessageKey;
  /** Condition token; absent = always visible. */
  readonly visibleWhen?: LayoutConditionToken;
  /** Sort order within the layout; defaults to declaration order. */
  readonly order: number;
}

/** A menu leaf (doc 07 §7.2 / doc 05 §8.3). */
export interface LayoutMenuItem {
  readonly id: string;
  readonly labelKey: MessageKey;
  readonly command?: string;
  readonly uiAction?: string;
  /** Accelerator binding id — keymaps are referenced by id, never raw keys. */
  readonly binding?: string;
  readonly order: number;
}

/** A menu tree (doc 07 §7.2 `layouts/menus.toml` — `[[menu]]`). */
export interface LayoutMenu {
  readonly id: string;
  readonly labelKey: MessageKey;
  readonly items: readonly LayoutMenuItem[];
}

export interface LayoutConfig {
  readonly schema: "layout.v1";
  readonly regions: readonly LayoutRegion[];
  readonly menus: readonly LayoutMenu[];
}

// ---------------------------------------------------------------------------
// TOML-subset parser. Supported syntax (everything doc 07's sketches use):
//   comments (#), blank lines, [section], [[array-of-tables]],
//   [[parent.item]] (appends to the last [[parent]] element),
//   key = "string" | integer | true/false | { inline table of those }
// Anything else is a parse error — no guessing.
// ---------------------------------------------------------------------------

type TomlTable = Record<string, unknown>;

function stripComment(line: string): string {
  let inString = false;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (ch === '"') inString = !inString;
    else if (ch === "#" && !inString) return line.slice(0, i);
  }
  return line;
}

function parseScalar(raw: string): unknown {
  const s = raw.trim();
  if (s.startsWith('"')) {
    if (!s.endsWith('"') || s.length < 2) throw new LayoutParseError(`unterminated string: ${raw}`);
    return s
      .slice(1, -1)
      .replace(/\\(.)/g, (_, c: string) => (c === "n" ? "\n" : c === "t" ? "\t" : c));
  }
  if (s === "true") return true;
  if (s === "false") return false;
  if (/^-?\d+$/.test(s)) return Number.parseInt(s, 10);
  throw new LayoutParseError(`unsupported value: ${raw}`);
}

function parseInlineTable(raw: string): TomlTable {
  const body = raw.trim();
  if (!body.startsWith("{") || !body.endsWith("}")) {
    throw new LayoutParseError(`not an inline table: ${raw}`);
  }
  const table: TomlTable = {};
  const inner = body.slice(1, -1).trim();
  if (inner === "") return table;
  // split on commas that are not inside a string
  const parts: string[] = [];
  let depth = 0;
  let inString = false;
  let start = 0;
  for (let i = 0; i < inner.length; i++) {
    const ch = inner[i];
    if (ch === '"') inString = !inString;
    else if (!inString && ch === "{") depth++;
    else if (!inString && ch === "}") depth--;
    else if (!inString && ch === "," && depth === 0) {
      parts.push(inner.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(inner.slice(start));
  for (const part of parts) {
    const kv = /^([A-Za-z0-9_-]+)\s*=\s*(.+)$/.exec(part.trim());
    if (!kv || kv[1] === undefined || kv[2] === undefined) {
      throw new LayoutParseError(`unparseable table entry: ${part}`);
    }
    const value = kv[2].trim();
    table[kv[1]] = value.startsWith("{") ? parseInlineTable(value) : parseScalar(value);
  }
  return table;
}

function parseValue(raw: string): unknown {
  const s = raw.trim();
  return s.startsWith("{") ? parseInlineTable(s) : parseScalar(s);
}

/** Parse the TOML subset into a plain tree. */
export function parseTomlSubset(src: string): TomlTable {
  const root: TomlTable = {};
  let current: TomlTable = root;
  for (const rawLine of src.split(/\r?\n/)) {
    const line = stripComment(rawLine).trim();
    if (line === "") continue;
    const arrayHeader = /^\[\[(.+?)\]\]$/.exec(line);
    const tableHeader = arrayHeader ? null : /^\[(.+?)\]$/.exec(line);
    if (arrayHeader && arrayHeader[1] !== undefined) {
      const path = arrayHeader[1].trim().split(".");
      if (path.length > 2) throw new LayoutParseError(`nested path too deep: ${line}`);
      const parent = path.length === 2 ? lastOf(root, path[0] ?? "") : root;
      current = pushTable(ensureArray(parent, path[path.length - 1] ?? ""));
    } else if (tableHeader && tableHeader[1] !== undefined) {
      const name = tableHeader[1].trim();
      if (name.includes(".")) throw new LayoutParseError(`dotted table header unsupported: ${line}`);
      if (root[name] === undefined) root[name] = {};
      if (!isTable(root[name])) throw new LayoutParseError(`section [${name}] conflicts with a value`);
      current = root[name];
    } else {
      const kv = /^([A-Za-z0-9_-]+)\s*=\s*(.+)$/.exec(line);
      if (!kv || kv[1] === undefined || kv[2] === undefined) {
        throw new LayoutParseError(`unparseable line: ${rawLine.trim()}`);
      }
      current[kv[1]] = parseValue(kv[2]);
    }
  }
  return root;
}

function isTable(v: unknown): v is TomlTable {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function ensureArray(parent: TomlTable, key: string): unknown[] {
  const existing = parent[key];
  if (existing === undefined) {
    const arr: unknown[] = [];
    parent[key] = arr;
    return arr;
  }
  if (!Array.isArray(existing)) throw new LayoutParseError(`[${key}] conflicts with a non-array value`);
  return existing;
}

function lastOf(parent: TomlTable, key: string): TomlTable {
  const arr = ensureArray(parent, key);
  const last = arr[arr.length - 1];
  if (!isTable(last)) throw new LayoutParseError(`[[${key}.…]] before any [[${key}]]`);
  return last;
}

function pushTable(arr: unknown[]): TomlTable {
  const table: TomlTable = {};
  arr.push(table);
  return table;
}

// ---------------------------------------------------------------------------
// Mapping + validation (consumption side of doc 07's validator rules)
// ---------------------------------------------------------------------------

const ANCHORS: readonly LayoutAnchor[] = ["top", "bottom", "left", "right", "center"];
const CONDITION_TOKENS: readonly LayoutConditionToken[] = [
  "has-media",
  "has-video",
  "fullscreen",
  "has-chapters",
  "miniplayer",
  "music-mode",
];

function requireString(v: unknown, what: string): string {
  if (typeof v !== "string" || v.length === 0) throw new LayoutParseError(`${what} must be a non-empty string`);
  return v;
}

function requireLabelKey(v: unknown, what: string): MessageKey {
  const key = requireString(v, what);
  if (!isMessageKey(key)) {
    // doc 07 §7.2 validator rule: every label_key must resolve (§6.4 rule 4).
    throw new LayoutParseError(`${what}: unknown label_key "${key}"`);
  }
  return key;
}

function parseRegion(row: unknown, fallbackOrder: number): LayoutRegion {
  if (!isTable(row)) throw new LayoutParseError("[[region]] entry must be a table");
  const id = requireString(row["id"], "region.id");
  const anchorRaw = requireString(row["anchor"], `region "${id}": anchor`);
  if (!ANCHORS.includes(anchorRaw as LayoutAnchor)) {
    throw new LayoutParseError(
      `region "${id}": unknown anchor "${anchorRaw}" (expected one of ${ANCHORS.join(", ")})`,
    );
  }
  const visibleRaw = row["visible_when"];
  if (visibleRaw !== undefined && !CONDITION_TOKENS.includes(visibleRaw as LayoutConditionToken)) {
    throw new LayoutParseError(`region "${id}": unknown condition token "${String(visibleRaw)}"`);
  }
  const order = typeof row["order"] === "number" ? row["order"] : fallbackOrder;
  return {
    id,
    anchor: anchorRaw as LayoutAnchor,
    labelKey: requireLabelKey(row["label_key"], `region "${id}": label_key`),
    visibleWhen: visibleRaw === undefined ? undefined : (visibleRaw as LayoutConditionToken),
    order,
  };
}

function parseMenuItem(row: unknown, fallbackOrder: number): LayoutMenuItem {
  if (!isTable(row)) throw new LayoutParseError("[[menu.item]] entry must be a table");
  const id = requireString(row["id"], "menu.item.id");
  const command = row["command"];
  const uiAction = row["ui_action"];
  const binding = row["binding"];
  return {
    id,
    labelKey: requireLabelKey(row["label_key"], `menu item "${id}": label_key`),
    command: command === undefined ? undefined : requireString(command, `menu item "${id}": command`),
    uiAction: uiAction === undefined ? undefined : requireString(uiAction, `menu item "${id}": ui_action`),
    binding: binding === undefined ? undefined : requireString(binding, `menu item "${id}": binding`),
    order: typeof row["order"] === "number" ? row["order"] : fallbackOrder,
  };
}

function parseMenu(row: unknown): LayoutMenu {
  if (!isTable(row)) throw new LayoutParseError("[[menu]] entry must be a table");
  const id = requireString(row["id"], "menu.id");
  const labelKey = requireLabelKey(row["label_key"], `menu "${id}": label_key`);
  const itemRows = row["item"];
  const items = Array.isArray(itemRows)
    ? itemRows.map((r, i) => parseMenuItem(r, i)).sort(byOrder)
    : [];
  return { id, labelKey, items };
}

function byOrder<T extends { order: number }>(a: T, b: T): number {
  return a.order - b.order;
}

/** Parse a layout.v1 TOML source. Throws LayoutParseError on any violation. */
export function parseLayoutConfig(src: string): LayoutConfig {
  const root = parseTomlSubset(src);
  if (root["schema"] !== "layout.v1") {
    throw new LayoutParseError(`unsupported schema "${String(root["schema"])}" (expected "layout.v1")`);
  }
  const regionRows = root["region"];
  if (!Array.isArray(regionRows) || regionRows.length === 0) {
    throw new LayoutParseError("at least one [[region]] is required");
  }
  const regions = regionRows.map((r, i) => parseRegion(r, i)).sort(byOrder);
  const seen = new Set<string>();
  for (const region of regions) {
    if (seen.has(region.id)) throw new LayoutParseError(`duplicate region id "${region.id}"`);
    seen.add(region.id);
  }
  const menuRows = root["menu"];
  const menus = (Array.isArray(menuRows) ? (menuRows as unknown[]) : []).map(parseMenu);
  return { schema: "layout.v1", regions, menus };
}

// ---------------------------------------------------------------------------
// Default layout [PROPOSAL] — mirrors the doc 07 §7.2 sketch + doc 08 §1.6
// player anatomy (top bar / canvas / control bar / library sidebar + playlist
// panel + media-info panel). Region and label existence is config-owned;
// panel CONTENT is later cards' work (4.C2 controls, 4.F3 library/playlist).
// ---------------------------------------------------------------------------

export const DEFAULT_LAYOUT_TOML = `# [PROPOSAL] default player layout (card 4.C1) — stands in for the
# not-yet-landed config/layouts/main.toml (doc 07 §7.2). Same format.
schema = "layout.v1"

[[region]]
id = "video-canvas"
anchor = "center"
label_key = "player.canvas.label"
order = 10

[[region]]
id = "title-bar"
anchor = "top"
label_key = "player.region.title-bar"
order = 10

[[region]]
id = "controls-bar"
anchor = "bottom"
label_key = "player.region.controls-bar"
order = 10

[[region]]
id = "library"
anchor = "left"
label_key = "player.region.library"
order = 10

[[region]]
id = "playlist"
anchor = "right"
label_key = "player.region.playlist"
order = 10

[[region]]
id = "media-info"
anchor = "right"
label_key = "player.region.media-info"
order = 20
visible_when = "has-media"

# Card 4.F3 — the customize surface (Crop/Aspect, Video Colors, Essential,
# HDR panels). Content is capability-gated per field; the panel itself opens
# through the same openPanels seam as the other side panels.
[[region]]
id = "customize"
anchor = "right"
label_key = "player.region.customize"
order = 30
visible_when = "has-media"

[[menu]]
id = "app"
label_key = "player.menu.app"

[[menu.item]]
id = "open-media"
label_key = "player.empty.open"
ui_action = "open-media"
order = 10

[[menu.item]]
id = "customize-player"
label_key = "player.region.customize"
ui_action = "open-panel:customize"
order = 20
`;

/** Parsed default layout (module-load; a bad default is a programmer error). */
export const DEFAULT_LAYOUT: LayoutConfig = parseLayoutConfig(DEFAULT_LAYOUT_TOML);

/**
 * Resolve the effective layout. A missing/empty source falls back to the
 * default layout; a PRESENT but invalid source throws (doc 07 §3.5 — never a
 * silent fallback on bad config).
 */
export function resolveLayout(source: string | null | undefined): LayoutConfig {
  if (source === null || source === undefined || source.trim() === "") return DEFAULT_LAYOUT;
  return parseLayoutConfig(source);
}

/** Evaluate a region's `visible_when` token against runtime state. */
export function evaluateCondition(
  token: LayoutConditionToken | undefined,
  ctx: LayoutConditionContext,
): boolean {
  switch (token) {
    case undefined:
      return true;
    case "has-media":
      return ctx.hasMedia;
    case "has-video":
      return ctx.hasVideo;
    case "fullscreen":
      return ctx.fullscreen;
    case "has-chapters":
      return ctx.hasChapters;
    case "miniplayer":
      return ctx.miniplayer === true;
    case "music-mode":
      return ctx.musicMode === true;
  }
}
