// w26.b11 — playlist subtitle flows public surface (the add-subtitles
// dialog + its pieces). Integration recipe for the Lead lives in
// docs/waves/w26/logs/w26.b11.md; in short (inside the /playlist tree):
//
//   const [subsOpen, setSubsOpen] = useState(false);
//   …trigger: <Button onClick={() => setSubsOpen(true)}>Add subtitles</Button>
//   <AddSubtitlesDialog
//     open={subsOpen}
//     onClose={() => setSubsOpen(false)}
//     mediaNames={entries.map((e) => e.title ?? sourceLocation(e))}
//     candidates={discoveredSubtitleFiles}   // host seam (folder scan / search)
//     folderPath={["Videos", "Shows", "The Wand", "S01"]}
//     onSave={(result) => { /* apply/record — no engine command yet */ }}
//   />
export { AddSubtitlesDialog } from "./AddSubtitlesDialog";
export type {
  AddSubtitlesDialogProps,
  AddSubtitlesResult,
  SubtitleDestination,
  SubtitleSource,
} from "./AddSubtitlesDialog";
export { FilenameMatchControl } from "./FilenameMatchControl";
export type { FilenameMatchControlProps } from "./FilenameMatchControl";
export { MatchedPairsList } from "./MatchedPairsList";
export type { MatchedPairsListProps } from "./MatchedPairsList";
export { SubtitlePickList } from "./SubtitlePickList";
export type { SubtitlePickListProps } from "./SubtitlePickList";
export { SubtitlePickRow } from "./SubtitlePickRow";
export type { SubtitlePickRowProps } from "./SubtitlePickRow";
export { SubtitleFileGlyph, MediaFileGlyph, CycleGlyph } from "./SubtitleGlyphs";
export type { GlyphProps } from "./SubtitleGlyphs";
export {
  DEFAULT_MATCH_MODE,
  MATCH_MODES,
  isSubtitleFile,
  matchesMode,
  normalizeStem,
  pairSubtitles,
  splitExtension,
  titleOfStem,
} from "./matching";
export type { MatchMode, SubtitlePair } from "./matching";
export { useSubtitlesMsg } from "./useSubtitlesMsg";
export type { SubtitlesLaneKey } from "./useSubtitlesMsg";
