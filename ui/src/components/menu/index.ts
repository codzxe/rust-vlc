/** Card 4.B3 — menu primitives barrel. */
export { MenuBar } from "./MenuBar";
export type { MenuBarProps, MenuBarMenu } from "./MenuBar";
export { Dropdown, Select } from "./Dropdown";
export type { DropdownProps, DropdownOption } from "./Dropdown";
export { ContextMenu } from "./ContextMenu";
export type { ContextMenuProps } from "./ContextMenu";
export { Tooltip } from "./Tooltip";
export type { TooltipProps } from "./Tooltip";
export { MenuItem } from "./MenuItem";
export type { MenuItemProps } from "./MenuItem";
// w26.i16 — the Profile flyout (Profiles.png §A10): local-store rows +
// selected-check grammar + the "+ New" quick-create row.
export {
  buildProfileFlyout,
  createProfile,
  selectProfile,
  resetProfilesForTests,
  useMediaProfiles,
} from "./ProfilesFlyout";
export type {
  MediaProfile,
  ProfilesPersistence,
  ProfilesState,
} from "./ProfilesFlyout";
export type {
  MenuDensity,
  MenuEntry,
  MenuEntryBase,
  MenuSeparator,
  ResolveLabel,
} from "./types";
