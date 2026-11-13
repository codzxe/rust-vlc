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
//   6. allowlist — ui/i18n-allowlist.json is a valid {text, files, reason,
//                  owner} array with non-empty reasons (doc 07 §6.5
//                  exemptions are reason-carrying by rule) and non-empty
//                  file scoping — every entry names the files it excuses
//                  (ui/-relative paths); v3 (w26.k20, j20 P3-14) — v2
//                  text-only entries exempted the text repo-wide.
//                  6b (w26.k18, j18 F9): every v3 entry's text must still
//                  occur in at least one of ITS OWN scoped files — an
//                  exemption whose text vanished from its files is stale.
//   7. dead keys — every catalog key must be referenced by some scanned
//                  source file (exact quoted literal, or a dynamic
//                  `prefix.${…}` template family), OR be grandfathered in
//                  ui/i18n-dead-keys.json with a reason (j18 F4, w26.k18).
//                  Two-way gate: a NEW dead key fails (future rot), and a
//                  grandfather entry that gained a reference or left the
//                  catalog fails too (the list must shrink, not grow moss).
//
// Scan universe for 6b/7: ui/src, ui/e2e, ui/scripts, ui/tools — every
// .ts/.tsx/.mjs/.js/.cjs file EXCEPT ui/src/i18n/messages.ts (the catalog)
// and ui/src/i18n/keys/** (the per-lane registration files); both are
// definition sites, not usage.
//
// Wired as `npm run l10n:check`; CI proposal in .github/workflows/ui.yml.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { resolve, dirname, join } from "node:path";
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
let allowlist = [];
try {
  allowlist = JSON.parse(readFileSync(allowlistPath, "utf8"));
  if (!Array.isArray(allowlist)) throw new Error("not an array");
  allowlistCount = allowlist.length;
  for (const e of allowlist) {
    if (typeof e.text !== "string" || e.text.length === 0)
      err("allowlist", "entry without a `text` string");
    if (typeof e.reason !== "string" || e.reason.trim().length === 0)
      err("allowlist", `entry "${e.text}" has no reason (doc 07 §6.5 exemptions carry reasons)`);
    if (typeof e.owner !== "string" || e.owner.trim().length === 0)
      err("allowlist", `entry "${e.text}" has no owner`);
    // v3 file scoping (w26.k20, j20 P3-14): an entry without files would
    // exempt its text in EVERY file — the rule (i18n-no-literals.js) throws
    // on the same violation at lint time; this gate catches it earlier
    // with file/line context.
    if (
      !Array.isArray(e.files) ||
      e.files.length === 0 ||
      e.files.some((f) => typeof f !== "string" || f.trim().length === 0)
    )
      err("allowlist", `entry "${e.text}" must carry a non-empty files: string[] (ui/-relative paths)`);
  }
} catch (e) {
  err("allowlist", `i18n-allowlist.json unreadable/invalid: ${e.message}`);
}

// --- shared source scan (rules 6b + 7, w26.k18) ------------------------------
const uiRoot = resolve(here, "..");
const SCAN_ROOTS = ["src", "e2e", "scripts", "tools"];
const SKIP_DIRS = new Set(["node_modules", "dist"]);
const SOURCE_EXT = /\.(ts|tsx|mjs|js|cjs)$/;
const DEFINITION_SITES = [resolve(uiRoot, "src/i18n/messages.ts")];
const KEYS_DIR = resolve(uiRoot, "src/i18n/keys") + "/";
const isDefinitionSite = (p) => DEFINITION_SITES.includes(p) || p.startsWith(KEYS_DIR);
const sourceTextsByPath = new Map();
const walkSources = (dir) => {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return; // root absent (e.g. no tools/) — nothing to scan
  }
  for (const entry of entries) {
    const p = join(dir, entry);
    let st;
    try {
      st = statSync(p);
    } catch {
      continue;
    }
    if (st.isDirectory()) {
      if (!SKIP_DIRS.has(entry)) walkSources(p);
    } else if (SOURCE_EXT.test(p) && !isDefinitionSite(p)) {
      sourceTextsByPath.set(p, readFileSync(p, "utf8"));
    }
  }
};
for (const root of SCAN_ROOTS) walkSources(resolve(uiRoot, root));

// --- 6b: allowlist entries still present in their scoped files (j18 F9) -----
// Raw-text presence inside the entry's OWN files list — the v3 `files`
// field says where the text lives; an entry whose text occurs in NONE of
// them is stale (the copy moved or died). Punctuation entries are
// structurally omnipresent, which is fine: they can only fail when their
// scoped files themselves vanish.
let allowlistStale = 0;
for (const e of allowlist) {
  if (typeof e.text !== "string" || !Array.isArray(e.files)) continue; // shape errors above
  const scoped = e.files.map((f) => resolve(uiRoot, f));
  const present = scoped.some((p) => {
    const t = sourceTextsByPath.get(p);
    return t !== undefined && t.includes(e.text);
  });
  if (!present) {
    allowlistStale += 1;
    err(
      "allowlist",
      `entry "${e.text}" does not occur in any of its scoped files [${e.files.join(", ")}] — the exemption is stale, drop or re-scope it`,
    );
  }
}

// --- 7: dead keys (j18 F4, w26.k18) ------------------------------------------
// A key is USED when some scanned file carries it as an exact quoted
// literal, or when it falls under a dynamic template family `prefix.${…}`
// (dotted prefixes only — conservative: the raw-text family match may
// over-mark a family as used, never under-mark).
const dynPrefixes = new Set();
for (const t of sourceTextsByPath.values())
  for (const m of t.matchAll(/([a-z0-9-]+(?:\.[a-z0-9-]+)+)\.\$\{/g)) dynPrefixes.add(m[1]);
const keyUsed = (k) => {
  if ([...dynPrefixes].some((p) => k.startsWith(p + "."))) return true;
  for (const t of sourceTextsByPath.values()) if (t.includes(`"${k}"`)) return true;
  return false;
};
const deadKeys = unionKeys.filter((k) => !keyUsed(k));

const deadKeysPath = resolve(here, "../i18n-dead-keys.json");
const grandfathered = new Map();
try {
  const list = JSON.parse(readFileSync(deadKeysPath, "utf8"));
  if (!Array.isArray(list)) throw new Error("not an array");
  for (const e of list) {
    if (e !== null && typeof e === "object" && !("key" in e)) continue; // $comment doc entry
    if (typeof e.key !== "string" || e.key.length === 0) {
      err("dead-keys", "i18n-dead-keys.json entry without a `key` string");
      continue;
    }
    if (typeof e.reason !== "string" || e.reason.trim().length === 0) {
      err("dead-keys", `dead-key entry "${e.key}" has no reason`);
      continue;
    }
    if (grandfathered.has(e.key))
      err("dead-keys", `dead-key entry "${e.key}" is listed twice`);
    grandfathered.set(e.key, e);
  }
} catch (e) {
  err("dead-keys", `i18n-dead-keys.json unreadable/invalid: ${e.message}`);
}

const deadSet = new Set(deadKeys);
for (const k of deadKeys) {
  if (!grandfathered.has(k))
    err(
      "dead-keys",
      `MessageKey "${k}" is referenced by NO scanned source file (new dead key — use it, drop it, or grandfather it with a reason in ui/i18n-dead-keys.json)`,
    );
}
// (w26.k20 lint fix, co-edit with the in-flight dead-keys lane: the map's
// value is unused here — iterate keys only, keeps `npm run lint` green.)
for (const k of grandfathered.keys()) {
  if (!catalogMap.has(k))
    err("dead-keys", `grandfather entry "${k}" left the catalog — prune it from i18n-dead-keys.json`);
  else if (!deadSet.has(k))
    err(
      "dead-keys",
      `grandfather entry "${k}" is referenced now — prune it from i18n-dead-keys.json`,
    );
}

// --- report -----------------------------------------------------------------
const summary =
  `l10n:check — ${unionKeys.length} MessageKeys, ${catalogEntries.length} en-US strings: ` +
  `missing ${missing.length}, duplicates ${unionDupes.length + catalogDupes.length}, orphans ${orphans.length}, ` +
  `key-grammar ok, placeholders ${placeholderCount} ok, allowlist ${allowlistCount} entries ` +
  `(${allowlistStale} stale), dead keys ${deadKeys.length}/${unionKeys.length} grandfathered ` +
  `(ui/i18n-dead-keys.json)`;
if (errors.length > 0) {
  console.error(`FAIL ${summary}`);
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
console.log(`PASS ${summary}`);
