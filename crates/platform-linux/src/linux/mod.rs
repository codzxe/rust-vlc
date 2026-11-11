//! The Linux integrations (cfg-gated to `target_os = "linux"`): the
//! PulseAudio client, the DRM sysfs scanner, the xdg dialog chain.
//! On any other OS every one of these is replaced by the typed
//! fallbacks in the seam modules (the crate still compiles and its
//! capability report says `WrongOs` honestly).

#[cfg(target_os = "linux")]
pub mod dl;
#[cfg(target_os = "linux")]
pub mod drm;
#[cfg(target_os = "linux")]
pub mod pulse;
#[cfg(target_os = "linux")]
pub mod pulse_ffi;
#[cfg(target_os = "linux")]
pub mod xdg;
