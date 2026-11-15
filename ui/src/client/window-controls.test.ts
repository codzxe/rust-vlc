// window-controls tests (w28 fix 2) — the OS window seam: label from
// internals.metadata, command shapes, and the null probe outside the
// desktop shell. Same Fake-internals posture as tauri-transport.test.ts:
// no webview is ever needed.
import { afterEach, describe, expect, it } from "vitest";
import { detectOsWindowBackend } from "./window-controls";

/** A scriptable __TAURI_INTERNALS__ surface (the injected shape). */
function installInternals(options: {
  label?: string;
  withMetadata?: boolean;
  maximized?: boolean;
}): {
  invokes: Array<{ command: string; payload?: unknown }>;
  fireResize: () => void;
} {
  const invokes: Array<{ command: string; payload?: unknown }> = [];
  let resizeHandler: (() => void) | null = null;
  const internals = {
    invoke: (command: string, payload?: unknown): Promise<unknown> => {
      invokes.push({ command, payload });
      if (command === "plugin:window|is_maximized") {
        return Promise.resolve(options.maximized === true);
      }
      if (command === "plugin:event|listen") {
        return Promise.resolve(7);
      }
      return Promise.resolve(undefined);
    },
    transformCallback: (callback: (response: unknown) => void): number => {
      // onResized registers through this; capture so tests can fire it.
      resizeHandler = () => callback({});
      return 3;
    },
    ...(options.withMetadata === false
      ? {}
      : { metadata: { currentWindow: { label: options.label ?? "main" } } }),
  };
  (globalThis as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;
  return {
    invokes,
    fireResize: () => resizeHandler?.(),
  };
}

afterEach(() => {
  delete (globalThis as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe("detectOsWindowBackend", () => {
  it("returns null outside the desktop shell (no injected internals)", () => {
    expect(detectOsWindowBackend()).toBeNull();
  });

  it("exposes the four chrome commands with the runtime-stamped label", async () => {
    const { invokes } = installInternals({ label: "main" });
    const backend = detectOsWindowBackend();
    expect(backend).not.toBeNull();
    if (backend === null) return; // TS narrowing — the expect above pins it
    expect(backend.label).toBe("main");

    await backend.minimize();
    await backend.toggleMaximize();
    await backend.close();
    const maximized = await backend.isMaximized();

    expect(maximized).toBe(false);
    expect(invokes).toEqual([
      { command: "plugin:window|minimize", payload: { label: "main" } },
      { command: "plugin:window|toggle_maximize", payload: { label: "main" } },
      { command: "plugin:window|close", payload: { label: "main" } },
      { command: "plugin:window|is_maximized", payload: { label: "main" } },
    ]);
  });

  it("is_maximized surfaces the window's real state (true case)", async () => {
    installInternals({ maximized: true });
    const backend = detectOsWindowBackend();
    await expect(backend?.isMaximized()).resolves.toBe(true);
  });

  it("falls back to the conventional label only when metadata is absent", async () => {
    const { invokes } = installInternals({ withMetadata: false });
    const backend = detectOsWindowBackend();
    expect(backend?.label).toBe("main");
    await backend?.minimize();
    expect(invokes[0]?.payload).toEqual({ label: "main" });
  });

  it("a second window's label drives THAT window — never a hardcoded one", async () => {
    const { invokes } = installInternals({ label: "mini" });
    const backend = detectOsWindowBackend();
    await backend?.close();
    expect(invokes[0]?.payload).toEqual({ label: "mini" });
  });

  it("onResized listens on tauri://resize and unlistens through the returned stop", async () => {
    const { invokes, fireResize } = installInternals({});
    const backend = detectOsWindowBackend();
    let fired = 0;
    const stop = await backend?.onResized(() => {
      fired += 1;
    });
    expect(invokes[0]?.command).toBe("plugin:event|listen");
    expect(invokes[0]?.payload).toMatchObject({ event: "tauri://resize" });

    fireResize();
    fireResize();
    expect(fired).toBe(2);

    await stop?.();
    const unlisten = invokes[invokes.length - 1];
    expect(unlisten?.command).toBe("plugin:event|unlisten");
    expect(unlisten?.payload).toMatchObject({ event: "tauri://resize", eventId: 7 });
  });
});
