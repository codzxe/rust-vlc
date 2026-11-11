// w26.c5 — dev-only vision harness: the mounted ToolsDialog (Audio tab and
// Video tab variants) for the contract §5 comparison loop. Never imported by
// routes or app code (the w26.c2 harness pattern); harness.test.tsx exports
// standalone HTML (CSS inlined) when W26C5_HARNESS_OUT is set, for
// headless screenshotting.

import { ToolsDialog } from "./ToolsDialog";
import type { ToolsDialogTab } from "./ToolsDialog";

/** One harness page: the tools dialog open on the requested tab. */
export function ToolsDialogHarness({ tab }: { tab: ToolsDialogTab }) {
  return (
    <div className="tools-dialog-harness" data-harness-tab={tab}>
      <ToolsDialog open onClose={() => undefined} defaultTab={tab} />
    </div>
  );
}
