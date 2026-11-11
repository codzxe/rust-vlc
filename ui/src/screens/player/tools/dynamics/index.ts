// w26.c2 — dynamics tools public surface (player audio dynamics): the three
// mountable tool cards for the integration lane (tools dialog via
// ToolsDialogShell) and the Settings → Audio effects area. All state is
// local to each card; controls are honestly staged (see DynamicsCard).
export { CompressorTool } from "./CompressorTool";
export { SpatializerTool } from "./SpatializerTool";
export { StereoWidenerTool } from "./StereoWidenerTool";
export { DynamicsCard } from "./DynamicsCard";
export { ToolSliderRow } from "./ToolSliderRow";
export type { ToolSliderRowProps } from "./ToolSliderRow";
export type {
  DynamicsCardProps,
  DynamicsLabelKey,
  DynamicsRowDef,
  DynamicsRowState,
  DynamicsToolProps,
} from "./types";
