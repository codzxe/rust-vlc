// Card 4.E1 acceptance proof: the generated settings screens DETECT SCHEMA
// DRIFT. The generator is re-run in --check mode (byte-compare against the
// committed generated/ files):
//   1. pristine schemas/settings.toml  → check passes
//   2. a MUTATED COPY of the schema     → check FAILS (exit 1)
// The mutation happens on a temp copy — the real schema and the committed
// generated files are never touched, so nothing needs restoring.
//
// Node builtins are dynamic-imported with local type shapes because the ui
// package carries no @types/node (card rule: zero new deps); vitest runs on
// node, so the builtins exist at runtime.

import { describe, expect, it } from "vitest";
import { isMessageKey, useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b02";
import { keys as seedKeys } from "../../i18n/keys/w26.f01";
import { keys as gfixKeys } from "../../i18n/keys/w26.gfixB";
import { SETTINGS_AREAS, SETTINGS_ENTRIES, SETTINGS_MSGIDS } from "./generated/schema";

interface NodeFs {
  readFileSync(path: string, enc: "utf8"): string;
  writeFileSync(path: string, data: string, enc: "utf8"): void;
  mkdtempSync(prefix: string): string;
}
interface NodeOs {
  tmpdir(): string;
}
interface NodeChildProcess {
  execFileSync(cmd: string, args: string[], opts: { cwd: string; stdio: "pipe" }): unknown;
}
interface NodeProcess {
  cwd(): string;
  execPath: string;
}

async function loadNode() {
  // @types/node is now a devDependency (F4/G1 tooling required it), so the
  // old zero-dep @ts-expect-error directives are unused and were removed.
  const fs = (await import("node:fs")) as NodeFs;
  const os = (await import("node:os")) as NodeOs;
  const cp = (await import("node:child_process")) as NodeChildProcess;
  const proc = (globalThis as unknown as { process: NodeProcess }).process;
  return { fs, os, cp, proc };
}

// Vite statically rewrites `new URL(literal, import.meta.url)` into an asset
// import — string slicing keeps the real filesystem path.
const dirUrl = import.meta.url.slice(0, import.meta.url.lastIndexOf("/") + 1);
const toPath = (url: string): string => url.replace(/^file:\/\//, "");
const uiRootPath = toPath(`${dirUrl}../../../`); // …/ui/
const schemaPath = toPath(`${dirUrl}../../../../schemas/settings.toml`); // repo root
const scriptPath = toPath(`${dirUrl}../../../scripts/gen-settings-screens.mjs`);

const runGenerator = async (args: string[]): Promise<number> => {
  const { cp, proc } = await loadNode();
  try {
    cp.execFileSync(proc.execPath, [scriptPath, ...args], {
      cwd: uiRootPath,
      stdio: "pipe",
    });
    return 0;
  } catch (err) {
    const e = err as { status?: number };
    return e.status ?? 1;
  }
};

const mutateSchemaCopy = async (mutate: (original: string) => string): Promise<void> => {
  const { fs, os } = await loadNode();
  const original = fs.readFileSync(schemaPath, "utf8");
  const mutated = mutate(original);
  expect(mutated).not.toBe(original); // the mutation must apply
  const dir = fs.mkdtempSync(`${os.tmpdir()}/lumen-settings-drift-`);
  fs.writeFileSync(`${dir}/settings.toml`, mutated, "utf8");
  mutatedInputPath = `${dir}/settings.toml`;
};

let mutatedInputPath = "";

describe("generated settings screens detect schema drift (4.E1 AC)", () => {
  it("passes settings:check against the committed generated files", async () => {
    expect(await runGenerator(["--check"])).toBe(0);
  });

  it("FAILS settings:check when a schema default changes (mutated copy)", async () => {
    await mutateSchemaCopy((s) => s.replace('default = "info"', 'default = "warn"'));
    expect(await runGenerator(["--check", "--input", mutatedInputPath])).toBe(1);
  });

  it("FAILS settings:check when a field's restart flag changes (mutated copy)", async () => {
    await mutateSchemaCopy((s) => s.replace("restart_required = false", "restart_required = true"));
    expect(await runGenerator(["--check", "--input", mutatedInputPath])).toBe(1);
  });

  it("FAILS settings:check when a [[setting]] entry is removed (mutated copy)", async () => {
    await mutateSchemaCopy((s) =>
      s.replace(/\[\[setting\]\]\nkey = "playback\.start-paused"[\s\S]*?since = "0\.1\.0"\n/, ""),
    );
    expect(await runGenerator(["--check", "--input", mutatedInputPath])).toBe(1);
  });
});

describe("generated schema data (committed snapshot sanity)", () => {
  it("covers the 10 ui areas with the 102 ui entries of schemas/settings.toml", () => {
    // w26.b02 seed: the three NEW Figma areas (controls, recording,
    // extensions) gained [[setting]] rows; video/audio/subtitles/library/
    // network stay hand-registered (areas/index.tsx) — content areas, not
    // schema screens.
    // w26.f01 seed extension (settings content gaps): video + audio gain
    // their frame sections (e06 F3 / e07 F2-F4) — 64 new rows across global /
    // privacy / playback / video / audio / performance. The video/audio
    // generated screens exist for the area lanes to compose (registry folds
    // keep the hand screens until then).
    expect(SETTINGS_AREAS.map((a) => a.id)).toEqual([
      "global",
      "interface",
      "privacy",
      "controls",
      "playback",
      "video",
      "audio",
      "performance",
      "recording",
      "extensions",
    ]);
    expect(SETTINGS_ENTRIES).toHaveLength(102); // w26.gfix-B re-seed: 97 → 102
  });

  it("has globally unique keys with in-range defaults (D2 + range rules)", () => {
    const keys = new Set(SETTINGS_ENTRIES.map((e) => e.key));
    expect(keys.size).toBe(SETTINGS_ENTRIES.length);
    for (const entry of SETTINGS_ENTRIES) {
      if (entry.min !== undefined && entry.max !== undefined) {
        expect(entry.defaultValue).toBeGreaterThanOrEqual(entry.min);
        expect(entry.defaultValue).toBeLessThanOrEqual(entry.max);
      }
      if (entry.type === "enum") {
        expect(entry.enumeration.map((o) => o.value)).toContain(String(entry.defaultValue));
      }
    }
  });

  it("resolves default_ref through product.toml (D1)", () => {
    const theme = SETTINGS_ENTRIES.find((e) => e.key === "interface.theme");
    expect(theme?.defaultValue).toBe("dark"); // product.toml defaults.theme
    expect(theme?.defaultRef).toBe("defaults.theme");
    // w26.gfix-B: interface.window-* went ui=false with the re-seed (the
    // frame shows no window-metric rows) — a live default_ref row carries D1.
    const geometry = SETTINGS_ENTRIES.find((e) => e.key === "interface.remember-window-geometry");
    expect(geometry?.defaultValue).toBe(true); // product.toml window.remember_geometry
    expect(geometry?.defaultRef).toBe("window.remember_geometry");
  });

  it("marks feature-gated entries unexposed per product.toml [features] (F1/F2)", () => {
    const hw = SETTINGS_ENTRIES.find((e) => e.key === "playback.hw-decode-policy");
    expect(hw?.featureGate).toBe("hw-decode");
    expect(hw?.featureExposed).toBe(false); // features.hw-decode = false until Phase 7
  });

  it("carries schema-flagged restart-required fields", () => {
    const restartKeys = SETTINGS_ENTRIES.filter((e) => e.restartRequired).map((e) => e.key);
    // w26.gfix-B: the ui=false window metrics left the registry; live
    // restart-required rows carry the assertion.
    expect(restartKeys).toContain("global.language");
    expect(restartKeys).toContain("interface.remember-window-geometry");
    expect(restartKeys).toContain("playback.menu-language");
    expect(restartKeys).not.toContain("interface.theme");
  });

  it("resolves every ui schema label msgid in the catalog or the lane keys (i18n coverage)", () => {
    const m = useMsg();
    // w26.b02 + w26.f01: the seed introduces msgids (controls/recording/
    // extensions rows + the w26.f01 content-gap rows across six areas, their
    // sections/areas and enum options) that are registered into messages.ts
    // only POST-WAVE (contract §4.5; ui/src/i18n/keys/w26.b02.ts and
    // w26.f01.ts carry them until then). Coverage = registered catalog entry
    // OR lane-key entry.
    const laneCatalog: Readonly<Record<string, string>> = {
      ...laneKeys,
      ...seedKeys,
      ...gfixKeys, // w26.gfix-B: the re-seed's relabels/moves pre-registration
    };
    const covered = (msgid: string): boolean =>
      isMessageKey(msgid) ? m(msgid as MessageKey).length > 0 : msgid in laneCatalog;
    // SETTINGS_MSGIDS carries all 108 entries incl. ui=false engine fields
    // (CLI --help territory, dev-facing English); coverage must hold for
    // every RENDERED label: the 97 ui entries + their enum options + the
    // section titles.
    expect(SETTINGS_MSGIDS.length).toBeGreaterThanOrEqual(SETTINGS_ENTRIES.length);
    for (const entry of SETTINGS_ENTRIES) {
      expect(covered(entry.description), `missing catalog/lane entry for ${entry.description}`).toBe(true);
      for (const option of entry.enumeration) {
        expect(covered(option.msgid), `missing catalog/lane entry for ${option.msgid}`).toBe(true);
      }
    }
    for (const area of SETTINGS_AREAS) {
      for (const section of area.sections) {
        expect(covered(section.title), `missing catalog/lane entry for ${section.title}`).toBe(true);
      }
    }
  });
});
