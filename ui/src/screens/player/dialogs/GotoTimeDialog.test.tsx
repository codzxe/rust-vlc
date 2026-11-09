// Go-to-Time dialog tests (issue #32, `ui-dialog-goto-time`).
//
// Pins the flow contract end-to-end at the seam: renders on the shared
// Dialog, prefills with the engine-reported position, submits a valid
// time as `playback.seek` ABSOLUTE through the player state seam, refuses
// unparseable input and times past the engine-reported duration (honest
// errors, no seek), and closes on submit / Escape / the quiet Close.
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PlayerControlsProvider } from "../context";
import { ClientProvider } from "../../../client/ClientProvider";
import type { CommandTransport } from "../../../client/transport";
import type { CommandRequest, CommandResponse, Event } from "../../../client/transport";
import { GotoTimeDialog, GOTO_COPY } from "./GotoTimeDialog";

/**
 * Transport that records every sent command, answers success, and can
 * push engine events (duration truth) into the subscribed seam.
 */
class RecordingTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return {
      gen: this.sent.length,
      requestId: request.requestId,
      result: { completed: { kind: "unit" } },
    };
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /** Push an engine event into every subscriber (duration truth). */
  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }
}

function renderDialog(transport: CommandTransport, onClose = vi.fn()) {
  render(
    <ClientProvider transport={transport}>
      <PlayerControlsProvider transport={transport}>
        <GotoTimeDialog open onClose={onClose} />
      </PlayerControlsProvider>
    </ClientProvider>,
  );
  return onClose;
}

describe("GotoTimeDialog — the seek funnel", () => {
  it("renders the dialog with the engine-reported position prefilled", () => {
    renderDialog(new RecordingTransport());
    expect(screen.getByRole("heading", { name: GOTO_COPY.title })).toBeInTheDocument();
    // Position 0 (no engine events) → "0:00" prefill, VLC parity.
    expect(screen.getByLabelText(GOTO_COPY.label)).toHaveValue("0:00");
    // Honest unknown duration renders "--:--", never a fabricated ceiling.
    expect(screen.getByText(/0:00\s*\/\s*--:--/)).toBeInTheDocument();
  });

  it("submits mm:ss as playback.seek ABSOLUTE seconds and closes", async () => {
    const transport = new RecordingTransport();
    const onClose = renderDialog(transport);
    const input = screen.getByLabelText(GOTO_COPY.label);
    fireEvent.change(input, { target: { value: "1:30" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
    const seek = transport.sent.find((r) => r.cmd === "playback.seek");
    expect(seek?.args).toEqual({
      target: { kind: "absolute", time: 90 },
    });
  });

  it("submits the primary Go button through the same form", async () => {
    const transport = new RecordingTransport();
    renderDialog(transport);
    fireEvent.change(screen.getByLabelText(GOTO_COPY.label), {
      target: { value: "1:02:05" },
    });
    fireEvent.click(screen.getByRole("button", { name: GOTO_COPY.action }));
    await waitFor(() => {
      const seek = transport.sent.find((r) => r.cmd === "playback.seek");
      expect(seek?.args).toEqual({
        target: { kind: "absolute", time: 3725 },
      });
    });
  });

  it("unparseable input is honestly invalid — error shown, no seek, stays open", () => {
    const transport = new RecordingTransport();
    const onClose = renderDialog(transport);
    const input = screen.getByLabelText(GOTO_COPY.label);
    fireEvent.change(input, { target: { value: "banana" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    expect(screen.getByText(GOTO_COPY.invalid)).toBeInTheDocument();
    expect(transport.sent.filter((r) => r.cmd === "playback.seek")).toHaveLength(0);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("a time past the engine-reported duration is refused with the duration in the error", () => {
    const transport = new RecordingTransport();
    const onClose = renderDialog(transport);
    // Engine truth: 10-minute media.
    transport.emit({ gen: 1, seq: 1, kind: "timing.duration-known", duration: 600 });
    const input = screen.getByLabelText(GOTO_COPY.label);
    fireEvent.change(input, { target: { value: "700" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    expect(screen.getByText(GOTO_COPY.beyondEnd("10:00"))).toBeInTheDocument();
    expect(transport.sent.filter((r) => r.cmd === "playback.seek")).toHaveLength(0);
    expect(onClose).not.toHaveBeenCalled();
    // The duration line now reports the known truth.
    expect(screen.getByText(/0:00\s*\/\s*10:00/)).toBeInTheDocument();
  });

  it("unknown duration (live) sets no ceiling — a far time still seeks", async () => {
    const transport = new RecordingTransport();
    renderDialog(transport);
    fireEvent.change(screen.getByLabelText(GOTO_COPY.label), {
      target: { value: "9999" },
    });
    fireEvent.submit(
      screen.getByLabelText(GOTO_COPY.label).closest("form") as HTMLFormElement,
    );
    await waitFor(() => {
      const seek = transport.sent.find((r) => r.cmd === "playback.seek");
      expect(seek?.args).toEqual({ target: { kind: "absolute", time: 9999 } });
    });
  });

  it("Escape closes (Dialog focus trap owns the key)", () => {
    const onClose = vi.fn();
    renderDialog(new RecordingTransport(), onClose);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledWith("escape");
  });

  it("the quiet Close affordance closes without seeking", () => {
    const transport = new RecordingTransport();
    const onClose = renderDialog(transport);
    fireEvent.click(screen.getByRole("button", { name: GOTO_COPY.close }));
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(transport.sent.filter((r) => r.cmd === "playback.seek")).toHaveLength(0);
  });
});
