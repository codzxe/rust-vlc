// EqualizerTool tests (w26.c1) — pins the EQ tool card per the frames
// (Audio - Equalizer.svg + EQ presets.svg): collapse behavior, the 11 band
// rows with their chips/sliders/accents, preset selection through the shared
// Dropdown, and the onChange wire seam.
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { keys as toolsKeys } from "../../../../i18n/keys/w26.a18";
import { EqualizerTool } from "./EqualizerTool";
import { EQ_PRESETS } from "./presets";

const EXPAND = toolsKeys["player.tools.expand-settings"];
const COLLAPSE = toolsKeys["player.tools.collapse-settings"];

describe("EqualizerTool — collapse behavior (frame: two stacked states)", () => {
  it("ships collapsed per the frames: toggle bar only, body hidden but wired", () => {
    render(<EqualizerTool />);
    const toggle = screen.getByRole("button", { name: EXPAND });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
    expect(body).toHaveAttribute("role", "group");
    // Collapsed card exposes no sliders/chips to the a11y tree.
    expect(screen.queryByRole("slider")).toBeNull();
  });

  it("the card is a named region (settings area mount)", () => {
    render(<EqualizerTool />);
    expect(screen.getByRole("region", { name: "Equalizer" })).toBeInTheDocument();
  });

  it("expand reveals the rows; collapse hides them again", () => {
    render(<EqualizerTool />);
    const toggle = screen.getByRole("button", { name: EXPAND });
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: COLLAPSE })).toBe(toggle);
    expect(screen.getAllByRole("slider")).toHaveLength(11);
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("slider")).toBeNull();
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<EqualizerTool defaultOpen />);
    expect(screen.getByRole("button", { name: COLLAPSE })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });
});

describe("EqualizerTool — band rows (frame: Preamp + 10 bands, pitch 90)", () => {
  it("renders Preamp + the 10 frame bands, each with a labelled slider", () => {
    render(<EqualizerTool defaultOpen />);
    expect(screen.getByRole("slider", { name: "Preamp" })).toBeInTheDocument();
    // Frame labels render verbatim ("60 Hz" … "16 KHz").
    expect(screen.getByRole("slider", { name: "60 Hz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "170 Hz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "310 Hz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "600 Hz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "1 KHz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "3 KHz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "6 KHz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "12 KHz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "14 KHz" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "16 KHz" })).toBeInTheDocument();
    expect(screen.getAllByRole("slider")).toHaveLength(11);
  });

  it("renders the frame value chips: preamp 12.0 dB, bands 0.0 dB", () => {
    render(<EqualizerTool defaultOpen />);
    expect(screen.getAllByText("12.0 dB")).toHaveLength(1);
    expect(screen.getAllByText("0.0 dB")).toHaveLength(10);
  });

  it("formats slider aria-valuetext with the unit (doc 08 §4.5)", () => {
    render(<EqualizerTool defaultOpen />);
    expect(screen.getByRole("slider", { name: "Preamp" })).toHaveAttribute(
      "aria-valuetext",
      "12.0 dB",
    );
    expect(screen.getByRole("slider", { name: "60 Hz" })).toHaveAttribute(
      "aria-valuetext",
      "0.0 dB",
    );
  });

  it("carries the ±20 dB band range and 10 accent dots from the eq tokens", () => {
    const { container } = render(<EqualizerTool defaultOpen />);
    for (const slider of screen.getAllByRole("slider")) {
      expect(slider).toHaveAttribute("aria-valuemin", "-20");
      expect(slider).toHaveAttribute("aria-valuemax", "20");
    }
    // 10 band dots carry a token index; the Preamp spacer keeps alignment.
    const accents = container.querySelectorAll(".eq-tool__accent");
    expect(accents).toHaveLength(11);
    expect(container.querySelectorAll(".eq-tool__accent[data-accent]")).toHaveLength(10);
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

  it("selecting a preset applies its band gains to the row chips", () => {
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

describe("EqualizerTool — band commits (doc 08 §5: live then commit)", () => {
  it("a keyboard step updates the chip and fires onChange with the new gains", () => {
    const onChange = vi.fn();
    render(<EqualizerTool defaultOpen onChange={onChange} />);
    const band60 = screen.getByRole("slider", { name: "60 Hz" });
    fireEvent.keyDown(band60, { key: "ArrowRight" });
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
      key: "ArrowLeft",
    });
    expect(screen.getAllByText("11.0 dB")).toHaveLength(1);
    expect(onChange).toHaveBeenLastCalledWith({
      preamp: 11,
      gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      preset: "none",
    });
  });

  it("manual band edits keep the last applied preset label (VLC behavior)", () => {
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
