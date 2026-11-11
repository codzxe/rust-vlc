//! The `[web.size_budgets]` config plane (card 7.K1, issue #49; doc 09
//! §2.4): the wasm size budgets live in `product.toml` (validated by
//! `product.schema.json`), never as literals in the `cargo xtask
//! web-budget` gate's code. These tests pin the committed values and the
//! not-layer-overridable rule the typed model documents.

mod common;

use product_config::load_partial_str;

#[test]
fn committed_budgets_match_doc_09_section_2_4_proposal() {
    let product = match common::load_root_product() {
        Ok(product) => product,
        Err(diags) => unreachable!("the committed root product.toml must load: {diags:?}"),
    };
    let web = match product.web {
        Some(web) => web,
        None => unreachable!(
            "the committed product.toml must carry [web.size_budgets] — the web-budget gate \
             (doc 09 §2.4) reads its ceilings from there and errors loudly when absent"
        ),
    };
    // Deliberate pin: 3 MiB / 6 MiB are doc 09 §2.4's baselines, still
    // [PROPOSAL] pending doc 09 §12 Q5. Changing product.toml's budgets
    // without the doc 09 §2.4 review that owns the number fails here on
    // purpose — update this pin in the same commit as the reviewed change.
    assert_eq!(
        web.size_budgets.phase2_vertical_max_mib, 3,
        "Phase-2 vertical baseline (doc 09 §2.4)"
    );
    assert_eq!(
        web.size_budgets.full_p0_max_mib, 6,
        "full-P0 codec-set baseline (doc 09 §2.4)"
    );
}

#[test]
fn budgets_respect_the_ordering_invariant() {
    let product = match common::load_root_product() {
        Ok(product) => product,
        Err(diags) => unreachable!("the committed root product.toml must load: {diags:?}"),
    };
    let Some(web) = product.web else {
        unreachable!("the committed product.toml must carry [web.size_budgets]")
    };
    // The schema documents this cross-field rule as enforced by the
    // web-budget gate (product.schema.json's full_p0_max_mib $comment);
    // it must hold on the committed values themselves.
    assert!(
        web.size_budgets.full_p0_max_mib >= web.size_budgets.phase2_vertical_max_mib,
        "the full-P0 ceiling must not sit below the Phase-2 vertical ceiling \
         (every full-P0 build is also a Phase-2 superset — doc 09 §2.4)"
    );
}

#[test]
fn layers_cannot_override_web_budgets() {
    let schema = common::root_schema();
    let src = "[web.size_budgets]\nphase2_vertical_max_mib = 1\nfull_p0_max_mib = 1\n";
    let diags = match load_partial_str("layer.toml", src, &schema) {
        Ok(_) => Vec::new(),
        Err(diags) => diags,
    };
    assert!(
        !diags.is_empty(),
        "a layer document carrying [web] must fail loudly, not override: budgets are a \
         product decision, not a user preference (model.rs; doc 07 §4.4) — got {diags:?}"
    );
}
