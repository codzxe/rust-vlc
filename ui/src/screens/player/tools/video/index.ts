// w26.c4 — video tools public surface (player video tools: Colors / Crop /
// Essential / HDR): the four mountable tools for the integration lane (tools
// dialog via ToolsDialogShell body) and the Settings → Video effects area.
// All state is local to each tool; controls are honestly staged (see
// VideoCard / HdrTool). Cards are fluid-width — frame-exact at 360px hosts
// (the HDR dropdown at 362px).
export { ColorsTool } from "./ColorsTool";
export { CropTool } from "./CropTool";
export { EssentialTool } from "./EssentialTool";
export { HdrTool } from "./HdrTool";
export { VideoCard } from "./VideoCard";
export { VideoSliderRow } from "./VideoSliderRow";
export type { VideoSliderRowProps } from "./VideoSliderRow";
export { useVideoMsg } from "./copy";
export { FRAME_EXTRACTION_COLOR, FRAME_THRESHOLD_COLOR, isHexColor } from "./frameData";
export type {
  CropAxis,
  HdrToolProps,
  VideoCardProps,
  VideoLabelKey,
  VideoSliderRowDef,
  VideoSliderRowState,
  VideoToolProps,
} from "./types";
