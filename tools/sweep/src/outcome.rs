//! The sweep's outcome vocabulary and per-class tallies (card 7-d).
//!
//! # Outcome taxonomy (the card's six classes)
//!
//! One of six honest per-file outcomes, plus `Unknown` for a worker
//! that exited without its marker line (a tool bug — counted, never
//! hidden):
//!
//! | Outcome        | Worker status | Meaning                                     |
//! |----------------|---------------|---------------------------------------------|
//! | `Ok`           | `ok`          | EndOfStream reached, decoders drained.      |
//! | `Unsupported`  | `unsupported` | Typed `ErrorKind::Unsupported` (first       |
//! |                |               | vertical: non-AVI container, no decoder).   |
//! | `DirtyReject`  | `corrupt`     | Typed `ErrorKind::Corrupt` — the file's own |
//! |                |               | damage refused/recovered-fatally.           |
//! | `TypedError`   | `error`       | Any other typed `EngineError` (Inaccessible,|
//! |                |               | Network, non-bitstream Decoder, Device,     |
//! |                |               | ResourceLimit — e.g. concealment budget).   |
//! | `Panic`        | (supervisor)  | Worker died: abort/signal/exit without the  |
//! |                |               | marker line; panic message captured.        |
//! | `Timeout`      | (supervisor)  | Per-file deadline exceeded; worker killed.  |
//!
//! The mapping is engine-side (what the engine DID), independent of
//! the corpus taxonomy (what the file IS): the report layer
//! cross-tabs outcome x corpus class, so e.g. a clean-class
//! `TypedError` is a candidate engine bug while a garbage-class
//! `DirtyReject` is the engine doing its job.

use std::collections::BTreeMap;
use std::path::Path;

/// How one file's sweep ended (the card's classification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// Decoded to end of stream, limbs drained.
    Ok,
    /// Typed `ErrorKind::Unsupported` — honest "not in this vertical".
    Unsupported,
    /// Typed `ErrorKind::Corrupt` — damage refused or fatally recovered.
    DirtyReject,
    /// Any other typed `EngineError`.
    TypedError,
    /// Worker process died (abort/signal/no marker); message captured.
    Panic,
    /// Per-file deadline exceeded; worker killed by the supervisor.
    Timeout,
    /// Worker exited without a marker line (tool bug — count it).
    Unknown,
}

impl Outcome {
    /// Every outcome, tally-column order.
    pub const ALL: [Outcome; 7] = [
        Outcome::Ok,
        Outcome::Unsupported,
        Outcome::DirtyReject,
        Outcome::TypedError,
        Outcome::Panic,
        Outcome::Timeout,
        Outcome::Unknown,
    ];

    /// The stable column name (also the report vocabulary).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Ok => "ok",
            Outcome::Unsupported => "unsupported",
            Outcome::DirtyReject => "dirty-reject",
            Outcome::TypedError => "typed-error",
            Outcome::Panic => "panic",
            Outcome::Timeout => "timeout",
            Outcome::Unknown => "unknown",
        }
    }

    /// Map a worker status field to its outcome.
    #[must_use]
    pub fn from_worker_status(status: &str) -> Option<Outcome> {
        match status {
            "ok" => Some(Outcome::Ok),
            "unsupported" => Some(Outcome::Unsupported),
            "corrupt" => Some(Outcome::DirtyReject),
            "error" => Some(Outcome::TypedError),
            _ => None,
        }
    }
}

/// One parsed worker marker line (the `SWEEP\t...` protocol).
#[derive(Debug, Clone, Default)]
pub struct WorkerLine {
    /// Worker status: `ok` | `unsupported` | `corrupt` | `error`.
    pub status: String,
    /// `PipelineStage` the file stopped at (`-` for `ok`).
    pub stage: String,
    /// Short `ErrorKind` tag (`unsupported`/`corrupt`/`inaccessible`/
    /// `network`/`decoder`/`device`/`resource-limit`; `-` for `ok`).
    pub kind: String,
    /// Stable dotted context code (`-` for `ok`).
    pub code: String,
    /// Video packets fed / video frames decoded.
    pub v_pkts: u64,
    pub v_dec: u64,
    /// Audio packets fed / audio frames decoded.
    pub a_pkts: u64,
    pub a_dec: u64,
    /// Per-frame damage concealed (video / audio) — the damage census.
    pub v_concealed: u64,
    pub a_concealed: u64,
    /// Demux `Recovered` events (container-level resyncs).
    pub demux_recovered: u64,
    /// Tracks the demuxer exposed (coverage truth vs the 2 limbs).
    pub tracks: u64,
    /// Which limbs decoded: `v+a` | `v` | `a` | `none`.
    pub limbs: String,
    /// The container the probe named.
    pub container: String,
    /// The error `Display` (tabs/newlines sanitized; `-` for `ok`).
    pub detail: String,
}

/// Parse the worker's `SWEEP\t...` marker line (14 fixed fields,
/// detail last so its spaces survive). Returns `None` on a
/// malformed line (counted as `Unknown` — never guessed).
#[must_use]
pub fn parse_worker_line(line: &str) -> Option<WorkerLine> {
    let rest = line.strip_prefix("SWEEP\t")?;
    let mut fields = rest.split('\t');
    let mut next = || fields.next().map(str::to_string);
    let status = next()?;
    let stage = next()?;
    let kind = next()?;
    let code = next()?;
    let v_pkts = next()?.parse().ok()?;
    let v_dec = next()?.parse().ok()?;
    let a_pkts = next()?.parse().ok()?;
    let a_dec = next()?.parse().ok()?;
    let v_concealed = next()?.parse().ok()?;
    let a_concealed = next()?.parse().ok()?;
    let demux_recovered = next()?.parse().ok()?;
    let tracks = next()?.parse().ok()?;
    let limbs = next()?;
    let container = next()?;
    // Detail is the rest of the line (may be empty or contain spaces).
    let detail = fields.next().unwrap_or_default().to_string();
    if fields.next().is_some() {
        return None;
    }
    Some(WorkerLine {
        status,
        stage,
        kind,
        code,
        v_pkts,
        v_dec,
        a_pkts,
        a_dec,
        v_concealed,
        a_concealed,
        demux_recovered,
        tracks,
        limbs,
        container,
        detail,
    })
}

/// The corpus-v4 class of a file, from its name (the generator's
/// `NNNN-<class>.<ext>` naming; see docs/corpus manifests — the
/// manifest's `class` field equals `<class>` here). Falls back to
/// the fixture naming (`name__variant` → `name`), then the stem,
/// then `other`.
#[must_use]
pub fn class_of(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("other");
    // `NNNN-class...` — at least three leading digits then '-'.
    let digits = stem.chars().take_while(char::is_ascii_digit).count();
    if digits >= 3 && stem[digits..].starts_with('-') {
        return stem[digits + 1..].to_string();
    }
    if let Some((class, _)) = stem.split_once("__") {
        return class.to_string();
    }
    stem.to_string()
}

/// Classify a finished worker process from its protocol outputs:
/// the parsed marker line (if the worker got far enough to print
/// one), whether the supervisor killed it on the deadline, and the
/// raw exit code (`None` = killed by a signal, e.g. our kill, or an
/// abort).
#[must_use]
pub fn classify(marker: Option<&WorkerLine>, timed_out: bool, exit_code: Option<i32>) -> Outcome {
    if let Some(line) = marker {
        // A parsed marker with an unrecognized status field is a
        // protocol bug, not an engine outcome — count it honestly.
        return Outcome::from_worker_status(&line.status).unwrap_or(Outcome::Unknown);
    }
    if timed_out {
        return Outcome::Timeout;
    }
    match exit_code {
        // Exited "successfully" but printed no marker: tool bug.
        Some(0) => Outcome::Unknown,
        // Non-zero exit or signal death without a marker line: the
        // engine panicked or aborted — the census's target.
        _ => Outcome::Panic,
    }
}

/// The crash note for a panic/unknown outcome: the `panicked at`
/// line when Rust's default panic hook wrote one, else the stderr
/// tail + the exit fact. Sanitized for the line protocol.
#[must_use]
pub fn crash_note(stderr: &str, exit_code: Option<i32>) -> String {
    for line in stderr.lines() {
        if line.contains("panicked at") {
            return sanitize(line);
        }
    }
    let exit = match exit_code {
        Some(code) => format!("exit={code}"),
        None => "killed-by-signal".to_string(),
    };
    let tail: String = stderr.chars().rev().take(200).collect::<Vec<_>>().into_iter().rev().collect();
    if tail.trim().is_empty() {
        exit
    } else {
        sanitize(&format!("{exit} {tail}"))
    }
}

/// Sanitize a free-text field for the tab-separated protocol
/// (tabs and newlines would corrupt the line shape).
#[must_use]
pub fn sanitize(text: &str) -> String {
    let clean: String = text
        .chars()
        .map(|c| if c == '\t' || c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let clean = clean.trim();
    if clean.is_empty() {
        "-".to_string()
    } else {
        clean.chars().take(400).collect()
    }
}

/// Per-class outcome tallies (BTreeMap: deterministic print order).
#[derive(Debug, Default)]
pub struct Tally {
    rows: BTreeMap<String, [u64; 7]>,
    total: u64,
}

impl Tally {
    /// The empty tally (`Default::default()`, spelled for style).
    #[must_use]
    pub fn new() -> Tally {
        Tally::default()
    }

    /// Record one file's outcome under its class.
    pub fn record(&mut self, class: &str, outcome: Outcome) {
        let row = self.rows.entry(class.to_string()).or_insert([0; 7]);
        row[outcome as usize] += 1;
        self.total += 1;
    }

    /// Grand total of files recorded.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.total
    }

    /// The per-outcome grand totals (column order `Outcome::ALL`).
    #[must_use]
    pub fn outcome_totals(&self) -> [u64; 7] {
        let mut totals = [0; 7];
        for row in self.rows.values() {
            for (i, n) in row.iter().enumerate() {
                totals[i] += n;
            }
        }
        totals
    }

    /// Render the summary tables (per-class matrix + outcome totals)
    /// — the census skeleton; later sessions append the report prose.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("=== SWEEP SUMMARY ===\n");
        out.push_str(&format!("total files: {}\n", self.total()));
        for (i, n) in self.outcome_totals().iter().enumerate() {
            out.push_str(&format!(
                "{:<14} {}\n",
                Outcome::ALL[i].as_str(),
                n
            ));
        }
        out.push_str("\nper-class matrix (class: ok unsupported dirty-reject typed-error panic timeout unknown total)\n");
        for (class, row) in &self.rows {
            let class_total: u64 = row.iter().sum();
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                class,
                row[0],
                row[1],
                row[2],
                row[3],
                row[4],
                row[5],
                row[6],
                class_total
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_mapping_is_total_for_worker_statuses() {
        assert_eq!(Outcome::from_worker_status("ok"), Some(Outcome::Ok));
        assert_eq!(
            Outcome::from_worker_status("unsupported"),
            Some(Outcome::Unsupported)
        );
        assert_eq!(
            Outcome::from_worker_status("corrupt"),
            Some(Outcome::DirtyReject)
        );
        assert_eq!(
            Outcome::from_worker_status("error"),
            Some(Outcome::TypedError)
        );
        assert_eq!(Outcome::from_worker_status("panic"), None);
    }

    #[test]
    fn parses_a_full_ok_line() {
        let line = "SWEEP\tok\t-\t-\t-\t10\t10\t4\t4\t1\t0\t2\t2\tv+a\tavi\t-";
        let parsed = parse_worker_line(line);
        assert!(parsed.is_some(), "parse failed");
        let Some(parsed) = parsed else { return; };
        assert_eq!(parsed.status, "ok");
        assert_eq!(parsed.v_pkts, 10);
        assert_eq!(parsed.a_dec, 4);
        assert_eq!(parsed.v_concealed, 1);
        assert_eq!(parsed.limbs, "v+a");
        assert_eq!(parsed.container, "avi");
        assert_eq!(parsed.detail, "-");
    }

    #[test]
    fn parses_an_error_line_with_spaces_in_detail() {
        let line = "SWEEP\terror\tDecode\tdecoder\tdemux.fatal\t0\t0\t0\t0\t0\t0\t0\t0\tnone\tavi\tboom boom boom";
        let parsed = parse_worker_line(line);
        assert!(parsed.is_some(), "parse failed");
        let Some(parsed) = parsed else { return; };
        assert_eq!(parsed.status, "error");
        assert_eq!(parsed.detail, "boom boom boom");
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(parse_worker_line("SWEEP").is_none());
        assert!(parse_worker_line("NOTSWEEP\tok").is_none());
        assert!(parse_worker_line("SWEEP\tok").is_none()); // missing fields
    }

    #[test]
    fn class_of_handles_corpus_and_fixture_naming() {
        assert_eq!(
            class_of(Path::new("/tmp/x/0042-uhd4k-mjpeg.avi")),
            "uhd4k-mjpeg"
        );
        assert_eq!(class_of(Path::new("fixture__variant3.avi")), "fixture");
        assert_eq!(class_of(Path::new("plainfile.avi")), "plainfile");
        assert_eq!(class_of(Path::new("12-abbr.avi")), "12-abbr"); // <3 digits: stem
    }

    #[test]
    fn sanitize_kills_field_breaking_chars() {
        assert_eq!(sanitize("a\tb\nc"), "a b c");
        assert_eq!(sanitize("   "), "-");
    }

    #[test]
    fn classify_maps_worker_and_crash_paths() {
        let mut ok_line = WorkerLine {
            status: "ok".to_string(),
            ..WorkerLine::default()
        };
        assert_eq!(classify(Some(&ok_line), false, Some(0)), Outcome::Ok);
        ok_line.status = "bogus".to_string();
        assert_eq!(classify(Some(&ok_line), false, Some(0)), Outcome::Unknown);
        assert_eq!(classify(None, true, None), Outcome::Timeout);
        assert_eq!(classify(None, false, Some(101)), Outcome::Panic);
        assert_eq!(classify(None, false, None), Outcome::Panic);
        assert_eq!(classify(None, false, Some(0)), Outcome::Unknown);
    }

    #[test]
    fn crash_note_finds_panic_line_or_exit_fact() {
        let stderr = "thread 'main' panicked at src/worker.rs:42:5:\nslice index";
        assert_eq!(
            crash_note(stderr, Some(101)),
            "thread 'main' panicked at src/worker.rs:42:5:"
        );
        assert_eq!(crash_note("", None), "killed-by-signal");
        assert_eq!(crash_note("", Some(6)), "exit=6");
    }

    #[test]
    fn tally_records_and_renders() {
        let mut tally = Tally::new();
        tally.record("avi-video", Outcome::Ok);
        tally.record("avi-video", Outcome::Panic);
        tally.record("truncated", Outcome::DirtyReject);
        assert_eq!(tally.total(), 3);
        let totals = tally.outcome_totals();
        assert_eq!(totals[Outcome::Ok as usize], 1);
        assert_eq!(totals[Outcome::Panic as usize], 1);
        let rendered = tally.render();
        assert!(rendered.contains("avi-video\t1\t0\t0\t0\t1\t0\t0\t2"));
    }
}
