// Card 4.C3 — playback UX states (doc 08 §5.3/§5.4): Loading, Buffering,
// Unsupported, Error, Empty + the StateRouter that maps engine/session state
// to the overlay over the 4.C1 canvas region. Honest by construction: state
// and progress come only from engine events; nothing renders as recovered or
// complete without engine evidence (doc 05 §1 C1–C8).

import "./states.css";

export { usePlayerEngineView, type PlayerEngineView } from "./context-bridge";
export {
  ERROR_ACTION_MESSAGE_KEYS,
  STAGE_MESSAGE_KEYS,
  UNSUPPORTED_DETAIL_KEYS,
  UNSUPPORTED_ERROR_CODES,
  isUnsupportedError,
  unsupportedFormatInfo,
  type UnsupportedDetailKey,
} from "./error-display";
export {
  BufferingOverlay,
  EmptyOverlay,
  ErrorOverlay,
  HomeOverlay,
  IdleSurface,
  LoadingOverlay,
  UnsupportedOverlay,
} from "./StageOverlays";
export {
  PlaylistBrowse,
  type BrowsePlaylistData,
  type PlaylistBrowseProps,
} from "./PlaylistBrowse";
export { StateRouter, resolveOverlay, type OverlayKind, type StateRouterProps } from "./StateRouter";
