// Issue #33 / D-012 — Effects & Tools area tests. Honest-surface rules:
// live controls (volume, mute) hydrate from the engine and send their
// catalog command on change; pending controls (balance, video adjust —
// engine stages exist, catalog commands do not yet) render DISABLED
// with the reason, never hidden, never alive.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import type { CommandRequest } from "../../../client/transport";
import type { CommandResponse } from "../../../client/transport";
import { EffectsArea } from "./effects";
import { makeFixtureTransport } from "./audio-fixtures";

const volumeResult = (volume: number): CommandResponse["result"] => ({
  completed: { kind: "audio-volume", volume },
});

const renderWith = (transport: ReturnType<typeof makeFixtureTransport>) =>
  render(
    <ClientProvider transport={transport}>
      <EffectsArea />
    </ClientProvider>,
  );

describe("EffectsArea — live controls wire to the catalog", () => {
  it("hydrates the volume slider from audio.get-volume", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.get-volume" ? volumeResult(0.8) : { completed: { kind: "unit" } },
    );
    renderWith(transport);
    const slider = await screen.findByRole("slider", { name: "Master volume" });
    await waitFor(() => expect(slider).toBeEnabled());
    expect((slider as HTMLInputElement).value).toBe("0.8");
  });

  it("changing volume dispatches audio.set-volume with the slider value", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.get-volume"
        ? volumeResult(1.0)
        : request.cmd === "audio.set-volume"
          ? volumeResult(0.6)
          : { completed: { kind: "unit" } },
    );
    renderWith(transport);
    const slider = await screen.findByRole("slider", { name: "Master volume" });
    fireEvent.change(slider, { target: { value: "0.6" } });
    const applied = transport.sent.find(
      (r): r is Extract<CommandRequest, { cmd: "audio.set-volume" }> =>
        r.cmd === "audio.set-volume",
    );
    expect(applied?.args.volume).toBe(0.6);
  });

  it("toggling mute dispatches audio.set-mute", async () => {
    const transport = makeFixtureTransport(() => ({
      completed: { kind: "applied-bool", value: true },
    }));
    renderWith(transport);
    fireEvent.click(await screen.findByRole("switch", { name: "Mute" }));
    const applied = transport.sent.find(
      (r): r is Extract<CommandRequest, { cmd: "audio.set-mute" }> =>
        r.cmd === "audio.set-mute",
    );
    expect(applied?.args.muted).toBe(true);
  });
});

describe("EffectsArea — pending controls are inert WITH REASON", () => {
  it("renders the balance slider disabled, naming the awaited command", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.get-volume" ? volumeResult(1.0) : { completed: { kind: "unit" } },
    );
    renderWith(transport);
    const balance = await screen.findByRole("slider", { name: "Balance" });
    expect(balance).toBeDisabled();
    expect(balance.closest("[data-pending-command]")?.getAttribute("data-pending-command")).toBe(
      "audio.set-balance",
    );
    expect(screen.getAllByRole("note").length).toBeGreaterThan(0);
  });

  it("renders the four video adjust sliders disabled (video.set-adjust)", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.get-volume" ? volumeResult(1.0) : { completed: { kind: "unit" } },
    );
    renderWith(transport);
    for (const name of ["Brightness", "Contrast", "Saturation", "Hue"]) {
      const slider = await screen.findByRole("slider", { name });
      expect(slider).toBeDisabled();
      expect(
        slider.closest("[data-pending-command]")?.getAttribute("data-pending-command"),
      ).toBe("video.set-adjust");
    }
  });

  it("pending controls never send anything", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.get-volume" ? volumeResult(1.0) : { completed: { kind: "unit" } },
    );
    renderWith(transport);
    await screen.findByRole("slider", { name: "Hue" });
    // Only the volume hydration happened — no config.set-setting, no
    // invented writes for keys the engine does not own.
    expect(transport.sent.map((r) => r.cmd)).toEqual(["audio.get-volume"]);
  });
});
