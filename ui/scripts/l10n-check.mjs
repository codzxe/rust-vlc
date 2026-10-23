#!/usr/bin/env node
// l10n-check.mjs — en-US catalog integrity gate (card 4.H3, doc 07 §6).
//
// Zero-dependency audit of the UI message catalog (ui/src/i18n/messages.ts):
//
//   1. missing   — every MessageKey union member has an en-US string
//   2. duplicate — no repeated keys in the union or in the catalog object
//   3. orphan    — no catalog string without a union key
//   4. grammar   — keys follow doc 07 §6.1: 2–5 dotted kebab-case segments
//                  ([a-z0-9-]+ per segment)
//   5. placeholders — every {...} token in a string is a well-formed
//                  doc 07 §6.2 message-format v1 placeholder ({name},
//                  {name, number, percent}, {name, select|plural, …});
//                  {{ }} escapes are honored. The per-key placeholder NAME
//                  set is reported so future catalogs can be diffed against
//                  it (the Rust-side crates/localization audit owns the
//                  cross-catalog comparison — doc 07 §6.4).
//   6. allowlist — ui/i18n-allowlist.json is a valid {text, reason, owner}
//                  array with non-empty reasons (doc 07 §6.5 exemptions are
//                  reason-carrying by rule).
//
// Wired as `npm run l10n:check`; CI proposal in .github/workflows/ui.yml.
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const messagesPath = resolve(here, "../src/i18n/messages.ts");
const allowlistPath = resolve(here, "../i18n-allowlist.json");

const errors = [];
const err = (rule, msg) => errors.push(`[${rule}] ${msg}`);

// --- parse messages.ts ----------------------------------------------------
const src = readFileSync(messagesPath, "utf8");

// Union arm: `  | "key"` between `export type MessageKey =` and the catalog
// object (the closing `;` cannot be used as the end marker — section
// comments inside the union may contain semicolons).
const unionStart = src.indexOf("export type MessageKey =");
const catalogAnchor = src.indexOf("const messages:");
if (unionStart === -1) err("parse", "MessageKey union not found in messages.ts");
const unionSrc = src.slice(unionStart, catalogAnchor === -1 ? undefined : catalogAnchor);
const unionKeys = [...unionSrc.matchAll(/\|\s*"([^"]+)"/g)].map((m) => m[1]);

// Catalog entries: `"key": "value"` pairs inside
// `const messages: Readonly<Record<MessageKey, string>> = { … };`.
const catalogHeader =
  "const messages: Readonly<Record<MessageKey, string>> = {";
const catalogStart = src.indexOf(catalogHeader);
if (catalogStart === -1) err("parse", "messages catalog object not found");
const catalogEnd = src.indexOf("\n};", catalogStart);
const catalogSrc = src.slice(
  catalogStart + catalogHeader.length,
  catalogEnd === -1 ? undefined : catalogEnd,
);
const stringLiteral = `"(?:[^"\\\\]|\\\\.)*"`;
const entryRe = new RegExp(`^\\s*${stringLiteral}\\s*:`, "gm");
const valueRe = new RegExp(`\\s*(${stringLiteral})`, "y");
const catalogEntries = [];
for (const m of catalogSrc.matchAll(entryRe)) {
  const key = JSON.parse(m[0].trim().replace(/:$/, ""));
  valueRe.lastIndex = m.index + m[0].length;
  const v = valueRe.exec(catalogSrc);
  if (v === null) {
    err("parse", `catalog key "${key}" has no string-literal value`);
    catalogEntries.push([key, null]);
    continue;
  }
  catalogEntries.push([key, JSON.parse(v[1])]);
}

// --- 1/2/3: coverage -------------------------------------------------------
const unionSet = new Set(unionKeys);
const catalogMap = new Map(catalogEntries);

const unionDupes = unionKeys.filter((k, i) => unionKeys.indexOf(k) !== i);
for (const k of unionDupes) err("duplicate", `union key "${k}" declared twice`);
const catalogCounts = new Map();
for (const [k] of catalogEntries) catalogCounts.set(k, (catalogCounts.get(k) ?? 0) + 1);
const catalogDupes = [...catalogCounts.entries()].filter(([, n]) => n > 1).map(([k]) => k);
for (const k of catalogDupes) err("duplicate", `catalog key "${k}" defined twice`);

const missing = unionKeys.filter((k) => !catalogMap.has(k));
for (const k of missing) err("missing", `MessageKey "${k}" has no en-US string`);
const orphans = [...catalogMap.keys()].filter((k) => !unionSet.has(k));
for (const k of orphans) err("orphan", `en-US string "${k}" is not in the MessageKey union`);

// --- 4: key grammar (doc 07 §6.1) -----------------------------------------
const keyGrammar = /^[a-z0-9-]+(\.[a-z0-9-]+){1,4}$/;
for (const k of new Set([...unionKeys, ...catalogMap.keys()]))
  if (!keyGrammar.test(k)) err("grammar", `key "${k}" violates doc 07 §6.1 (2–5 dotted [a-z0-9-]+ segments)`);

// --- 5: placeholder grammar (doc 07 §6.2) ----------------------------------
// A valid token is {name} or {name, argstyle…} where name is snake_case.
// `{{`/`}}` are literal-brace escapes and are stripped before tokenizing.
const placeholderGrammar = /^\{[A-Za-z_][A-Za-z0-9_]*(\s*,\s*[a-z][^{}]*)?\}$/;
let placeholderCount = 0;
for (const [k, value] of catalogEntries) {
  if (typeof value !== "string" || value.length === 0) {
    err("empty", `key "${k}" has an empty en-US string`);
    continue;
  }
  // Mask {{ }} escapes (PUA sentinels — not control chars), then every
  // remaining {...} must be a well-formed placeholder; leftovers = strays.
  const masked = value.replaceAll("{{", "\uE002").replaceAll("}}", "\uE003");
  for (const token of masked.matchAll(/\{[^{}]*\}/g)) {
    placeholderCount += 1;
    if (!placeholderGrammar.test(token[0]))
      err("placeholder", `key "${k}": malformed placeholder ${token[0]} (doc 07 §6.2 grammar)`);
  }
  if (/[{}]/.test(masked.replace(/\{[^{}]*\}/g, "")))
    err("placeholder", `key "${k}": stray unbalanced brace`);
}

// --- 6: allowlist shape -----------------------------------------------------
let allowlistCount = 0;
try {
  const allowlist = JSON.parse(readFileSync(allowlistPath, "utf8"));
  if (!Array.isArray(allowlist)) throw new Error("not an array");
  allowlistCount = allowlist.length;
  for (const e of allowlist) {
    if (typeof e.text !== "string" || e.text.length === 0)
      err("allowlist", "entry without a `text` string");
    if (typeof e.reason !== "string" || e.reason.trim().length === 0)
      err("allowlist", `entry "${e.text}" has no reason (doc 07 §6.5 exemptions carry reasons)`);
    if (typeof e.owner !== "string" || e.owner.trim().length === 0)
      err("allowlist", `entry "${e.text}" has no owner`);
  }
} catch (e) {
  err("allowlist", `i18n-allowlist.json unreadable/invalid: ${e.message}`);
}

// --- report -----------------------------------------------------------------
const summary =
  `l10n:check — ${unionKeys.length} MessageKeys, ${catalogEntries.length} en-US strings: ` +
  `missing ${missing.length}, duplicates ${unionDupes.length + catalogDupes.length}, orphans ${orphans.length}, ` +
  `key-grammar ok, placeholders ${placeholderCount} ok, allowlist ${allowlistCount} entries`;
if (errors.length > 0) {
  console.error(`FAIL ${summary}`);
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
console.log(`PASS ${summary}`);
