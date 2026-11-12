// EqualizerTool tests (w26.c1; w26.f05 A2 rewrite) — pins the tools-dialog
// Equalizer surface per the w26.f05 Lead ruling: the EQ presets selector
// (EQ presets.svg) above the GraphicEQ vertical-fader panel (EQ.svg). The
// A1 horizontal rows card is retired; collapse behavior, the 11 vertical
// faders, preset selection through the shared Dropdown, and the onChange
// wire seam stay under test.
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { keys as panelKeys } from "../../../../i18n/keys/w26.f05";
import { EqualizerTool } from "./EqualizerTool";
import { EQ_PRESETS } from "./presets";

const OPEN = panelKeys["screen.eq.open-panel"];
const CLOSE = panelKeys["screen.eq.close-panel"];

describe("EqualizerTool — collapse behavior (frame: EQ.svg bar↔panel)", () => {
  it("ships collapsed per the frames: the 'Open Equalizer' bar, faders hidden", () => {
    render(<EqualizerTool />);
    const toggle = screen.getByRole("button", { name: OPEN });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    // Collapsed tool exposes no sliders/chips to the a11y tree — and the
    // preset selector hides with the panel (the frames show the bar only).
    expect(screen.queryByRole("slider")).toBeNull();
    expect(screen.queryByRole("button", { name: "Equalizer preset" })).toBeNull();
  });

  it("the tool hosts the named Equalizer region (GraphicEQ inside)", () => {
    render(<EqualizerTool />);
    expect(screen.getByRole("region", { name: "Equalizer" })).toBeInTheDocument();
  });

  it("expand reveals the preset selector + the 11 faders; collapse hides them", () => {
    render(<EqualizerTool />);
    const toggle = screen.getByRole("button", { name: OPEN });
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: CLOSE })).toBe(toggle);
    expect(screen.getAllByRole("slider")).toHaveLength(11);
    expect(screen.getByRole("button", { name: "Equalizer preset" })).toBeInTheDocument();
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("slider")).toBeNull();
    expect(screen.queryByRole("button", { name: "Equalizer preset" })).toBeNull();
  });

  it("defaultOpen mounts the tool expanded", () => {
    render(<EqualizerTool defaultOpen />);
    expect(screen.getByRole("button", { name: CLOSE })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });
});

describe("EqualizerTool — fader columns (frame: Preamp + 10 bands, vertical)", () => {
  it("renders Preamp + the 10 frame bands as vertical faders, labels verbatim", () => {
    render(<EqualizerTool defaultOpen />);
    for (const label of [
      "Preamp",
      "60 Hz",
      "170 Hz",
      "310 Hz",
      "600 Hz",
      "1 KHz",
      "3 KHz",
      "6 KHz",
      "12 KHz",
      "14 KHz",
      "16 KHz",
    ]) {
      const fader = screen.getByRole("slider", { name: label });
      expect(fader).toHaveAttribute("aria-orientation", "vertical");
    }
    expect(screen.getAllByRole("slider")).toHaveLength(11);
  });

  it("renders the frame value chips: preamp 12.0 dB, bands 0.0 dB", () => {
    render(<EqualizerTool defaultOpen />);
    expect(screen.getAllByText("12.0 dB")).toHaveLength(1);
    expect(screen.getAllByText("0.0 dB")).toHaveLength(10);
  });

  it("carries the ±20 dB band range; monochrome columns (no accent dots, F8)", () => {
    const { container } = render(<EqualizerTool defaultOpen />);
    for (const slider of screen.getAllByRole("slider")) {
      expect(slider).toHaveAttribute("aria-valuemin", "-20");
      expect(slider).toHaveAttribute("aria-valuemax", "20");
    }
    expect(container.querySelector(".eq-tool__accent")).toBeNull();
    expect(container.querySelectorAll(".eq-panel__fader--zero")).toHaveLength(10);
  });
});

describe("EqualizerTool — preset panel (frame: EQ presets.svg)", () => {
  it("renders the frame's 8 presets in the shared Dropdown grammar", () => {
    render(<EqualizerTool defaultOpen />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    const listbox = screen.getByRole("listbox", { name: "Equalizer preset" });
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(8);
    expect(listbox).toContainElement(screen.getByRole("option", { name: "None" }));
    expect(listbox).toContainElement(screen.getByRole("option", { name: "Flat" }));
    expect(listbox).toContainElement(screen.getByRole("option", { name: "Full bass" }));
    expect(listbox).toContainElement(
      screen.getByRole("option", { name: "Full bass & treble" }),
    );
  });

  it("selecting a preset applies its band gains to the fader chips", () => {
    render(<EqualizerTool defaultOpen />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Club" }));
    // Club = [0, 0, 4.8, 3.6, 3.6, 3.6, 2.4, 0, 0, 0] — chips follow.
    expect(screen.getAllByText("4.8 dB")).toHaveLength(1);
    expect(screen.getAllByText("3.6 dB")).toHaveLength(3);
    expect(screen.getAllByText("2.4 dB")).toHaveLength(1);
    // The trigger now reads the applied preset.
    expect(
      screen.getByRole("button", { name: "Equalizer preset" }).textContent,
    ).toContain("Club");
  });

  it("a preset pick fires onChange with the full state (wire seam)", () => {
    const onChange = vi.fn();
    render(<EqualizerTool defaultOpen onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Classical" }));
    const classical = EQ_PRESETS.find((preset) => preset.id === "classical");
    expect(onChange).toHaveBeenLastCalledWith({
      preamp: 12,
      gains: classical?.gains,
      preset: "classical",
    });
    // Preamp is untouched by presets (frame: presets shape the bands).
    expect(screen.getAllByText("12.0 dB")).toHaveLength(1);
  });
});

describe("EqualizerTool — fader commits (doc 08 §5: live then commit)", () => {
  it("a keyboard step updates the chip and fires onChange with the new gains", () => {
    const onChange = vi.fn();
    render(<EqualizerTool defaultOpen onChange={onChange} />);
    const band60 = screen.getByRole("slider", { name: "60 Hz" });
    fireEvent.keyDown(band60, { key: "ArrowUp" });
    expect(band60).toHaveAttribute("aria-valuenow", "1");
    expect(screen.getAllByText("1.0 dB")).toHaveLength(1);
    expect(onChange).toHaveBeenLastCalledWith({
      preamp: 12,
      gains: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      preset: "none",
    });
  });

  it("a preamp commit updates its chip and the seam payload", () => {
    const onChange = vi.fn();
    render(<EqualizerTool defaultOpen onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Preamp" }), {
      key: "ArrowDown",
    });
    expect(screen.getAllByText("11.0 dB")).toHaveLength(1);
    expect(onChange).toHaveBeenLastCalledWith({
      preamp: 11,
      gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      preset: "none",
    });
  });

  it("manual fader edits keep the last applied preset label (VLC behavior)", () => {
    render(<EqualizerTool defaultOpen />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Dance" }));
    fireEvent.keyDown(screen.getByRole("slider", { name: "60 Hz" }), {
      key: "ArrowUp",
    });
    // The band left the Dance shape, but the selector still names it.
    expect(
      screen.getByRole("button", { name: "Equalizer preset" }).textContent,
    ).toContain("Dance");
    // 5.6 + 1 (ArrowUp) then snapped onto the 1 dB step lattice → 7.0.
    expect(screen.getAllByText("7.0 dB")).toHaveLength(1);
  });

  it("defaultState seeds the tool (hydration seam)", () => {
    render(
      <EqualizerTool
        defaultOpen
        defaultState={{ preamp: 6, gains: [3, 0, 0, 0, 0, 0, 0, 0, 0, 0] }}
      />,
    );
    expect(screen.getAllByText("6.0 dB")).toHaveLength(1);
    expect(screen.getAllByText("3.0 dB")).toHaveLength(1);
  });
});
