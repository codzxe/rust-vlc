pub fn title() -> String {
    // lumen-allow-literal: temp fixture prefix, test-only
    "lumen-backpressure-1-2s.wav".to_string()
}
pub fn smuggle() -> &'static str {
    "see // lumen-allow-literal: fake marker inside a string — Lumen"
}
#[cfg(test)]
mod helper2;
#[cfg(test)]
pub mod helper;
