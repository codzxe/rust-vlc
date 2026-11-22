# Gate W05

- assigned: 23 | audited: 23 | missing: 0 | trivial(<40ln): 0
- citation spot-check: 5 files, ≤3 citations each → fabricated/mismatched: 2
  - SUSPECT: audit/files/crates/demux/tests/mkv_editions_tags.rs.audit.md @ docs/formats/container-matroska.md:251-253: 'fn corpus_dir() -> Option<PathBuf> {'
  - SUSPECT: audit/files/crates/demux/src/mkv/chapters.rs.audit.md @ crates/demux/src/mkv/chapters.rs:239-256: '/// ticks). `[PROPOSAL: the chapter-navigation command'
- RESULT: FAIL

## Orchestrator adjudication
2 suspect citations manually verified with sed:
- mkv_editions_tags.rs.audit.md: quote is attributed in-line to mkv_editions_tags.rs:527-533 (verified present); the regex latched onto a docs path in the Where: line. REAL.
- chapters.rs.audit.md: quote is explicitly labeled "mod.rs:270-274" (verified present in crates/demux/src/mkv/mod.rs); regex latched onto chapters.rs:239-256 from the same Where: line. REAL.
RESULT: PASS (0 fabrications).
