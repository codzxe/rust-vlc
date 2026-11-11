// ToolsDialog (w26.c5) — the player's audio/video TOOLS dialog: the
// wave-c tool cards (w26.c1–c4) mounted into ToolsDialogShell (w26.a18)
// behind the two tabs the player frames' tool families define —
//
//   AUDIO (frames "Audio - Equalizer" + "EQ presets", "Audio - Compressor
//   Settings", "Audio - Spatializer Settings", "Audio - Stereo Widener",
//   "Audio Visualizer")  → Equalizer (+ its presets panel) · Compressor ·
//   Spatializer · Stereo Widener · Visualizer
//   VIDEO (frames "Video - Colors", "Video - Crop", "Video - Essential",
//   "HDR Options")                                        → Colors · Crop ·
//   Essential · HDR
//
// Every card ships COLLAPSED per the frames (the "Expand Settings" toggle
// grammar); all state is local to the tools; the dynamics/visualizer/video
// cards carry their own honest staged markers. The EqualizerTool (w26.c1)
// has no staged marker of its own — the wrapper below carries the same
// honest-unbound line the sibling cards render (role="note" +
// data-pending-command naming the awaited `audio.set-eq` command).
//
// THE TRIGGER SEAM (decoupled contract, never rename — the
// `lumen:open-media` pattern): any surface asks for the tools dialog by
// calling `openToolsDialog(tab)` or dispatching
// `window.dispatchEvent(new CustomEvent("lumen:open-tools", { detail: tab }))`.
// `ToolsDialogHost` (this module) owns the listener and mounts the dialog
// per-open. The player-side affordance — ControlBar's gear / app-menu entry
// and/or a DIALOG_KEYMAP row (VLC parity: Ctrl+E, ui_action "open-tools") —
// lives in PlayerScreen/ControlBar (not this lane's files); the one-line
// wiring is recorded in docs/waves/w26/logs/w26.c5.md for the Lead.

import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { Tab, Tabs, TabsList, TabsPanel } from "../../../components/list";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.c5";
import { EqualizerTool } from "../tools/equalizer";
import { CompressorTool, SpatializerTool, StereoWidenerTool } from "../tools/dynamics";
import { VisualizerTool } from "../tools/visualizer";
import { ColorsTool, CropTool, EssentialTool, HdrTool } from "../tools/video";
import { ToolsDialogShell } from "./ToolsDialogShell";
import type { SurfaceCloseReason } from "../../../components/surface/Surface";
import "./dialogs.css";

/**
 * The cross-surface CustomEvent name (see module header — never rename).
 * Same contract shape as `lumen:open-media` (OpenMediaDialog).
 */
export const TOOLS_DIALOG_EVENT = "lumen:open-tools";

/** The dialog's two tabs — the player frames' tool families. */
export type ToolsDialogTab = "audio" | "video";

/** The tab ids in frame-family order. */
export const TOOLS_DIALOG_TABS: readonly ToolsDialogTab[] = ["audio", "video"];

/** Normalize an event detail / prop to a tab id (unknown → audio). */
const asTab = (value: unknown): ToolsDialogTab => (value === "video" ? "video" : "audio");

/**
 * Open the tools dialog from any surface — the trigger seam ControlBar /
 * PlayerScreen wire to their gear/menu affordances (see module header).
 */
export function openToolsDialog(tab: ToolsDialogTab = "audio"): void {
  window.dispatchEvent(new CustomEvent<ToolsDialogTab>(TOOLS_DIALOG_EVENT, { detail: tab }));
}

type LaneKey = keyof typeof laneKeys;

/** The lane catalog (this lane's keys only — the tools resolve their own). */
const laneCatalog: Readonly<Record<string, string>> = { ...laneKeys };

/** Keys resolvable by `useToolsMsg`. */
export type ToolsDialogLabelKey = MessageKey | LaneKey;

/** useToolsMsg — useMsg widened with the w26.c5 lane keys (the w26.c1
 * useEqMsg seam-first pattern: the central catalog wins once the Lead
 * registers the keys; the lane copy is the fallback reference). */
export function useToolsMsg() {
  const m = useMsg();
  return (key: ToolsDialogLabelKey): string => m(key as MessageKey) ?? laneCatalog[key];
}

export interface ToolsDialogProps {
  /** Whether the dialog is mounted and trapped. */
  open: boolean;
  /** Dismissal callback (Escape, scrim click, header ✕ — the shell's). */
  onClose: (reason: SurfaceCloseReason) => void;
  /** Initially selected tab (uncontrolled). @default "audio" */
  defaultTab?: ToolsDialogTab;
}

/**
 * The tools dialog — ToolsDialogShell chrome + the Audio/Video tab pair
 * hosting all eight wave-c tools (tab selection is the Tabs primitive's:
 * roving tabindex, arrows/Home/End, aria-selected). Mount per-open like the
 * other player dialogs (fresh tool state each open):
 * `{open ? <ToolsDialog …/> : null}`.
 */
export function ToolsDialog({
  open,
  onClose,
  defaultTab = "audio",
}: ToolsDialogProps): ReactElement {
  const m = useToolsMsg();
  return (
    <ToolsDialogShell open={open} onClose={onClose} title={m("player.tools.dialog.title")}>
      <Tabs defaultValue={defaultTab} className="tools-dialog__tabs">
        <TabsList className="tools-dialog__tablist">
          {TOOLS_DIALOG_TABS.map((id) => (
            <Tab key={id} value={id}>
              {m(id === "audio" ? "player.tools.tab.audio" : "player.tools.tab.video")}
            </Tab>
          ))}
        </TabsList>
        {/* AUDIO — the five audio tool cards, frame order. */}
        <TabsPanel value="audio" className="tools-dialog__panel">
          <div
            className="tools-dialog__staged-tool"
            data-staged-tool="equalizer"
            data-pending-command="audio.set-eq"
          >
            <EqualizerTool />
            <p className="tools-dialog__staged" role="note">
              {m("player.tools.equalizer.staged")}
            </p>
          </div>
          <CompressorTool />
          <SpatializerTool />
          <StereoWidenerTool />
          <VisualizerTool />
        </TabsPanel>
        {/* VIDEO — the four video tool cards, frame order. */}
        <TabsPanel value="video" className="tools-dialog__panel">
          <ColorsTool />
          <CropTool />
          <EssentialTool />
          <HdrTool />
        </TabsPanel>
      </Tabs>
    </ToolsDialogShell>
  );
}

/**
 * ToolsDialogHost — the listener half of the trigger seam: mounts the tools
 * dialog per-open when any surface dispatches {@link TOOLS_DIALOG_EVENT}
 * (via {@link openToolsDialog}). Host it once beside the other player
 * dialogs — PlayerScreen's one-line mount (recorded in the w26.c5 log):
 * `<ToolsDialogHost />`.
 */
export function ToolsDialogHost(): ReactElement | null {
  const [openTab, setOpenTab] = useState<ToolsDialogTab | null>(null);

  useEffect(() => {
    const onOpenTools = (event: Event): void => {
      setOpenTab(asTab((event as CustomEvent<ToolsDialogTab>).detail));
    };
    window.addEventListener(TOOLS_DIALOG_EVENT, onOpenTools);
    return () => window.removeEventListener(TOOLS_DIALOG_EVENT, onOpenTools);
  }, []);

  if (openTab === null) return null;
  // Keyed by the requested tab: a NEW open request (e.g. openToolsDialog
  // "video" while the audio tab is up) remounts the per-open dialog on the
  // requested tab — fresh tool state, the same per-open contract.
  return (
    <ToolsDialog key={openTab} open defaultTab={openTab} onClose={() => setOpenTab(null)} />
  );
}
