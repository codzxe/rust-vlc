#!/usr/bin/env node
// gen-settings-screens.mjs — card 4.E1, doc 07 §5.3 generator G1 (UI side).
//
//   schemas/settings.toml   (authored source of truth, card 1.B2 — READ-ONLY)
//     + product.toml        (default_ref targets + [features] exposure — READ-ONLY)
//          └▶ THIS SCRIPT (node, zero deps — a TOML-subset parser suffices:
//              both files use only tables, array-of-tables, inline tables,
//              strings/bools/numbers/string-arrays)
//               ├▶ ui/src/screens/settings/generated/schema.ts  (typed data)
//               └▶ ui/src/screens/settings/generated/Screens.tsx (one screen
//                   component per settings area, statically bound to entries)
//
// `node scripts/gen-settings-screens.mjs`                regenerate (idempotent)
// `node scripts/gen-settings-screens.mjs --check`        exit 1 on drift
// `--input <toml>` / `--out-dir <dir>`                   overrides for the
//                                                        drift-detection test
//
// The script NEVER writes outside ui/src/screens/settings/generated/ (unless
// --out-dir). crates/settings-schema and schemas/ are untouched.

import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, "..");
const repoRoot = resolve(uiRoot, "..");
const defaultInput = resolve(repoRoot, "schemas", "settings.toml");
const productTomlPath = resolve(repoRoot, "product.toml");
const defaultOutDir = resolve(uiRoot, "src", "screens", "settings", "generated");

// ---------------------------------------------------------------------------
// TOML-subset parser (tables [a.b], array-of-tables [[x]], key = value where
// value is a basic string, bool, int/float, array of those, or inline table).
// ---------------------------------------------------------------------------

class TomlError extends Error {}

export function parseToml(text) {
  const root = {};
  let current = root;
  let i = 0;
  const n = text.length;

  const skipWs = () => {
    while (i < n) {
      const c = text[i];
      if (c === " " || c === "\t" || c === "\r" || c === "\n") {
        i++;
      } else if (c === "#") {
        while (i < n && text[i] !== "\n") i++;
      } else {
        break;
      }
    }
  };

  const parseString = () => {
    i++; // opening quote
    let out = "";
    while (i < n && text[i] !== '"') {
      if (text[i] === "\\") {
        const e = text[i + 1];
        if (e === '"' || e === "\\") {
          out += e;
          i += 2;
          continue;
        }
      }
      out += text[i];
      i++;
    }
    if (i >= n) throw new TomlError("unterminated string");
    i++; // closing quote
    return out;
  };

  const parseValue = () => {
    skipWs();
    const c = text[i];
    if (c === '"') return parseString();
    if (c === "[") {
      i++;
      const arr = [];
      skipWs();
      if (text[i] === "]") {
        i++;
        return arr;
      }
      for (;;) {
        arr.push(parseValue());
        skipWs();
        if (text[i] === ",") {
          i++;
          skipWs();
          if (text[i] === "]") {
            i++;
            return arr;
          }
          continue;
        }
        if (text[i] === "]") {
          i++;
          return arr;
        }
        throw new TomlError("unterminated array");
      }
    }
    if (c === "{") {
      i++;
      const obj = {};
      skipWs();
      if (text[i] === "}") {
        i++;
        return obj;
      }
      for (;;) {
        skipWs();
        const key = parseBareKey();
        skipWs();
        expect("=");
        obj[key] = parseValue();
        skipWs();
        if (text[i] === ",") {
          i++;
          continue;
        }
        if (text[i] === "}") {
          i++;
          return obj;
        }
        throw new TomlError("unterminated inline table");
      }
    }
    // bare token: bool or number
    let j = i;
    while (j < n && !",]}#\n".includes(text[j])) j++;
    const raw = text.slice(i, j).trim();
    i = j;
    if (raw === "true") return true;
    if (raw === "false") return false;
    if (raw !== "" && /^[-+0-9._eE]+$/.test(raw)) {
      const num = Number(raw.replace(/_/g, ""));
      if (!Number.isNaN(num)) return num;
    }
    throw new TomlError(`unsupported TOML value: ${JSON.stringify(raw)}`);
  };

  const expect = (ch) => {
    skipWs();
    if (text[i] !== ch) throw new TomlError(`expected ${JSON.stringify(ch)} at offset ${i}`);
    i++;
  };

  const parseBareKey = () => {
    let j = i;
    while (j < n && /[A-Za-z0-9_-]/.test(text[j])) j++;
    if (j === i) throw new TomlError(`expected key at offset ${i}`);
    const key = text.slice(i, j);
    i = j;
    return key;
  };

  const navigate = (parts, array) => {
    let node = root;
    for (let k = 0; k < parts.length; k++) {
      const part = parts[k];
      const last = k === parts.length - 1;
      if (node[part] === undefined) {
        node[part] = last && array ? [] : {};
      }
      if (last && array) {
        if (!Array.isArray(node[part])) throw new TomlError(`[[${parts.join(".")}]] conflicts with a table`);
        const item = {};
        node[part].push(item);
        return item;
      }
      if (typeof node[part] !== "object" || node[part] === null) {
        throw new TomlError(`[${parts.join(".")}] conflicts with a value`);
      }
      node = node[part];
    }
    return node;
  };

  for (;;) {
    skipWs();
    if (i >= n) break;
    if (text[i] === "[") {
      const array = text[i + 1] === "[";
      i += array ? 2 : 1;
      const parts = [parseBareKey()];
      skipWs();
      while (text[i] === ".") {
        i++;
        parts.push(parseBareKey());
        skipWs();
      }
      expect("]");
      if (array) expect("]");
      current = navigate(parts, array);
      continue;
    }
    const key = parseBareKey();
    expect("=");
    current[key] = parseValue();
  }
  return root;
}

// ---------------------------------------------------------------------------
// Schema validation + default_ref / feature-gate resolution.
// ---------------------------------------------------------------------------

const KNOWN_TYPES = new Set(["bool", "int", "float", "string", "enum", "path", "url", "duration-ms", "percent"]);
const LIST_TYPE_RE = /^list<(.+)>$/;

const lookupPath = (obj, path) => {
  let node = obj;
  for (const part of path.split(".")) {
    if (node === null || typeof node !== "object" || !(part in node)) return undefined;
    node = node[part];
  }
  return node;
};

export function buildModel(schemaToml, productToml) {
  const areasKnown = schemaToml.areas?.known;
  if (!Array.isArray(areasKnown) || areasKnown.length === 0) {
    throw new TomlError("schemas/settings.toml: [areas].known must be a non-empty array");
  }
  const settings = schemaToml.setting;
  if (!Array.isArray(settings) || settings.length === 0) {
    throw new TomlError("schemas/settings.toml: no [[setting]] entries");
  }

  const features = productToml.features ?? {};
  const entries = [];
  const seenKeys = new Set();
  const seenMsgids = new Set();
  const seenFlags = new Set();

  for (const raw of settings) {
    const where = raw.key ? `entry "${raw.key}"` : "unnamed [[setting]]";
    for (const field of ["key", "type", "description", "restart_required", "placement", "cli", "feature_gate", "since"]) {
      if (raw[field] === undefined) throw new TomlError(`${where}: mandated field "${field}" missing (doc 07 §5.2)`);
    }
    const type = String(raw.type);
    if (!KNOWN_TYPES.has(type) && !LIST_TYPE_RE.test(type)) {
      throw new TomlError(`${where}: unknown type "${type}"`);
    }
    if (seenKeys.has(raw.key)) throw new TomlError(`D2 violation: duplicate key "${raw.key}"`);
    seenKeys.add(raw.key);
    if (seenMsgids.has(raw.description)) throw new TomlError(`D4 violation: duplicate description msgid "${raw.description}"`);
    seenMsgids.add(raw.description);

    const placement = raw.placement ?? {};
    for (const field of ["screen", "section", "order"]) {
      if (placement[field] === undefined) throw new TomlError(`${where}: placement.${field} missing`);
    }
    if (!areasKnown.includes(placement.screen)) {
      throw new TomlError(`D5 violation: ${where}: unknown screen "${placement.screen}"`);
    }
    const cli = raw.cli ?? {};
    if (!cli.flag) throw new TomlError(`${where}: cli.flag missing`);
    if (seenFlags.has(cli.flag)) throw new TomlError(`D3 violation: duplicate flag "${cli.flag}"`);
    seenFlags.add(cli.flag);

    // D1: default xor default_ref; default_ref must resolve in product.toml.
    const hasDefault = raw.default !== undefined;
    const hasDefaultRef = raw.default_ref !== undefined && raw.default_ref !== "";
    if (hasDefault === hasDefaultRef) {
      throw new TomlError(`D1 violation: ${where}: exactly one of default | default_ref required`);
    }
    let defaultValue;
    let defaultRef = "";
    if (hasDefaultRef) {
      defaultRef = String(raw.default_ref);
      defaultValue = lookupPath(productToml, defaultRef);
      if (defaultValue === undefined) {
        throw new TomlError(`D1 violation: ${where}: default_ref "${defaultRef}" not found in product.toml`);
      }
    } else {
      defaultValue = raw.default;
    }

    // range / enumeration per type (doc 07 §5.2).
    let min;
    let max;
    let enumeration = [];
    if (type === "enum") {
      if (!Array.isArray(raw.enumeration) || raw.enumeration.length === 0) {
        throw new TomlError(`${where}: enum requires a non-empty enumeration`);
      }
      enumeration = raw.enumeration.map((value) => ({
        value: String(value),
        msgid: `settings.value.${String(raw.key).replace(/\./g, "-")}.${String(value).toLowerCase().replace(/[^a-z0-9-]+/g, "-")}`, // w26 Lead: value-segment sanitization (doc 07 §6.1 grammar — colons/dots in values become hyphens; "16:10"→"16-10", "5.1"→"5-1")
      }));
      if (!enumeration.some((o) => o.value === defaultValue)) {
        throw new TomlError(`${where}: default "${defaultValue}" not in enumeration`);
      }
    } else if (type === "int" || type === "float" || type === "duration-ms" || type === "percent") {
      const range = raw.range;
      if (!range || typeof range.min !== "number" || typeof range.max !== "number") {
        throw new TomlError(`${where}: ${type} requires range = { min, max }`);
      }
      min = range.min;
      max = range.max;
      if (typeof defaultValue !== "number" || defaultValue < min || defaultValue > max) {
        throw new TomlError(`${where}: default ${JSON.stringify(defaultValue)} outside range ${min}..${max}`);
      }
    } else if (raw.range !== undefined || raw.enumeration !== undefined) {
      throw new TomlError(`${where}: type "${type}" carries neither range nor enumeration`);
    }

    const featureGate = String(raw.feature_gate ?? "");
    if (featureGate !== "" && typeof features[featureGate] !== "boolean") {
      throw new TomlError(`${where}: feature_gate "${featureGate}" not a [features] boolean in product.toml`);
    }

    entries.push({
      key: String(raw.key),
      type,
      description: String(raw.description),
      defaultValue,
      defaultRef,
      restartRequired: Boolean(raw.restart_required),
      advanced: Boolean(raw.advanced),
      featureGate,
      featureExposed: featureGate === "" ? true : features[featureGate] === true,
      enumeration,
      min,
      max,
      screen: String(placement.screen),
      section: String(placement.section),
      order: Number(placement.order),
      ui: placement.ui !== false,
      cliFlag: String(cli.flag),
      since: String(raw.since),
    });
  }

  // D5: unique order within (screen, section); group ui areas.
  const orderSeen = new Set();
  for (const e of entries) {
    const tag = `${e.screen}/${e.section}/${e.order}`;
    if (orderSeen.has(tag)) throw new TomlError(`D5 violation: duplicate order ${e.order} in ${tag}`);
    orderSeen.add(tag);
  }

  const areas = [];
  for (const screen of areasKnown) {
    const areaEntries = entries.filter((e) => e.screen === screen && e.ui);
    if (areaEntries.length === 0) continue;
    const sections = [];
    for (const e of [...areaEntries].sort((a, b) => a.order - b.order)) {
      let section = sections.find((s) => s.id === e.section);
      if (!section) {
        section = { id: e.section, order: e.order, entries: [] };
        sections.push(section);
      }
      section.entries.push(e);
    }
    sections.sort((a, b) => a.order - b.order);
    areas.push({
      id: screen,
      title: `settings.area.${screen}`,
      sections: sections.map((s) => ({ id: s.id, title: `settings.section.${screen}.${s.id}`, entries: s.entries })),
    });
  }
  return { areas, entries, msgids: [...seenMsgids] };
}

// ---------------------------------------------------------------------------
// Emission (pure string builders — deterministic, sorted where unordered).
// ---------------------------------------------------------------------------

const constName = (key) => `SETTING_${key.replace(/[.-]/g, "_").toUpperCase()}`;

const tsValue = (v) => {
  if (typeof v === "string") return JSON.stringify(v);
  if (typeof v === "boolean" || typeof v === "number") return String(v);
  if (Array.isArray(v)) return `[${v.map((x) => JSON.stringify(String(x))).join(", ")}]`;
  throw new TomlError(`cannot emit value ${JSON.stringify(v)}`);
};

export function emitSchemaTs(model) {
  const lines = [];
  lines.push("// GENERATED FILE — do not edit (card 4.E1, doc 07 §5.3 generator G1).");
  lines.push("// Source of truth: schemas/settings.toml (card 1.B2) with default_ref");
  lines.push("// targets and feature exposure resolved from product.toml.");
  lines.push("// Regenerate: npm run settings:generate    Drift guard: npm run settings:check");
  lines.push("/* eslint-disable */");
  lines.push("");
  lines.push("export type SettingScalarValue = string | number | boolean;");
  lines.push("export type SettingValue = SettingScalarValue | readonly string[];");
  lines.push("export type SettingValueType =");
  lines.push('  | "bool"');
  lines.push('  | "int"');
  lines.push('  | "float"');
  lines.push('  | "string"');
  lines.push('  | "enum"');
  lines.push('  | "list<string>"');
  lines.push('  | "duration-ms"');
  // w26.b02 fix: the validator (KNOWN_TYPES + the Rust model) accepts
  // path/url/percent, but the emitted union omitted them — any such
  // [[setting]] row broke the generated TS (type "path" not assignable).
  lines.push('  | "path"');
  lines.push('  | "url"');
  lines.push('  | "percent";');
  lines.push("");
  lines.push("export interface SettingEnumOption {");
  lines.push("  readonly value: string;");
  lines.push("  readonly msgid: string;");
  lines.push("}");
  lines.push("");
  lines.push("export interface SettingEntry {");
  lines.push("  /** Dotted canonical id, e.g. \"interface.theme\" (doc 07 §5.2). */");
  lines.push("  readonly key: string;");
  lines.push("  readonly type: SettingValueType;");
  lines.push("  /** Label msgid — rendered via useMsg, never literal copy. */");
  lines.push("  readonly description: string;");
  lines.push("  /** default_ref resolved through product.toml (D1). */");
  lines.push("  readonly defaultValue: SettingValue;");
  lines.push("  /** product.toml path this default references; \"\" when literal. */");
  lines.push("  readonly defaultRef: string;");
  lines.push("  readonly restartRequired: boolean;");
  lines.push("  readonly advanced: boolean;");
  lines.push("  /** \"\" = ungated; otherwise a product.toml [features] id. */");
  lines.push("  readonly featureGate: string;");
  lines.push("  /** Resolved from product.toml [features] (doc 05 §6.4 F1/F2). */");
  lines.push("  readonly featureExposed: boolean;");
  lines.push("  readonly enumeration: readonly SettingEnumOption[];");
  lines.push("  readonly min?: number;");
  lines.push("  readonly max?: number;");
  lines.push("  readonly screen: string;");
  lines.push("  readonly section: string;");
  lines.push("  readonly order: number;");
  lines.push("  readonly cliFlag: string;");
  lines.push("  readonly since: string;");
  lines.push("}");
  lines.push("");
  lines.push("export interface SettingsSection {");
  lines.push("  readonly id: string;");
  lines.push("  /** Section heading msgid. */");
  lines.push("  readonly title: string;");
  lines.push("  readonly entries: readonly SettingEntry[];");
  lines.push("}");
  lines.push("");
  lines.push("export interface SettingsArea {");
  lines.push("  readonly id: string;");
  lines.push("  /** Nav label msgid. */");
  lines.push("  readonly title: string;");
  lines.push("  readonly sections: readonly SettingsSection[];");
  lines.push("}");
  lines.push("");
  for (const e of model.entries) {
    lines.push(`export const ${constName(e.key)}: SettingEntry = {`);
    lines.push(`  key: ${JSON.stringify(e.key)},`);
    lines.push(`  type: ${JSON.stringify(e.type)},`);
    lines.push(`  description: ${JSON.stringify(e.description)},`);
    lines.push(`  defaultValue: ${tsValue(e.defaultValue)},`);
    lines.push(`  defaultRef: ${JSON.stringify(e.defaultRef)},`);
    lines.push(`  restartRequired: ${e.restartRequired},`);
    lines.push(`  advanced: ${e.advanced},`);
    lines.push(`  featureGate: ${JSON.stringify(e.featureGate)},`);
    lines.push(`  featureExposed: ${e.featureExposed},`);
    lines.push(
      `  enumeration: [${e.enumeration.map((o) => `{ value: ${JSON.stringify(o.value)}, msgid: ${JSON.stringify(o.msgid)} }`).join(", ")}],`,
    );
    lines.push(`  min: ${e.min === undefined ? "undefined" : String(e.min)},`);
    lines.push(`  max: ${e.max === undefined ? "undefined" : String(e.max)},`);
    lines.push(`  screen: ${JSON.stringify(e.screen)},`);
    lines.push(`  section: ${JSON.stringify(e.section)},`);
    lines.push(`  order: ${e.order},`);
    lines.push(`  cliFlag: ${JSON.stringify(e.cliFlag)},`);
    lines.push(`  since: ${JSON.stringify(e.since)},`);
    lines.push("};");
    lines.push("");
  }
  lines.push("/** Every ui=true entry, in (screen, order) order. */");
  lines.push("export const SETTINGS_ENTRIES: readonly SettingEntry[] = [");
  for (const e of model.entries.filter((x) => x.ui)) lines.push(`  ${constName(e.key)},`);
  lines.push("];");
  lines.push("");
  lines.push("export const SETTINGS_AREAS: readonly SettingsArea[] = [");
  for (const area of model.areas) {
    lines.push("  {");
    lines.push(`    id: ${JSON.stringify(area.id)},`);
    lines.push(`    title: ${JSON.stringify(area.title)},`);
    lines.push("    sections: [");
    for (const s of area.sections) {
      lines.push("      {");
      lines.push(`        id: ${JSON.stringify(s.id)},`);
      lines.push(`        title: ${JSON.stringify(s.title)},`);
      lines.push(`        entries: [${s.entries.map((e) => constName(e.key)).join(", ")}],`);
      lines.push("      },");
    }
    lines.push("    ],");
    lines.push("  },");
  }
  lines.push("];");
  lines.push("");
  lines.push("/** Field-label msgids in schema order — the i18n coverage contract. */");
  lines.push("export const SETTINGS_MSGIDS: readonly string[] = [");
  for (const msgid of model.msgids) lines.push(`  ${JSON.stringify(msgid)},`);
  lines.push("];");
  lines.push("");
  return lines.join("\n");
}

export function emitScreensTs(model) {
  const lines = [];
  lines.push("// GENERATED FILE — do not edit (card 4.E1, doc 07 §5.3 generator G1).");
  lines.push("// One screen component per settings area, statically bound to the");
  lines.push("// schema entries — adding/removing/reordering schema entries changes");
  lines.push("// this file, and `npm run settings:check` fails until it is regenerated.");
  lines.push("/* eslint-disable */");
  lines.push("");
  lines.push('import type { ReactElement } from "react";');
  lines.push('import type { MessageKey } from "../../../i18n/messages";');
  lines.push('import { SettingsSections } from "../SettingsSections";');
  // w26.b02 fix: section/area titles are msgid STRINGS until the Lead
  // registers the new screens' keys post-wave (contract §4.5); the msgid()
  // seam below narrows them so this file type-checks in the window.
  lines.push('import type { GeneratedSection } from "../SettingsSections";');
  lines.push('import type { SettingEntry } from "./schema";');
  for (const area of model.areas) {
    for (const e of area.sections.flatMap((s) => s.entries)) {
      lines.push(`import { ${constName(e.key)} } from "./schema";`);
    }
  }
  lines.push("");
  // w26.b02 seam: new screens' title msgids are carried by per-lane key
  // files (ui/src/i18n/keys/*.ts) until post-wave registration narrows
  // nothing — after registration the literals are MessageKeys and this cast
  // is a no-op.
  lines.push("const msgid = (id: string): MessageKey => id as MessageKey;");
  lines.push("");
  lines.push("/** Section props for <SettingsSections> (see msgid above). */");
  lines.push("const section = (id: string, title: string, entries: readonly SettingEntry[]): GeneratedSection => ({");
  lines.push("  id,");
  lines.push("  title: msgid(title),");
  lines.push("  entries,");
  lines.push("});");
  lines.push("");
  for (const area of model.areas) {
    const fn = `${area.id[0].toUpperCase()}${area.id.slice(1)}Screen`;
    lines.push(`/** Screen "${area.id}" — generated from schemas/settings.toml. */`);
    lines.push(`function ${fn}(): ReactElement {`);
    lines.push("  return (");
    lines.push("    <SettingsSections");
    lines.push(`      area=${JSON.stringify(area.id)}`);
    lines.push("      sections={[");
    for (const s of area.sections) {
      lines.push(`        section(${JSON.stringify(s.id)}, ${JSON.stringify(s.title)}, [`);
      lines.push(`          ${s.entries.map((e) => constName(e.key)).join(", ")},`);
      lines.push("        ]),");
    }
    lines.push("      ]}");
    lines.push("    />");
    lines.push("  );");
    lines.push("}");
    lines.push("");
  }
  lines.push(`export const GENERATED_AREA_IDS = [${model.areas.map((a) => JSON.stringify(a.id)).join(", ")}] as const;`);
  lines.push("");
  lines.push("export type SettingsAreaId = (typeof GENERATED_AREA_IDS)[number];");
  lines.push("");
  // w26.b02 fix: the record keeps its MessageKey contract (consumers rely on
  // it); the VALUES flow through the msgid() seam above because new screens'
  // titles are registered into ui/src/i18n/messages.ts only POST-WAVE
  // (contract §4.5, per-lane keys files).
  lines.push("export const GENERATED_AREA_TITLES: Record<SettingsAreaId, MessageKey> = {");
  for (const area of model.areas) lines.push(`  ${area.id}: msgid(${JSON.stringify(area.title)}),`);
  lines.push("};");
  lines.push("");
  lines.push("export const GENERATED_SCREENS: Record<SettingsAreaId, () => ReactElement> = {");
  for (const area of model.areas) {
    const fn = `${area.id[0].toUpperCase()}${area.id.slice(1)}Screen`;
    lines.push(`  ${area.id}: ${fn},`);
  }
  lines.push("};");
  lines.push("");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// CLI.
// ---------------------------------------------------------------------------

async function main() {
  const args = process.argv.slice(2);
  const check = args.includes("--check");
  const inputIdx = args.indexOf("--input");
  const outDirIdx = args.indexOf("--out-dir");
  const inputPath = inputIdx >= 0 ? resolve(args[inputIdx + 1]) : defaultInput;
  const outDir = outDirIdx >= 0 ? resolve(args[outDirIdx + 1]) : defaultOutDir;

  const schemaToml = parseToml(await readFile(inputPath, "utf8"));
  const productToml = parseToml(await readFile(productTomlPath, "utf8"));
  const model = buildModel(schemaToml, productToml);

  const outputs = [
    { path: resolve(outDir, "schema.ts"), content: emitSchemaTs(model) },
    { path: resolve(outDir, "Screens.tsx"), content: emitScreensTs(model) },
  ];

  if (check) {
    let drifted = false;
    for (const out of outputs) {
      let committed;
      try {
        committed = await readFile(out.path, "utf8");
      } catch {
        committed = null;
      }
      if (committed !== out.content) {
        drifted = true;
        console.error(
          `gen-settings-screens --check: ${out.path} is stale vs ${inputPath} — run \`npm run settings:generate\``,
        );
      }
    }
    if (drifted) process.exit(1);
    const uiCount = model.entries.filter((e) => e.ui).length;
    console.log(
      `gen-settings-screens --check: OK (${model.areas.length} ui areas, ${uiCount} ui entries, ${model.entries.length} total)`,
    );
    return;
  }

  await mkdir(outDir, { recursive: true });
  for (const out of outputs) await writeFile(out.path, out.content, "utf8");
  const uiCount = model.entries.filter((e) => e.ui).length;
  console.log(
    `gen-settings-screens: wrote ${outputs.length} files (${model.areas.length} ui areas, ${uiCount} ui entries, ${model.entries.length} total)`,
  );
}

const isDirectRun = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isDirectRun) {
  main().catch((err) => {
    console.error(`gen-settings-screens: ${err instanceof Error ? err.message : String(err)}`);
    process.exit(1);
  });
}
