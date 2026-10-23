#!/usr/bin/env node
// gen-hotkeys-data.mjs — card 4.E2, doc 07 §5 (hotkeys editor data).
//
//   keymaps/default.toml   (card 0.B3 G6 seed — READ-ONLY source of truth)
//          └▶ THIS SCRIPT (zero deps: reuses the TOML-subset parser exported
//              by gen-settings-screens.mjs, which handles dotted section
//              headers + nested inline tables — player/layout.ts's parser
//              cannot, hence this dedicated emitter)
//               └▶ ui/src/screens/settings/areas/hotkeys/generated/bindings.ts
//
// Hotkey bindings are DYNAMIC data (one row per keymaps/default.toml binding),
// not schema-static [[setting]] entries — the settings-schema owner's ~45-entry
// seed (doc 07 §5.4) intentionally excludes them. This emitter is the recorded
// mechanism choice: the editor renders from the real keymap file via a
// committed generated module + byte-compare drift guard, mirroring
// settings:generate / tokens:generate.
//
// `node scripts/gen-hotkeys-data.mjs`          regenerate (idemponent)
// `node scripts/gen-hotkeys-data.mjs --check`  exit 1 on drift

import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseToml } from "./gen-settings-screens.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, "..");
const repoRoot = resolve(uiRoot, "..");
const defaultInput = resolve(repoRoot, "keymaps", "default.toml");
const defaultOut = resolve(uiRoot, "src", "screens", "settings", "areas", "hotkeys", "generated", "bindings.ts");

export function buildHotkeyModel(toml) {
  const root = parseToml(toml);
  const rows = [];
  const seen = new Set();
  for (const [section, table] of Object.entries(root)) {
    if (typeof table !== "object" || table === null || Array.isArray(table)) continue;
    if (section === "platform_overrides") continue; // platform tables are additive, not editor rows
    for (const [id, row] of Object.entries(table)) {
      if (typeof row !== "object" || row === null || Array.isArray(row)) continue;
      if (typeof row["key"] !== "string") continue; // not a binding row (e.g. schema = "keymap.v1")
      const key = `${section}.${id}`;
      if (seen.has(key)) throw new Error(`keymap: duplicate binding id "${key}"`);
      seen.add(key);
      const str = (v) => (typeof v === "string" ? v : undefined);
      rows.push({
        id: key,
        section,
        key: row["key"],
        labelMsgid: typeof row["label-msgid"] === "string" ? row["label-msgid"] : `hotkeys.${id}`,
        command: str(row["command"]),
        recipe: str(row["recipe"]),
        uiAction: str(row["ui_action"]),
        mode: str(row["mode"]),
        when: str(row["when"]),
        disabled: row["disabled"] === true,
      });
    }
  }
  // platform_overrides.<platform> rows patch base bindings (key overrides) —
  // surfaced as data, not editor rows.
  const platform = root["platform_overrides"];
  const platformKeys = [];
  if (typeof platform === "object" && platform !== null) {
    for (const [platformId, table] of Object.entries(platform)) {
      if (typeof table !== "object" || table === null) continue;
      for (const [id, row] of Object.entries(table)) {
        if (typeof row !== "object" || row === null || Array.isArray(row)) continue;
        if (typeof row["key"] !== "string") continue;
        platformKeys.push({ platform: platformId, id, key: row["key"] });
      }
    }
  }
  rows.sort((a, b) => (a.section === b.section ? a.id.localeCompare(b.id) : a.section.localeCompare(b.section)));
  return { rows, platformKeys };
}

export function emitBindingsTs(model) {
  const lines = [];
  lines.push("// GENERATED FILE — do not edit (card 4.E2 hotkeys editor data).");
  lines.push("// Source of truth: keymaps/default.toml (card 0.B3). Regenerate:");
  lines.push("//   npm run hotkeys:generate    Drift guard: npm run hotkeys:check");
  lines.push("/* eslint-disable */");
  lines.push("");
  lines.push("export interface HotkeyBinding {");
  lines.push("  /** Dotted id: <section>.<binding-id> — stable across edits. */");
  lines.push("  readonly id: string;");
  lines.push("  /** keymaps/default.toml section ([media], [navigation], …). */");
  lines.push("  readonly section: string;");
  lines.push("  /** Default key combo spelling, e.g. \"Ctrl+O\", \"Space\". */");
  lines.push("  readonly key: string;");
  lines.push("  /** Action label msgid (keymaps label-msgid). */");
  lines.push("  readonly labelMsgid: string;");
  lines.push("  /** Command id (doc 05 §2), when the binding routes to one. */");
  lines.push("  readonly command?: string;");
  lines.push("  /** Shell recipe id (doc 05 §2.7), when the binding routes to one. */");
  lines.push("  readonly recipe?: string;");
  lines.push("  /** UI action id (shell-local), when the binding routes to one. */");
  lines.push("  readonly uiAction?: string;");
  lines.push("  /** Activation mode (\"player-focused\", \"always\", …). */");
  lines.push("  readonly mode?: string;");
  lines.push("  /** Condition token (\"has-media\", …). */");
  lines.push("  readonly when?: string;");
  lines.push("  /** Ships disabled from data (feature-gated source, doc 05 §6.4). */");
  lines.push("  readonly disabled: boolean;");
  lines.push("}");
  lines.push("");
  lines.push("export const HOTKEY_BINDINGS: readonly HotkeyBinding[] = [");
  for (const r of model.rows) {
    const opt = (v) => (v === undefined ? "undefined" : JSON.stringify(v));
    lines.push(
      `  { id: ${JSON.stringify(r.id)}, section: ${JSON.stringify(r.section)}, key: ${JSON.stringify(r.key)}, labelMsgid: ${JSON.stringify(r.labelMsgid)}, command: ${opt(r.command)}, recipe: ${opt(r.recipe)}, uiAction: ${opt(r.uiAction)}, mode: ${opt(r.mode)}, when: ${opt(r.when)}, disabled: ${r.disabled} },`,
    );
  }
  lines.push("];");
  lines.push("");
  lines.push("/** Platform key overrides (keymaps/default.toml [platform_overrides]). */");
  lines.push("export const HOTKEY_PLATFORM_OVERRIDES: readonly { platform: string; id: string; key: string }[] = [");
  for (const p of model.platformKeys) {
    lines.push(`  { platform: ${JSON.stringify(p.platform)}, id: ${JSON.stringify(p.id)}, key: ${JSON.stringify(p.key)} },`);
  }
  lines.push("];");
  lines.push("");
  return lines.join("\n");
}

async function main() {
  const args = process.argv.slice(2);
  const check = args.includes("--check");
  const inputIdx = args.indexOf("--input");
  const outIdx = args.indexOf("--out");
  const inputPath = inputIdx >= 0 ? resolve(args[inputIdx + 1]) : defaultInput;
  const outPath = outIdx >= 0 ? resolve(args[outIdx + 1]) : defaultOut;

  const model = buildHotkeyModel(await readFile(inputPath, "utf8"));
  const content = emitBindingsTs(model);

  if (check) {
    let committed = null;
    try {
      committed = await readFile(outPath, "utf8");
    } catch {
      /* missing = drifted */
    }
    if (committed !== content) {
      console.error(
        `gen-hotkeys-data --check: ${outPath} is stale vs ${inputPath} — run \`node scripts/gen-hotkeys-data.mjs\``,
      );
      process.exit(1);
    }
    console.log(`gen-hotkeys-data --check: OK (${model.rows.length} bindings, ${model.platformKeys.length} platform overrides)`);
    return;
  }

  await mkdir(dirname(outPath), { recursive: true });
  await writeFile(outPath, content, "utf8");
  console.log(`gen-hotkeys-data: wrote ${outPath} (${model.rows.length} bindings)`);
}

const isDirectRun = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isDirectRun) {
  main().catch((err) => {
    console.error(`gen-hotkeys-data: ${err instanceof Error ? err.message : String(err)}`);
    process.exit(1);
  });
}
