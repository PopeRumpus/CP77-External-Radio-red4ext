//! CP77 External Radio — ported to red4ext-rs 0.11 for Cyberpunk 2077 patch 2.5.
//!
//! Exposes two global native functions to redscript, `play()` and `pause()`,
//! which drive whatever app currently owns Windows' system media controls
//! (Spotify, VLC, browsers, ...).
//!
//! Original plugin by DrJackieBright (MIT). The WinRT calls block until the
//! media app answers, so they run on a short-lived worker thread instead of
//! the game's script thread; the old version blocked the game while waiting.

use std::thread;

use red4ext_rs::{
    Exportable, GlobalExport, Plugin, PluginOps, SemVer, U16CStr, export_plugin_symbols, exports,
    global, log, wcstr,
};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as MediaSessions;

pub struct ExternalRadio;

impl Plugin for ExternalRadio {
    const AUTHOR: &'static U16CStr = wcstr!("DrJackieBright");
    const NAME: &'static U16CStr = wcstr!("CP77-External-Radio");
    const VERSION: SemVer = SemVer::new(2, 1, 15);

    fn exports() -> impl Exportable {
        exports![
            GlobalExport(global!(c"play", play)),
            GlobalExport(global!(c"pause", pause)),
        ]
    }
}

export_plugin_symbols!(ExternalRadio);

#[derive(Clone, Copy)]
enum Action {
    Play,
    Pause,
}

impl Action {
    fn name(self) -> &'static str {
        match self {
            Action::Play => "play",
            Action::Pause => "pause",
        }
    }
}

fn run(action: Action) -> windows::core::Result<bool> {
    let manager = MediaSessions::RequestAsync()?.get()?;
    let session = manager.GetCurrentSession()?;
    match action {
        Action::Play => session.TryPlayAsync()?.get(),
        Action::Pause => session.TryPauseAsync()?.get(),
    }
}

/// Starts the request off the game thread and returns immediately.
fn dispatch(action: Action) -> bool {
    let spawned = thread::Builder::new()
        .name(format!("external-radio-{}", action.name()))
        .spawn(move || match run(action) {
            Ok(true) => {}
            Ok(false) => log::warn!(
                ExternalRadio::env(),
                "{}: the media app declined the request",
                action.name()
            ),
            // No media app open is the common case; GetCurrentSession then errors.
            Err(err) => log::warn!(ExternalRadio::env(), "{} failed: {}", action.name(), err),
        });
    spawned.is_ok()
}

fn play() -> bool {
    dispatch(Action::Play)
}

fn pause() -> bool {
    dispatch(Action::Pause)
}
