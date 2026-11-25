// EqualizerTool tests (w26.c1; w26.f05 A2 rewrite; FX4-A08 wired) — pins
// the tools-dialog Equalizer surface per the w26.f05 Lead ruling: the EQ
// presets selector (EQ presets.svg) above the GraphicEQ vertical-fader
// panel (EQ.svg). FX4-A08 (wave FX4, plan F1.1) cut the panel to the
// engine's 3-band Eq3Band truth and DISPATCHES the real `audio.set-eq`
// verb through the player context on every commit (the dispatch pins live
// in fx4_a08_eq_dispatch.test.tsx); collapse behavior, the 4 vertical
// faders, preset selection through the shared Dropdown, and the onChange
// wire seam stay under test here — with NO staging markers (retired).
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { keys as panelKeys } from "../../../../i18n/keys/w26.f05";
import { EqualizerTool } from "./EqualizerTool";
import { EQ_PRESETS } from "./presets";

const OPEN = panelKeys["screen.eq.open-panel"];
const CLOSE = panelKeys["screen.eq.close-panel"];

describe("EqualizerTool — collapse behavior (frame: EQ.svg bar↔panel)", () => {
  it("ships collapsed per the frames: the 'Open 3-band equalizer' bar, faders hidden", () => {
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

  it("expand reveals the preset selector + the 4 faders; collapse hides them", () => {
    render(<EqualizerTool />);
    const toggle = screen.getByRole("button", { name: OPEN });
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: CLOSE })).toBe(toggle);
    // FX4-A08: Preamp + the engine's 3 bands.
    expect(screen.getAllByRole("slider")).toHaveLength(4);
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

describe("EqualizerTool — fader columns (FX4-A08: Preamp + the engine's 3 bands)", () => {
  it("renders Preamp + the engine's 3 bands as vertical faders, frequencies stated", () => {
    render(<EqualizerTool defaultOpen />);
    for (const label of ["Preamp", "Low (250 Hz)", "Mid (1 kHz)", "High (4 kHz)"]) {
      const fader = screen.getByRole("slider", { name: label });
      expect(fader).toHaveAttribute("aria-orientation", "vertical");
    }
    expect(screen.getAllByRole("slider")).toHaveLength(4);
  });

  it("renders the chips flat at the engine's unity default (0.0 dB x4)", () => {
    render(<EqualizerTool defaultOpen />);
    // FX4-A08: default preamp is the engine identity (0 dB), bands flat.
    expect(screen.getAllByText("0.0 dB")).toHaveLength(4);
  });

  it("carries the ±20 dB band range; monochrome columns (no accent dots, F8)", () => {
    const { container } = render(<EqualizerTool defaultOpen />);
    for (const slider of screen.getAllByRole("slider")) {
      expect(slider).toHaveAttribute("aria-valuemin", "-20");
      expect(slider).toHaveAttribute("aria-valuemax", "20");
    }
    // w26.k10 (j10 P3-2): the real monochrome namespace is [data-accent]
    // (GraphicEQ.test pins the same marker).
    expect(container.querySelector("[data-accent]")).toBeNull();
    expect(container.querySelectorAll(".eq-panel__fader--zero")).toHaveLength(3);
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

  it("the listbox's children are exactly the 8 options in preset order", () => {
    // The option ORDER is still frame-pinned (labels below); since w26.k10
    // the preset curve glyphs key on the structural data-preset marker, so
    // this pin guards order for the LABELS, no longer for glyph geometry.
    render(<EqualizerTool defaultOpen />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    const listbox = screen.getByRole("listbox", { name: "Equalizer preset" });
    expect(listbox.children).toHaveLength(EQ_PRESETS.length);
    const names = [...listbox.children].map((child) => child.textContent);
    expect(names).toEqual(["None", "Flat", "Classical", "Club", "Dance", "Full bass", "Full bass & treble", "Full treble"]);
    // Every option carries its structural data-preset marker (equalizer.css
    // keys the per-preset curve glyphs on it — j10 P3-6).
    const markers = [...listbox.children].map((child) =>
      child.getAttribute("data-preset"),
    );
    expect(markers).toEqual(EQ_PRESETS.map((preset) => preset.id));
    for (const child of [...listbox.children]) {
      expect(child).toHaveAttribute("role", "option");
    }
  });

  it("the LAST preset (Full treble) selects and applies — h13 #1 regression pin", () => {
    // h13's functional NO: row 8 was unreachable by pointer (the shared
    // Dropdown's scrollport reset to 0 mid-click, so the row escaped the
    // cursor between mousedown and mouseup). The root fix is i16's stable
    // scrollport (Dropdown.tsx, w26.i16); the EQ scope ships the frame's
    // 348px list (equalizer.css) so row 8 is scroll-required again — this
    // pins the last row's selection wiring end-to-end (the live real-mouse
    // repro is in the lane log w26.i06 §verify).
    render(<EqualizerTool defaultOpen />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Full treble" }));
    // FX4-A08: Full treble projects to the 3 engine bands as
    // [-9.6, 0, 16.1] (presets.ts's mechanical projection).
    expect(screen.getAllByText("-9.6 dB")).toHaveLength(1);
    expect(screen.getAllByText("16.1 dB")).toHaveLength(1);
    // The list closed and the trigger reads the applied preset.
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(
      screen.getByRole("button", { name: "Equalizer preset" }).textContent,
    ).toContain("Full treble");
  });

  it("selecting a preset applies its projected band gains to the fader chips", () => {
    render(<EqualizerTool defaultOpen />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Club" }));
    // FX4-A08: Club projects to [0, 3.9, 0.6] — the chips follow.
    expect(screen.getAllByText("3.9 dB")).toHaveLength(1);
    expect(screen.getAllByText("0.6 dB")).toHaveLength(1);
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
      preamp: 0,
      gains: classical?.gains,
      preset: "classical",
    });
    // Preamp is untouched by presets (the engine's unity default stays).
    expect(screen.getAllByText("0.0 dB")).toHaveLength(2);
  });
});

describe("EqualizerTool — fader commits (doc 08 §5: live then commit)", () => {
  it("a keyboard step updates the chip and fires onChange with the new gains", () => {
    const onChange = vi.fn();
    render(<EqualizerTool defaultOpen onChange={onChange} />);
    const lowBand = screen.getByRole("slider", { name: "Low (250 Hz)" });
    fireEvent.keyDown(lowBand, { key: "ArrowUp" });
    // w26.k10 (j10 P2-1): the lattice is 0.1 dB — ArrowUp from 0 is +0.1,
    // and the value lands as a CLEAN one-decimal double (no 0.1-float dust
    // in aria-valuenow or the seam payload).
    expect(lowBand).toHaveAttribute("aria-valuenow", "0.1");
    expect(screen.getAllByText("0.1 dB")).toHaveLength(1);
    expect(onChange).toHaveBeenLastCalledWith({
      preamp: 0,
      gains: [0.1, 0, 0],
      preset: "none",
    });
  });

  it("a preamp commit updates its chip and the seam payload", () => {
    const onChange = vi.fn();
    render(<EqualizerTool defaultOpen onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Preamp" }), {
      key: "ArrowDown",
    });
    expect(screen.getAllByText("-0.1 dB")).toHaveLength(1);
    expect(onChange).toHaveBeenLastCalledWith({
      preamp: -0.1,
      gains: [0, 0, 0],
      preset: "none",
    });
  });

  it("keyboard steps stay ON the fader lattice from fractional preset gains (P2-1 regression)", () => {
    // The j10 P2-1 defect: with a 1 dB step, ArrowUp from a fractional
    // preset gain jumped asymmetrically — off the lattice a drag can land
    // on. At 0.1 dB the steps are symmetric from ANY preset value; the
    // coarse step (Shift/PageUp = 10 × 0.1) is symmetric too.
    render(<EqualizerTool defaultOpen />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Dance" }));
    const lowBand = screen.getByRole("slider", { name: "Low (250 Hz)" });
    expect(lowBand).toHaveAttribute("aria-valuenow", "4.6");
    fireEvent.keyDown(lowBand, { key: "ArrowUp" });
    expect(lowBand).toHaveAttribute("aria-valuenow", "4.7");
    expect(screen.getAllByText("4.7 dB")).toHaveLength(1);
    fireEvent.keyDown(lowBand, { key: "ArrowDown" });
    fireEvent.keyDown(lowBand, { key: "ArrowDown" });
    expect(lowBand).toHaveAttribute("aria-valuenow", "4.5");
    expect(screen.getAllByText("4.5 dB")).toHaveLength(1);
    fireEvent.keyDown(lowBand, { key: "ArrowUp", shiftKey: true });
    expect(lowBand).toHaveAttribute("aria-valuenow", "5.5");
  });

  it("manual fader edits keep the last applied preset label (VLC behavior)", () => {
    render(<EqualizerTool defaultOpen />);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Dance" }));
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    // The band left the Dance shape, but the selector still names it.
    expect(
      screen.getByRole("button", { name: "Equalizer preset" }).textContent,
    ).toContain("Dance");
    // 4.6 + 0.1 (ArrowUp) — one lattice step, exactly.
    expect(screen.getAllByText("4.7 dB")).toHaveLength(1);
  });

  it("defaultState seeds the tool (hydration seam)", () => {
    render(
      <EqualizerTool
        defaultOpen
        defaultState={{ preamp: 6, gains: [3, 0, 0] }}
      />,
    );
    expect(screen.getAllByText("6.0 dB")).toHaveLength(1);
    expect(screen.getAllByText("3.0 dB")).toHaveLength(1);
  });

  it("FX4-A08: the tool carries NO staged markers — it dispatches the real verb", () => {
    // The pre-wave tool self-staged (data-staged-tool/data-pending-command
    // + an in-body note naming the awaited audio.set-eq command). The
    // command exists and is called now (PlayerActions.setEq — the dispatch
    // pins live in fx4_a08_eq_dispatch.test.tsx), so every staging marker
    // is retired; this pin keeps them retired.
    const { container } = render(<EqualizerTool />);
    const tool = container.querySelector('[data-tool="equalizer"]');
    expect(tool).not.toBeNull();
    expect(tool).not.toHaveAttribute("data-staged-tool");
    expect(tool).not.toHaveAttribute("data-pending-command");
    expect(container.querySelector(".eq-tool__staged")).toBeNull();
    // Collapsed: no note in the a11y tree — and none when expanded either.
    expect(screen.queryByRole("note")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: OPEN }));
    expect(screen.queryByRole("note")).toBeNull();
  });
});
