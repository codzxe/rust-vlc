// useDropdownScrollportFlip tests (w26.i08) — pins the clip-aware placement
// decision that closes h15's HDR NO (h15-P1 / g17-P1-1). The DOM half of the
// hook needs layout (jsdom has none — the live behavior is verified by the
// lane's Playwright probe, /tmp/cap-w26i08/); the decision itself is a pure
// function measured against the geometry h15/g17 actually captured in the
// tools dialog.

import { describe, expect, it } from "vitest";
import { resolveListPlacement } from "./useDropdownScrollportFlip";

/** The HDR rest state at 1440×900 as measured live (probe 2026-11-14):
 * trigger 544.8–592.8 inside a dialog body clip box of 304.8–645.3 — the
 * 6-row list (280px at the shared 44@46 grammar) fits NEITHER side. */
const HDR_REST_1440 = {
  trigger: { top: 544.8, bottom: 592.8, left: 541, width: 358 },
  clip: { top: 304.8, bottom: 645.3 },
  listHeight: 280,
  viewportHeight: 900,
};

describe("resolveListPlacement — in-flow (the shared flip owns it)", () => {
  it("fits below the clip box → inflow (shared data-direction holds)", () => {
    const placement = resolveListPlacement({
      ...HDR_REST_1440,
      clip: { top: 304.8, bottom: 900 },
      listHeight: 142, // Gradient-sized list, plenty of room below
    });
    expect(placement).toEqual({ mode: "inflow" });
  });

  it("fits above the clip box only → still inflow (the shared primitive flips up)", () => {
    // Gradient sub-dropdown at scrollTop 0: 558px above the trigger inside
    // the dialog body, 54px below — the shared flip lands it up, in-body.
    const placement = resolveListPlacement({
      trigger: { top: 703, bottom: 751, left: 541, width: 328 },
      clip: { top: 141, bottom: 809 },
      listHeight: 142,
      viewportHeight: 900,
    });
    expect(placement).toEqual({ mode: "inflow" });
  });
});

describe("resolveListPlacement — viewport escape (the HDR rest state)", () => {
  it("escapes BELOW at 1440×900 rest — the frame draws the list below the trigger", () => {
    const placement = resolveListPlacement(HDR_REST_1440);
    expect(placement).toEqual({
      mode: "escape",
      side: "below",
      top: 596.8, // trigger bottom 592.8 + the 4px list gap
      left: 541,
      width: 358,
    });
  });

  it("escapes ABOVE at 1024×768 rest — not enough viewport below, plenty above", () => {
    const placement = resolveListPlacement({
      trigger: { top: 478.8, bottom: 526.8, left: 333, width: 358 },
      clip: { top: 238.8, bottom: 579.3 },
      listHeight: 280,
      viewportHeight: 768,
    });
    expect(placement).toEqual({
      mode: "escape",
      side: "above",
      top: 194.8, // trigger top 478.8 − 4px gap − 280px list
      left: 333,
      width: 358,
    });
  });
});

describe("resolveListPlacement — last-resort cap (list taller than the viewport)", () => {
  it("caps on the roomier viewport side when neither fits", () => {
    const placement = resolveListPlacement({
      trigger: { top: 280, bottom: 328, left: 100, width: 358 },
      clip: { top: 0, bottom: 600 },
      listHeight: 280,
      viewportHeight: 600,
    });
    expect(placement).toEqual({ mode: "capped", side: "above", cap: 268 });
  });

  it("never caps below the two-row floor", () => {
    const placement = resolveListPlacement({
      trigger: { top: 30, bottom: 50, left: 100, width: 358 },
      clip: { top: 0, bottom: 100 },
      listHeight: 280,
      viewportHeight: 100,
    });
    expect(placement).toEqual({ mode: "capped", side: "below", cap: 96 });
  });
});
