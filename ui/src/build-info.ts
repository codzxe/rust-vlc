// Build-injected package identity (card 4.E4 About panel).
// vite.config.ts reads ui/package.json at config load and `define`s these
// constants — the only path by which the manifest's name/version reach UI
// code without escaping ui/src (doc 08 §8 boundary) and without a hardcoded
// literal that can drift from the package it ships in.
// Types: ./build-info.globals.d.ts.

export const UI_PACKAGE_NAME: string = __UI_PACKAGE_NAME__;
export const UI_PACKAGE_VERSION: string = __UI_PACKAGE_VERSION__;
