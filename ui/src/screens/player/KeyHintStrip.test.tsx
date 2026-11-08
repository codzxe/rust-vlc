// Solstice v2 — keycap hint strip tests (KeyHintStrip.tsx).
//
// Pins the audit fixes: arrow keys render as GLYPHS (← / → — the audit saw
// them fall back to dash-like tofu at 6px; the DOM contract here is the
// spelled glyph from KEY_GLYPHS), the strip renders its real keymap data
// (Space / ← / → / F / M from keymaps/default.toml via the generated
// bindings), and it disappears the moment something is playing (hints would
// only fight the content).

import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandTransport } from "../../client/transport";
import type { Event } from "../../client/transport";
import { PlayerControlsProvider } from "./context";
import { PlayerKeyHintStrip } from "./KeyHintStrip";

/** Scripted transport: records nothing, just emits engine events on demand. */
class ScriptedTransport implements CommandTransport {
  private readonly listeners = new Set<(event: Event) => void>();

  send(): Promise<never> {
    return Promise.reject(new Error("not used in this test"));
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }
}

function renderStrip(
  hasMedia: boolean,
  transport: CommandTransport = new ScriptedTransport(),
): ReturnType<typeof render> {
  return render(
    <ClientProvider transport={transport}>
      <PlayerControlsProvider>
        <PlayerKeyHintStrip hasMedia={hasMedia} />
      </PlayerControlsProvider>
    </ClientProvider>,
  );
}

describe("PlayerKeyHintStrip — keymap data + glyph rendering (brief §2.9)", () => {
  it("renders the strip's bindings as keycaps with arrow GLYPHS for seek keys", () => {
    const { container } = renderStrip(false);
    const strip = screen.getByRole("group", { name: "Player keys" });
    expect(strip).toBeInTheDocument();
    // Real keymap data (keymaps/default.toml via the generated bindings):
    // Space, Left, Right, F, M — seek keys render as glyphs, never raw words.
    const caps = Array.from(strip.querySelectorAll(".solstice-key")).map(
      (cap) => cap.textContent,
    );
    expect(caps).toContain("Space");
    expect(caps).toContain("←");
    expect(caps).toContain("→");
    expect(caps).toContain("F");
    expect(caps).toContain("M");
    // One label per keycap, from the same binding data.
    expect(strip.querySelectorAll(".lumen-player__keyhint")).toHaveLength(caps.length);
    expect(container.querySelector(".lumen-player__keyhints")).not.toBeNull();
  });

  it("hides while media is open and playing — hints never fight the content", () => {
    const { container } = renderStrip(true);
    // No engine word yet (idle default) — with media open the strip hides
    // unless the engine reported `paused`.
    expect(screen.queryByRole("group", { name: "Player keys" })).toBeNull();
    expect(container.querySelector(".lumen-player__keyhints")).toBeNull();
  });

  it("reappears for an engine-reported pause; first-run stays visible", () => {
    const transport = new ScriptedTransport();
    const { rerender } = renderStrip(true, transport);
    act(() => {
      transport.emit({ kind: "session.state-changed", from: "idle", to: "playing", seq: 1 });
    });
    expect(screen.queryByRole("group", { name: "Player keys" })).toBeNull();
    act(() => {
      transport.emit({ kind: "session.state-changed", from: "playing", to: "paused", seq: 2 });
    });
    expect(screen.getByRole("group", { name: "Player keys" })).toBeInTheDocument();
    // No media (first-run): visible regardless.
    rerender(
      <ClientProvider transport={transport}>
        <PlayerControlsProvider>
          <PlayerKeyHintStrip hasMedia={false} />
        </PlayerControlsProvider>
      </ClientProvider>,
    );
    expect(screen.getByRole("group", { name: "Player keys" })).toBeInTheDocument();
  });

  it("keycaps render as .solstice-key--sm chips inside the chrome-part strip", () => {
    renderStrip(false);
    const strip = screen.getByRole("group", { name: "Player keys" });
    // data-chrome-part makes the strip fade with the chrome (shell contract).
    expect(strip).toHaveAttribute("data-chrome-part", "");
    expect(strip.querySelectorAll(".solstice-key--sm").length).toBeGreaterThan(0);
  });
});
