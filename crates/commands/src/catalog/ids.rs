//! Command and event IDs: the stable kebab-case wire strings and their
//! mechanical Rust-variant mapping (doc 05 §2.1, §3.2 — card 1.D4).
//!
//! An ID is `<area>.<verb-noun>` — an area token, a dot, then one or
//! more kebab-case tokens. The Rust mapping is mechanical: split on `.`
//! and `-`, Pascal-case each token, concatenate (§2.1). IDs are
//! **stable forever within major version 1** (§6); the enums are
//! `#[non_exhaustive]` and hosts must map unknown IDs to
//! `invalid-argument` (senders) or a logged no-op (event receivers).
//!
//! The macro below generates, from one `(Variant, "wire-id")` table per
//! enum, the enum itself, `as_str`, `Display`, `FromStr` (rejecting
//! unknown ids with [`UnknownId`]) and an `ALL` slice — so the id list
//! exists in exactly one place and the spec table ( [`crate::catalog::spec`]
//! ) + the checksum consume the same data.

use std::fmt;
use std::str::FromStr;

/// Parsing an id that is not in the catalog (§2.1: unknown ids are
/// rejected, never guessed — senders map them to `invalid-argument`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownId(pub String);

impl fmt::Display for UnknownId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown catalog id `{}`", self.0)
    }
}

impl std::error::Error for UnknownId {}

/// Generates a stable-id enum plus `as_str`/`Display`/`FromStr`/`ALL`
/// from a `(Variant => "wire.id")` table.
macro_rules! stable_id_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident => $id:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[non_exhaustive]
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $variant,
            )+
        }

        impl $name {
            /// Every catalog entry, in doc 05 table order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// Every stable wire id, in doc 05 table order.
            pub const ALL_IDS: &'static [&'static str] = &[$($id),+];

            /// The stable wire id (§2.1/§3.2).
            pub const fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $id,)+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = UnknownId;

            /// Parse a stable wire id; unknown ids are an error (V1 —
            /// closed shapes; no guessing, no aliasing).
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $(if s == $id { return Ok($name::$variant); })+
                Err(UnknownId(s.to_owned()))
            }
        }
    };
}

stable_id_enum! {
    /// One v1 catalog command (doc 05 §2.1 — 61 entries + the issue #32
    /// bookmark trio; the §2.3–2.12 tables define each entry's payload,
    /// class and errors).
    ///
    /// The enum is metadata: [`crate::catalog::command::Command`] is the
    /// payload-carrying twin whose variants serialize under these ids.
    pub enum CommandId {
        // session (§2.3)
        SessionOpen => "session.open",
        SessionClose => "session.close",
        // playback (§2.4)
        PlaybackPlay => "playback.play",
        PlaybackPause => "playback.pause",
        PlaybackTogglePlayPause => "playback.toggle-play-pause",
        PlaybackStop => "playback.stop",
        PlaybackSeek => "playback.seek",
        PlaybackSetRate => "playback.set-rate",
        PlaybackStepFrame => "playback.step-frame",
        PlaybackSetPosition => "playback.set-position",
        // tracks (§2.5)
        TracksList => "tracks.list",
        TracksGetInfo => "tracks.get-info",
        TracksSetVideoTrack => "tracks.set-video-track",
        TracksSetAudioTrack => "tracks.set-audio-track",
        TracksSetSubtitleTrack => "tracks.set-subtitle-track",
        TracksSetSecondaryAudio => "tracks.set-secondary-audio",
        // audio (§2.6)
        AudioSetVolume => "audio.set-volume",
        AudioGetVolume => "audio.get-volume",
        AudioSetMute => "audio.set-mute",
        AudioListDevices => "audio.list-devices",
        AudioSetDevice => "audio.set-device",
        AudioSetDelay => "audio.set-delay",
        // video (§2.7)
        VideoSetFullscreen => "video.set-fullscreen",
        VideoSetAspectRatio => "video.set-aspect-ratio",
        VideoSetCrop => "video.set-crop",
        VideoSetZoom => "video.set-zoom",
        VideoSetDelay => "video.set-delay",
        VideoTakeScreenshot => "video.take-screenshot",
        // subtitles (§2.8)
        SubtitleAddFile => "subtitle.add-file",
        SubtitleSetSync => "subtitle.set-sync",
        SubtitleSetEncoding => "subtitle.set-encoding",
        SubtitleSetScale => "subtitle.set-scale",
        // playlist (§2.9)
        PlaylistEnqueue => "playlist.enqueue",
        PlaylistEnqueueMany => "playlist.enqueue-many",
        PlaylistPlayItem => "playlist.play-item",
        PlaylistNext => "playlist.next",
        PlaylistPrev => "playlist.prev",
        PlaylistSkip => "playlist.skip",
        PlaylistRemove => "playlist.remove",
        PlaylistMove => "playlist.move",
        PlaylistClear => "playlist.clear",
        PlaylistSetShuffle => "playlist.set-shuffle",
        PlaylistSetRepeat => "playlist.set-repeat",
        PlaylistSave => "playlist.save",
        PlaylistLoad => "playlist.load",
        // media information (§2.10)
        MediaGetInfo => "media.get-info",
        MediaGetCurrentTime => "media.get-current-time",
        MediaGetDuration => "media.get-duration",
        MediaGetChapters => "media.get-chapters",
        MediaSetChapter => "media.set-chapter",
        MediaGetBookmarks => "media.get-bookmarks",
        MediaAddBookmark => "media.add-bookmark",
        MediaRemoveBookmark => "media.remove-bookmark",
        // configuration (§2.11)
        ConfigGetSetting => "config.get-setting",
        ConfigSetSetting => "config.set-setting",
        ConfigResetSetting => "config.reset-setting",
        ConfigListAreas => "config.list-areas",
        // system (§2.12)
        SystemGetEngineInfo => "system.get-engine-info",
        SystemListPlugins => "system.list-plugins",
        SystemPluginEnable => "system.plugin-enable",
        SystemPluginDisable => "system.plugin-disable",
        SystemSetLogLevel => "system.set-log-level",
        SystemGetStateSnapshot => "system.get-state-snapshot",
        SystemListCommands => "system.list-commands",
    }
}

stable_id_enum! {
    /// One v1 catalog event (doc 05 §3.2 — 23 entries).
    ///
    /// `session.state-changed`, `timing.seek-completed`,
    /// `timing.seek-failed`, `playback.error`, `tracks.changed`,
    /// `tracks.list-ready` and `media.info-ready` are the completion
    /// evidence events for class-A commands (§1.4 C3).
    pub enum EventId {
        // session (§3.2)
        SessionStateChanged => "session.state-changed",
        SessionOpeningProgress => "session.opening-progress",
        SessionBuffering => "session.buffering",
        // timing (§3.2)
        TimingTimeTick => "timing.time-tick",
        TimingDurationKnown => "timing.duration-known",
        TimingSeekCompleted => "timing.seek-completed",
        TimingSeekFailed => "timing.seek-failed",
        // tracks (§3.2)
        TracksListReady => "tracks.list-ready",
        TracksChanged => "tracks.changed",
        // playback (§3.2)
        PlaybackEndOfTrack => "playback.end-of-track",
        PlaybackEndOfMedia => "playback.end-of-media",
        PlaybackError => "playback.error",
        PlaybackRateChanged => "playback.rate-changed",
        // audio (§3.2)
        AudioDeviceLost => "audio.device-lost",
        AudioDeviceChanged => "audio.device-changed",
        // video (§3.2)
        VideoScreenshotSaved => "video.screenshot-saved",
        // playlist (§3.2)
        PlaylistChanged => "playlist.changed",
        // config (§3.2)
        ConfigSettingChanged => "config.setting-changed",
        // plugins (§3.2)
        PluginsLoaded => "plugins.loaded",
        PluginsFailed => "plugins.failed",
        PluginsCrashed => "plugins.crashed",
        // system (§3.2)
        SystemLog => "system.log",
        // media (§3.2)
        MediaInfoReady => "media.info-ready",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §2.1's four worked examples of the mechanical id↔variant mapping.
    #[test]
    fn doc_05_examples_round_trip() {
        for (id, variant) in [
            ("session.open", CommandId::SessionOpen),
            (
                "playback.toggle-play-pause",
                CommandId::PlaybackTogglePlayPause,
            ),
            ("audio.set-volume", CommandId::AudioSetVolume),
            (
                "tracks.set-subtitle-track",
                CommandId::TracksSetSubtitleTrack,
            ),
        ] {
            assert_eq!(variant.as_str(), id);
            match id.parse::<CommandId>() {
                Ok(back) => assert_eq!(back, variant),
                Err(err) => unreachable!("known id rejected: {err}"),
            }
        }
    }

    #[test]
    fn every_id_round_trips_and_is_unique() {
        let mut seen: Vec<&str> = Vec::new();
        for variant in CommandId::ALL {
            let id = variant.as_str();
            assert_eq!(id.parse::<CommandId>().ok(), Some(*variant));
            assert!(!seen.contains(&id), "duplicate command id {id}");
            seen.push(id);
        }
        let mut seen: Vec<&str> = Vec::new();
        for variant in EventId::ALL {
            let id = variant.as_str();
            assert_eq!(id.parse::<EventId>().ok(), Some(*variant));
            assert!(!seen.contains(&id), "duplicate event id {id}");
            seen.push(id);
        }
    }

    #[test]
    fn catalog_sizes_match_doc_05() {
        // Doc 05 §2.1 lists 61 command entries (§3.2: 23 events); issue
        // #32 adds the media.*-bookmark trio (64 total).
        assert_eq!(CommandId::ALL.len(), 64);
        assert_eq!(EventId::ALL.len(), 23);
    }

    #[test]
    fn unknown_ids_are_rejected_not_guessed() {
        for bad in [
            "nope.nope",
            "session.open ",
            "SESSION.OPEN",
            "playback.seeky",
            "",
            "session",
        ] {
            assert!(bad.parse::<CommandId>().is_err(), "{bad} parsed");
            assert!(bad.parse::<EventId>().is_err(), "{bad} parsed");
        }
        match "bogus.command".parse::<CommandId>() {
            Ok(_) => unreachable!("bogus id accepted"),
            Err(err) => assert_eq!(err.to_string(), "unknown catalog id `bogus.command`"),
        }
    }
}
