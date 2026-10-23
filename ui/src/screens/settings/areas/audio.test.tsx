// Card 4.E3 — Audio area tests. The card's key honest rule under test:
// absent capabilities render DISABLED WITH REASON (visible, inert, never
// enabled, never hidden), and the device list renders only from real engine
// enumeration — config-never-invents.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import type { CommandRequest } from "../../../client/transport";
import type { CommandResponse } from "../../../client/transport";
import { AudioArea } from "./audio";
import { makeFixtureTransport } from "./audio-fixtures";
import type { AudioDevice } from "./audio";

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

const renderWith = (transport: CommandTransportShim) =>
  render(
    <ClientProvider transport={transport}>
      <AudioArea />
    </ClientProvider>,
  );

type CommandTransportShim = Parameters<typeof ClientProvider>[0]["transport"];

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
    renderWith(transport);
    expect(await screen.findByLabelText("HDMI output")).toBeInTheDocument();
    expect(screen.getByLabelText("System default — default")).toBeInTheDocument();
    expect(transport.sent.some((r) => r.cmd === "audio.list-devices")).toBe(true);
  });

  it("pre-selects the engine-reported default device", async () => {
    const transport = makeFixtureTransport(() => devicesResult(DEVICES));
    renderWith(transport);
    await waitFor(() => expect(screen.getByRole("radiogroup")).toBeEnabled());
    expect(screen.getByLabelText("System default — default")).toBeChecked();
  });

  it("selecting a device dispatches audio.set-device with the engine device id", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.list-devices" ? devicesResult(DEVICES) : { completed: { kind: "unit" } },
    );
    renderWith(transport);
    fireEvent.click(await screen.findByLabelText("HDMI output"));
    const applied = findSetDevice(transport.sent);
    expect(applied?.args.id).toBe("wasapi:hdmi");
    await waitFor(() => expect(screen.getByLabelText("HDMI output")).toBeChecked());
  });

  it("a failed set-device reverts the selection (truthful accepted-vs-completed)", async () => {
    const transport = makeFixtureTransport((request) =>
      request.cmd === "audio.list-devices" ? devicesResult(DEVICES) : errorResult("device"),
    );
    renderWith(transport);
    fireEvent.click(await screen.findByLabelText("HDMI output"));
    await waitFor(() => expect(screen.getByLabelText("System default — default")).toBeChecked());
  });
});

describe("AudioDeviceControl — absent capability renders disabled WITH REASON", () => {
  it("feature-not-exposed: control is visible, disabled, and carries the honest reason", async () => {
    renderWith(new MockTransport()); // answers feature-not-exposed for audio.list-devices
    // Not hidden — the exact rule: disabled + reason text.
    expect(await screen.findByText("Device enumeration is not available")).toBeVisible();
    const group = screen.getByRole("radiogroup", { name: "Output device" });
    expect(group).toBeDisabled();
    expect(group).not.toBeEnabled();
    expect(screen.getByRole("note")).toBeVisible();
  });

  it("disabled control is inert — no audio.set-device is ever dispatched", async () => {
    const transport = makeFixtureTransport(() => errorResult("feature-not-exposed"));
    renderWith(transport);
    const group = await screen.findByRole("radiogroup");
    fireEvent.click(group); // nothing to activate: no options, group disabled
    expect(transport.sent.filter((r) => r.cmd === "audio.set-device")).toHaveLength(0);
    expect(transport.sent.map((r) => r.cmd)).toEqual(["audio.list-devices"]);
  });

  it("pending enumeration renders disabled with the loading reason, then the final reason", async () => {
    const transport = makeFixtureTransport(() => errorResult("feature-not-exposed"));
    renderWith(transport);
    expect(screen.getByText("Checking for available audio devices…")).toBeInTheDocument();
    expect(screen.getByRole("radiogroup")).toBeDisabled();
    expect(await screen.findByText("Device enumeration is not available")).toBeInTheDocument();
  });

  it("empty enumeration → disabled with the no-devices reason (no invented devices)", async () => {
    const transport = makeFixtureTransport(() => devicesResult([]));
    renderWith(transport);
    expect(await screen.findByText("No audio devices found")).toBeVisible();
    expect(screen.getByRole("radiogroup")).toBeDisabled();
  });

  it("non-feature errors → generic honest reason, still disabled", async () => {
    const transport = makeFixtureTransport(() => errorResult("internal"));
    renderWith(transport);
    expect(await screen.findByText("Not available in this build")).toBeVisible();
    expect(screen.getByRole("radiogroup")).toBeDisabled();
  });
});

describe("AudioArea — config-never-invents (schema-backed options)", () => {
  it("renders no control for audio exclusivity mode: no schema key → nothing, not a dead control", async () => {
    const transport = makeFixtureTransport(() => devicesResult(DEVICES));
    const { container } = renderWith(transport);
    await waitFor(() => expect(screen.getByRole("radiogroup")).toBeEnabled());
    // Exactly one control: the capability-backed device picker. No invented
    // exclusivity/volume/normalize/visualizer controls render.
    expect(container.querySelectorAll(".settings-field")).toHaveLength(1);
    expect(container.querySelector('[data-setting="audio.exclusive-output"]')).toBeNull();
  });
});
