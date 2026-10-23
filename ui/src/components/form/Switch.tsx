/**
 * Switch — the same two-state control as the button family's `Toggle`
 * (doc 08 §1.5 has a single Toggle.svg; §4.5 assigns toggles the `switch`
 * role). Re-exported under its ARIA name for form-row composition so call
 * sites read naturally next to Checkbox/Radio. No behavioral duplication.
 */
export { Toggle as Switch } from "../button";
export type { ToggleProps as SwitchProps } from "../button";
