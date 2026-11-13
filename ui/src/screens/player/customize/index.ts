// w26.gfixA (issue #62) — the customize surface barrel (the modal rebuild).
// w26.i05 — the consumer half joins it (toolbarItems + the live layout hook).
// w26.k08 (j08 finding #4) — the retired gating helpers (tileVisible/
// visibleTiles) left the barrel with their capability-gate policy.
export { CustomizePanels } from "./CustomizePanels";
export type { CustomizePanelsProps } from "./CustomizePanels";
export { CustomizeToolbarItems } from "./toolbarItems";
export { useCommandCapabilities } from "./capabilities";
export type { CommandCapabilities } from "./capabilities";
export type { CycleSelections, SetCycleSelections } from "./cycleState";
export {
  applyCycleCompletion,
  applyCycleEvent,
  applyCyclePayload,
  CYCLE_PAYLOADS,
  deepEqual,
} from "./cycleState";
export {
  CUSTOMIZE_TILE_IDS,
  CUSTOMIZE_TILES,
  DEFAULT_TOOLBAR_LAYOUT,
  cycleArgs,
  tileById,
} from "./panels";
export {
  TOOLBAR_CHANGED_EVENT,
  TOOLBAR_STORAGE_KEY,
  bandTiles,
  clearPersistedToolbarLayout,
  loadToolbarLayout,
  parseToolbarLayout,
  persistToolbarLayout,
  toolbarLayoutEqual,
  useToolbarLayout,
} from "./toolbarLayout";
export type {
  CommandArgs,
  CustomizeControl,
  CustomizeLabelKey,
  CustomizeOption,
  CustomizeTile,
  ToolbarLayout,
} from "./types";
