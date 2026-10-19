// Template-only build script (task 1.A1): registers the `lint_canary` cfg
// name so rustc's `unexpected_cfgs` lint does not warn about the cfg-gated
// canary module in `src/lint_canary.rs`. This is the ONLY reason a build
// script exists here. DELETE THIS FILE when copying the template: a plain
// Lumen crate must not carry a build script (build scripts are native-code
// audit suspects per doc 10 §2.3 / doc 11 §3).
fn main() {
    println!("cargo::rustc-check-cfg=cfg(lint_canary)");
}
