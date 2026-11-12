// Go-to-Time dialog tests (issue #32, `ui-dialog-goto-time`).
//
// Pins the flow contract end-to-end at the seam: renders on the shared
// Dialog, prefills with the engine-reported position, submits a valid
// time as `playback.seek` ABSOLUTE through the player state seam, refuses
// unparseable input and times past the engine-reported duration (honest
// errors, no seek), and closes on submit / Escape / the quiet Close.
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { PlayerControlsProvider } from "../context";
import { ClientProvider } from "../../../client/ClientProvider";
import type { CommandTransport } from "../../../client/transport";
import type { CommandRequest, CommandResponse, Event } from "../../../client/transport";
import { GotoTimeDialog } from "./GotoTimeDialog";
import { GOTO_COPY } from "./useDialogsMsg";

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

/**
 * Per-open host (the PlayerScreen mount pattern): the dialog mounts on
 * demand, so a test can seed seam truth BEFORE the dialog's mount-time
 * prefill captures it.
 */
function GotoOpenOnDemand() {
  const [open, setOpen] = useState(false);
  return open ? (
    <GotoTimeDialog open onClose={() => setOpen(false)} />
  ) : (
    <button type="button" onClick={() => setOpen(true)}>
      open-dialog
    </button>
  );
}

describe("GotoTimeDialog — the seek funnel", () => {
  it("initial focus lands on the Time input, never the header ✕ (h19 P4)", () => {
    // w26.i09: body renders before the header in the DOM (the header
    // paints first via `order: -1`), so the surface trap's first-focusable
    // lands on the Time input — Enter on a fresh dialog submits the seek
    // instead of dismissing the window.
    renderDialog(new RecordingTransport());
    expect(screen.getByLabelText(GOTO_COPY.label)).toHaveFocus();
    expect(
      screen.getByRole("button", { name: GOTO_COPY.close }),
    ).not.toHaveFocus();
  });

  it("renders the dialog with the engine-reported position prefilled", () => {
    renderDialog(new RecordingTransport());
    expect(screen.getByRole("heading", { name: GOTO_COPY.title })).toBeInTheDocument();
    // Position 0 (no engine events) → "0:00" prefill, VLC parity.
    expect(screen.getByLabelText(GOTO_COPY.label)).toHaveValue("0:00");
    // The mm:ss grammar needs ":" — text keypad, not numeric (j09 P3:
    // numeric keypads have no colon).
    expect(screen.getByLabelText(GOTO_COPY.label)).toHaveAttribute("inputmode", "text");
    // Honest unknown duration renders "--:--", never a fabricated ceiling.
    expect(screen.getByText(/0:00\s*\/\s*--:--/)).toBeInTheDocument();
  });

  it("prefills with a nonzero engine-reported position — seam seconds, formatted (j09 §4 gap)", () => {
    // The prefill is captured at mount (= open), so the dialog mounts
    // AFTER the seam carries the engine truth: a host that opens it on
    // demand, the PlayerScreen per-open pattern.
    const transport = new RecordingTransport();
    render(
      <ClientProvider transport={transport}>
        <PlayerControlsProvider transport={transport}>
          <GotoOpenOnDemand />
        </PlayerControlsProvider>
      </ClientProvider>,
    );
    transport.emit({
      gen: 1,
      seq: 1,
      kind: "timing.time-tick",
      position: 90,
      time: 90_000_000,
    });
    fireEvent.click(screen.getByRole("button", { name: "open-dialog" }));
    expect(screen.getByLabelText(GOTO_COPY.label)).toHaveValue("1:30");
    expect(screen.getByText(/1:30\s*\/\s*--:--/)).toBeInTheDocument();
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
