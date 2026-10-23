// Card 4.E2 drift guard: the committed hotkeys data must match
// keymaps/default.toml (regenerate: `npm run hotkeys:generate`).

import { describe, expect, it } from "vitest";
import { HOTKEY_BINDINGS, HOTKEY_PLATFORM_OVERRIDES } from "./generated/bindings";

interface NodeChildProcess {
  execFileSync(cmd: string, args: string[], opts: { cwd: string; stdio: "pipe" }): unknown;
}
interface NodeProcess {
  cwd(): string;
  execPath: string;
}

async function runCheck(): Promise<number> {
  // @ts-expect-error node builtin without @types/node (zero-dep rule)
  const cp = (await import("node:child_process")) as NodeChildProcess;
  const proc = (globalThis as unknown as { process: NodeProcess }).process;
  try {
    cp.execFileSync(proc.execPath, ["scripts/gen-hotkeys-data.mjs", "--check"], {
      cwd: proc.cwd(),
      stdio: "pipe",
    });
    return 0;
  } catch (err) {
    return (err as { status?: number }).status ?? 1;
  }
}

describe("hotkeys data drift (4.E2)", () => {
  it("passes hotkeys:check against keymaps/default.toml", async () => {
    // vitest runs with cwd = ui/ (package root); the script resolves paths
    // from its own module url, so cwd only matters for the exec itself.
    expect(await runCheck()).toBe(0);
  });

  it("carries real keymap shape: sections, modes, command ids", () => {
    expect(HOTKEY_BINDINGS.length).toBeGreaterThan(100);
    const sections = new Set(HOTKEY_BINDINGS.map((b) => b.section));
    expect([...sections]).toEqual(
      expect.arrayContaining(["media", "navigation", "audio", "video", "subtitles", "playlist", "window", "advanced"]),
    );
    expect(HOTKEY_BINDINGS.some((b) => b.command === "playback.toggle-play-pause")).toBe(true);
    expect(HOTKEY_BINDINGS.some((b) => b.mode === "always")).toBe(true);
    expect(HOTKEY_PLATFORM_OVERRIDES.some((o) => o.platform === "macos")).toBe(true);
  });
});
