//! Golden test: the doc 07 §3.5 example diagnostic, reproduced from the
//! `range-probe-max-bytes.toml` fixture (probe_max_bytes = 1048576).
//!
//! The document's mandatory fields — location `file:key.path`, `value:`,
//! `rule:` (units expanded), `hint:` — are asserted byte-for-byte where they
//! are machine-generated; the wrapped hint prose is asserted
//! whitespace-normalized (the doc's manual line wrapping is not normative,
//! its content is).

mod common;

#[test]
fn probe_max_bytes_renders_the_doc_07_s35_example() {
    let name = "range-probe-max-bytes.toml";
    let src = common::read_fixture(name);
    let line = match common::marker_line(&src, "probe_max_bytes = 1048576") {
        Some(l) => l,
        None => unreachable!("fixture must contain the mutated key"),
    };
    let diags = match common::load_fixture_as("product.toml", name) {
        Ok(_) => unreachable!("{name} must be rejected"),
        Err(d) => d,
    };
    let hits = common::for_key(&diags, "limits.probe_max_bytes");
    assert_eq!(hits.len(), 1, "{}", product_config::render_all(&diags));
    let rendered = match hits.first() {
        Some(d) => d.to_string(),
        None => unreachable!("unreachable: length asserted above"),
    };

    // Location line: doc 07 §3.5 `file:key.path`, extended with the line
    // number (1.B1 contract: file + line + field path).
    let first = rendered.lines().next().unwrap_or_default();
    assert_eq!(
        first,
        format!("error[config]: product.toml:{line}:limits.probe_max_bytes")
    );

    // §3.5 value line (found), units expanded.
    assert!(
        rendered.contains("  value: 1048576 (1.0 MiB)\n"),
        "{rendered}"
    );
    // §3.5 rule line: the constraint with units expanded for humans.
    assert!(
        rendered.contains("  rule:  must be >= 16777216 (16 MiB) and <= 67108864 (64 MiB)\n"),
        "{rendered}"
    );
    // 1.B1 expected line.
    assert!(
        rendered.contains("  expected: integer in 16777216 (16 MiB)..=67108864 (64 MiB)\n"),
        "{rendered}"
    );
    // §3.5 hint: the doc's exact prose (whitespace-normalized).
    let normalized: String = rendered.split_whitespace().collect::<Vec<_>>().join(" ");
    for fragment in [
        "hint: probing below 16 MiB rejects valid multi-track files;",
        "if you need a lower bound for an embedded target,",
        "change the schema range via review",
        "(docs/07-config-driven-system.md \u{a7}3.3), not this value.",
    ] {
        let frag_norm: String = fragment.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(normalized.contains(&frag_norm), "{rendered}");
    }
}
