// w26.c4 — video tools lane (player video tools: Colors / Crop / Essential /
// HDR, frames "Video - Colors.svg", "Video - Crop.svg", "Video - Essential.
// svg", "HDR Options.svg"). Genuinely new keys (CONTRACT §4.5). The Lead
// registers these into ui/src/i18n/messages.ts after the wave merge; until
// then the components resolve them seam-first (central catalog → this file)
// via the useVideoMsg resolver — the w26.a17/c2 pattern — so the copy renders
// identically before and after registration.
//
// NOT duplicated here (reused through the same seam):
//   * card-toggle labels — w26.a18's player.tools.expand-settings /
//     player.tools.collapse-settings (also central since a18 registered them);
//   * tool names — central customize.panel.video-colors ("Video colors"),
//     customize.field.crop ("Crop"), customize.panel.essential ("Essential"),
//     customize.panel.hdr ("HDR options");
//   * Essential's Hue/Brightness/Contrast/Saturation/Gamma rows and Colors'
//     Saturation row — central customize.field.* keys.
export const keys = {
  // Colors rows (frame: Video - Colors.svg).
  "player.tools.colors.extraction": "Color Extraction",
  "player.tools.colors.threshold": "Color Threshold",
  "player.tools.colors.similarity": "Similarity",
  "player.tools.colors.negate": "Negate Colors",
  "player.tools.colors.posterize": "Posterize",
  "player.tools.colors.gradient": "Gradient",
  "player.tools.colors.color": "Color",
  "player.tools.colors.cartoon": "Cartoon",
  "player.tools.colors.sepia": "Sepia Intensity",
  // Gradient mode sub-dropdown (frame shows the closed trigger labelled
  // "Gradient"; the option set is VLC's gradient filter modes [VERIFY engine
  // lane] — the frame exports no open state).
  "player.tools.gradient.mode": "Gradient mode",
  "player.tools.gradient.gradient": "Gradient",
  "player.tools.gradient.edge": "Edge",
  "player.tools.gradient.hemboss": "Hemboss",
  // Crop rows (frame: Video - Crop.svg — Top/Bottom and Left/Right pairs).
  "player.tools.crop.top": "Top",
  "player.tools.crop.bottom": "Bottom",
  "player.tools.crop.left": "Left",
  "player.tools.crop.right": "Right",
  "player.tools.crop.unit": "px",
  "player.tools.crop.link-vertical": "Link top and bottom",
  "player.tools.crop.link-horizontal": "Link left and right",
  // Essential rows (frame: Video - Essential.svg) — the three beyond the
  // central customize.field.* set.
  "player.tools.essential.sharpen-sigma": "Sharpen Sigma",
  "player.tools.essential.banding-radius": "Banding Removal Radius",
  "player.tools.essential.grain-variance": "Film Grain Variance",
  // HDR mode dropdown (frame: HDR Options.svg — 6 options, no leading icons).
  "player.tools.hdr.mode": "HDR mode",
  "player.tools.hdr.auto": "Auto",
  "player.tools.hdr.off": "Off",
  "player.tools.hdr.hdr10": "HDR10",
  "player.tools.hdr.hdr10-plus": "HDR10+",
  "player.tools.hdr.dolby-vision": "Dolby Vision",
  "player.tools.hdr.hlg": "HLG",
  // Staged-tool note — the honest-unbound marker (effects-area pattern,
  // ui/src/screens/settings/areas/effects.tsx + the w26.c2 dynamics twin):
  // visible, inert of engine claims, names the command layer it awaits.
  "player.tools.video.staged":
    "Staged — controls activate with the video effects command card",
} as const;
