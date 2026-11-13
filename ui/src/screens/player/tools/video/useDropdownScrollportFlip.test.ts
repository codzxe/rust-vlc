// useDropdownScrollportFlip tests (w26.i08) — pins the clip-aware placement
// decision that closes h15's HDR NO (h15-P1 / g17-P1-1). The decision is a
// pure function measured against the geometry h15/g17 actually captured in
// the tools dialog; w26.k11 (j11 P3-1) additionally pins the hook's DOM half
// — applyPlacement/clearPlacement drive the escape/cap/clear writes directly
// (jsdom has no layout, so the hook's measure() bails at zero geometry; the
// extracted seam takes the live-measured geometries as inputs, making a
// selector rename or clear-order bug fail tests instead of passing).

import { afterEach, describe, expect, it } from "vitest";
import {
  applyPlacement,
  clearPlacement,
  resolveListPlacement,
} from "./useDropdownScrollportFlip";

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

describe("applyPlacement / clearPlacement — the DOM half (w26.k11, j11 P3-1)", () => {
  /** The wrapper + dropdown + trigger + list the hook owns (the shared
   * primitive's classes — the seam the writes target). */
  function mountDropdown(withList = true): {
    root: HTMLElement;
    dropdown: HTMLElement;
    list: HTMLElement | null;
  } {
    const root = document.createElement("div");
    const dropdown = document.createElement("div");
    dropdown.className = "lumen-dropdown";
    const trigger = document.createElement("button");
    trigger.className = "lumen-dropdown__trigger";
    const list = withList ? document.createElement("ul") : null;
    if (list !== null) list.className = "lumen-dropdown__list";
    dropdown.append(trigger, ...(list !== null ? [list] : []));
    root.appendChild(dropdown);
    document.body.appendChild(root);
    return { root, dropdown, list };
  }

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("escape writes the marker + fixed coordinates on the list (HDR rest 1440×900)", () => {
    const { root, dropdown, list } = mountDropdown();
    applyPlacement(root, dropdown, resolveListPlacement(HDR_REST_1440));
    expect(root.getAttribute("data-list-escape")).toBe("below");
    expect(root.getAttribute("data-list-direction")).toBeNull();
    expect(list?.style.position).toBe("fixed");
    expect(list?.style.top).toBe("596.8px"); // trigger bottom 592.8 + 4px gap
    expect(list?.style.bottom).toBe("auto"); // menu.css's up rule must not stretch it
    expect(list?.style.left).toBe("541px");
    expect(list?.style.width).toBe("358px");
  });

  it("escape above pins the list over the trigger (1024×768 rest)", () => {
    const { root, dropdown, list } = mountDropdown();
    applyPlacement(
      root,
      dropdown,
      resolveListPlacement({
        trigger: { top: 478.8, bottom: 526.8, left: 333, width: 358 },
        clip: { top: 238.8, bottom: 579.3 },
        listHeight: 280,
        viewportHeight: 768,
      }),
    );
    expect(root.getAttribute("data-list-escape")).toBe("above");
    expect(list?.style.position).toBe("fixed");
    expect(list?.style.top).toBe("194.8px");
    expect(list?.style.left).toBe("333px");
  });

  it("capped pins the direction marker + the cap var (no inline coordinates)", () => {
    const { root, dropdown, list } = mountDropdown();
    applyPlacement(root, dropdown, { mode: "capped", side: "above", cap: 268 });
    expect(root.getAttribute("data-list-direction")).toBe("up");
    expect(root.getAttribute("data-list-escape")).toBeNull();
    expect(root.style.getPropertyValue("--video-dropdown-cap")).toBe("268px");
    expect(list?.getAttribute("style") ?? "").not.toMatch(/position|top|left|width/);
  });

  it("inflow clears a previous escape — the shared flip owns the list again", () => {
    const { root, dropdown, list } = mountDropdown();
    applyPlacement(root, dropdown, resolveListPlacement(HDR_REST_1440));
    expect(root.getAttribute("data-list-escape")).toBe("below");
    applyPlacement(root, dropdown, { mode: "inflow" });
    expect(root.getAttribute("data-list-escape")).toBeNull();
    expect(root.getAttribute("data-list-direction")).toBeNull();
    expect(list?.getAttribute("style") ?? "").not.toMatch(/position|top|left|width/);
  });

  it("clearPlacement removes every marker, cap var and inline coordinate", () => {
    const { root, dropdown, list } = mountDropdown();
    applyPlacement(root, dropdown, { mode: "capped", side: "below", cap: 200 });
    applyPlacement(root, dropdown, resolveListPlacement(HDR_REST_1440));
    clearPlacement(root, dropdown);
    expect(root.getAttribute("data-list-escape")).toBeNull();
    expect(root.getAttribute("data-list-direction")).toBeNull();
    expect(root.style.getPropertyValue("--video-dropdown-cap")).toBe("");
    expect(list?.getAttribute("style") ?? "").not.toMatch(/position|top|bottom|left|width/);
  });

  it("a missing list writes nothing (a selector rename cannot crash or mark)", () => {
    const { root, dropdown } = mountDropdown(false);
    applyPlacement(root, dropdown, resolveListPlacement(HDR_REST_1440));
    applyPlacement(root, dropdown, { mode: "capped", side: "below", cap: 200 });
    expect(root.getAttribute("data-list-escape")).toBeNull();
    expect(root.getAttribute("data-list-direction")).toBeNull();
  });
});
