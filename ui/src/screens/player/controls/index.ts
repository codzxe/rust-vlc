// Card 4.C2 owns ui/src/screens/player/controls/** (transport controls:
// play/pause/stop, seekbar, volume, track menus — every control invoking a
// real command per doc 08 §5).
export { ControlBar } from "./ControlBar";
export {
  controlCommandOf,
  mapTransportControls,
  seekDisableReason,
  type PlayButtonState,
  type PlayGlyph,
  type PlayerControlCommand,
  type SeekControlState,
  type SeekDisableReason,
  type StopButtonState,
  type TransportControlStates,
  type TransportMappingInputs,
} from "./state-mapping";
