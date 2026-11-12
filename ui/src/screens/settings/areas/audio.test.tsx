// w26.gfix-B — Audio area tests (g12-P0-2 rewrite): the area composes the
// frame's 3 schema sections (Settings - audio.svg §C6) with the
// capability-backed Output device row interleaved (frame order: Enable audio ·
// Output device · Speaker setup). The card's key honest rule under test:
// absent capabilities render DISABLED WITH REASON (the closed 320×36 dropdown
// trigger — visible, inert, never a 4×4 stub, g12-P1-5), and the device list
// renders only from real engine enumeration — config-never-invents.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import type { CommandRequest } from "../../../client/transport";
import type { CommandResponse } from "../../../client/transport";
import { AudioArea, AudioDeviceControl } from "./audio";
import { makeFixtureTransport } from "./audio-fixtures";
import type { AudioDevice } from "./audio";
import { SettingsProvider } from "../SettingsProvider";
import { SETTINGS_ENTRIES } from "../generated/schema";

const DEVICES: readonly AudioDevice[] = [
  { id: "wasapi:default", isDefault: true, name: "System default" },
  { id: "wasapi:hdmi", isDefault: false, name: "HDMI output" },
];

type WireError = Extract<CommandResponse["result"], { error: unknown }>["error"];

const devicesResult = (devices: readonly AudioDevice[]): CommandResponse["result"] => ({
  completed: { kind: "audio-devices", devices: [...devices] },
});

const errorResult = (code: WireError["code"]): CommandResponse["result"] => ({
  error: { code, details: {}, message: "fixture", stage: "control" },
});

type CommandTransportShim = Parameters<typeof ClientProvider>[0]["transport"];

const renderWith = (transport: CommandTransportShim) =>
  render(
    <ClientProvider transport={transport}>
      <SettingsProvider>
        <AudioArea />
      </SettingsProvider>
    </ClientProvider>,
  );

const renderControl = (transport: CommandTransportShim) =>
  render(
    <ClientProvider transport={transport}>
      <SettingsProvider>
        <AudioDeviceControl />
      </SettingsProvider>
    </ClientProvider>,
  );

const findSetDevice = (
  sent: readonly CommandRequest[],
): Extract<CommandRequest, { cmd: "audio.set-device" }> | undefined =>
  sent.find((r) => r.cmd === "audio.set-device") as
    | Extract<CommandRequest, { cmd: "audio.set-device" }>
    | undefined;

describe("AudioDeviceControl — device list from real enumeration", () => {
  it("queries the engine (audio.list-devices) and renders the fixture enumeration", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.list-devices" ? devicesResult(DEVICES) : { completed: { kind: "unit" } },
    );
    renderControl(transport);
    const trigger = await screen.findByRole("button", { name: "Output device" });
    await waitFor(() => expect(trigger).toBeEnabled());
    // The engine-reported default is pre-selected on the closed trigger.
    expect(trigger).toHaveTextContent("System default — default");
    fireEvent.click(trigger);
    expect(screen.getByRole("option", { name: "HDMI output" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "System default — default", selected: true })).toBeInTheDocument();
    expect(transport.sent.some((r) => r.cmd === "audio.list-devices")).toBe(true);
  });

  it("pre-selects the engine-reported default device", async () => {
    const transport = makeFixtureTransport(() => devicesResult(DEVICES));
    renderControl(transport);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Output device" })).toHaveTextContent(
        "System default — default",
      ),
    );
  });

  it("selecting a device dispatches audio.set-device with the engine device id", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.list-devices" ? devicesResult(DEVICES) : { completed: { kind: "unit" } },
    );
    renderControl(transport);
    fireEvent.click(await screen.findByRole("button", { name: "Output device" }));
    fireEvent.click(screen.getByRole("option", { name: "HDMI output" }));
    const applied = findSetDevice(transport.sent);
    expect(applied?.args.id).toBe("wasapi:hdmi");
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Output device" })).toHaveTextContent("HDMI output"),
    );
  });

  it("a failed set-device reverts the selection (truthful accepted-vs-completed)", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.list-devices" ? devicesResult(DEVICES) : errorResult("device"),
    );
    renderControl(transport);
    fireEvent.click(await screen.findByRole("button", { name: "Output device" }));
    fireEvent.click(screen.getByRole("option", { name: "HDMI output" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Output device" })).toHaveTextContent(
        "System default — default",
      ),
    );
  });
});

describe("AudioDeviceControl — absent capability renders disabled WITH REASON", () => {
  it("feature-not-exposed: the closed trigger is visible, disabled, and carries the honest reason", async () => {
    renderControl(new MockTransport()); // answers feature-not-exposed for audio.list-devices
    // Not hidden — the exact rule: the 320×36 trigger stays visible (never a
    // 4×4 stub) with the honest fallback value + the reason text.
    expect(await screen.findByText("Device enumeration is not available")).toBeVisible();
    const trigger = screen.getByRole("button", { name: "Output device" });
    expect(trigger).toBeDisabled();
    expect(trigger).toHaveTextContent("System default"); // frame C6 fallback value
    expect(screen.getByRole("note")).toBeVisible();
  });

  it("disabled control is inert — no audio.set-device is ever dispatched", async () => {
    const transport = makeFixtureTransport(() => errorResult("feature-not-exposed"));
    renderControl(transport);
    fireEvent.click(await screen.findByRole("button", { name: "Output device" })); // disabled: nothing opens
    expect(transport.sent.filter((r) => r.cmd === "audio.set-device")).toHaveLength(0);
    expect(transport.sent.map((r) => r.cmd)).toEqual(["audio.list-devices"]);
  });

  it("pending enumeration renders disabled with the loading reason, then the final reason", async () => {
    const transport = makeFixtureTransport(() => errorResult("feature-not-exposed"));
    renderControl(transport);
    expect(screen.getByText("Checking for available audio devices…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Output device" })).toBeDisabled();
    expect(await screen.findByText("Device enumeration is not available")).toBeInTheDocument();
  });

  it("empty enumeration → disabled with the no-devices reason (no invented devices)", async () => {
    const transport = makeFixtureTransport(() => devicesResult([]));
    renderControl(transport);
    expect(await screen.findByText("No audio devices found")).toBeVisible();
    expect(screen.getByRole("button", { name: "Output device" })).toBeDisabled();
  });

  it("non-feature errors → generic honest reason, still disabled", async () => {
    const transport = makeFixtureTransport(() => errorResult("internal"));
    renderControl(transport);
    expect(await screen.findByText("Not available in this build")).toBeVisible();
    expect(screen.getByRole("button", { name: "Output device" })).toBeDisabled();
  });
});

describe("AudioArea — the frame's sections from the seed (g12-P0-2)", () => {
  it("renders every schema row bound to its key, in frame order with the hand row interleaved", () => {
    const transport = makeFixtureTransport(() => devicesResult(DEVICES));
    const { container } = renderWith(transport);
    for (const entry of SETTINGS_ENTRIES.filter((e) => e.screen === "audio")) {
      const field =
        entry.key === "audio.startup-volume-mode"
          ? container.querySelector('[data-setting-composite="audio.startup-volume"]') // grouped inside the compound widget
          : container.querySelector(`[data-setting="${entry.key}"]`);
      expect(field, `audio entry ${entry.key} must render bound to its schema key`).not.toBeNull();
    }
    // Frame order: Enable audio · Output device (hand) · Speaker setup.
    const output = container.querySelector('[data-section="output"]');
    const rows = [...(output?.querySelectorAll(".settings-field") ?? [])];
    expect(rows.map((row) => row.querySelector(".settings-field__label")?.textContent)).toEqual([
      "Enable audio",
      "Output device",
      "Speaker setup",
    ]);
    // Output — frame states.
    expect(screen.getByRole("switch", { name: "Enable audio" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("button", { name: "Speaker setup" })).toHaveTextContent("Automatic");
    // Volume — the frame's slider + Values-1 radio rows + step input.
    expect(screen.getByRole("slider", { name: "Maximum volume" })).toHaveAttribute("aria-valuenow", "200");
    const group = screen.getByRole("radiogroup", { name: "Startup volume" });
    expect(screen.getByRole("radio", { name: "Remember last volume" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("radio", { name: "Always starts at:" })).toHaveAttribute("aria-checked", "false");
    // The nested input renders dimmed+inert while row 1 owns the selection.
    const nested = group.querySelector(".settings-values__input") as HTMLInputElement | null;
    expect(nested).toBeDisabled();
    expect(nested?.value).toBe("200%");
    const step = screen.getByRole("spinbutton", { name: "Volume step" }) as HTMLInputElement;
    expect(step).toHaveValue(1); // frame: input "1"
    // Loudness — the frame's 5 rows.
    expect(screen.getByRole("switch", { name: "Normalize loudness" })).toHaveAttribute("aria-checked", "false");
    expect((screen.getByRole("spinbutton", { name: "Target level" }) as HTMLInputElement).value).toBe("1");
    expect(screen.getByRole("switch", { name: "Night mode" })).toHaveAttribute("aria-checked", "false");
    expect(screen.getByRole("button", { name: "Replay gain" })).toHaveTextContent("Off");
    // w26.gfix-B: the float well spells the frame's one-decimal "0.0" (the
    // bare number input would render "0" — audio.svg pin, EQ chip grammar).
    expect((screen.getByRole("textbox", { name: "Replay gain preamp" }) as HTMLInputElement).value).toBe("0.0");
  });

  it("renders no control for audio exclusivity mode: no schema key → nothing, not a dead control", () => {
    const transport = makeFixtureTransport(() => devicesResult(DEVICES));
    const { container } = renderWith(transport);
    expect(container.querySelector('[data-setting="audio.exclusive-output"]')).toBeNull();
    expect(screen.queryByText(/exclusiv/i)).toBeNull();
  });
});
