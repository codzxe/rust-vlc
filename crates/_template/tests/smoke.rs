//! Smoke test for the crate template (task 1.A1) — proves the template
//! builds and its public surface works inside the workspace. Replace with
//! real tests when copying the template.

#[test]
fn template_marker_is_stable() {
    assert_eq!(template::template_marker(), "lumen-crate-template");
}
