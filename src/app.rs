// SPDX-License-Identifier: MPL-2.0

use std::str::FromStr;
use thiserror::Error;
pub use {
    applet::AppletModel,
    service::{ServiceFlags, ServiceModel},
    utils::to_f32,
};

mod about_window;
mod applet;
mod edit_style;
mod restore_view;
mod service;
mod settings_view;
mod sticky_window;
mod styles_view;
mod utils;

const APP_ID: &str = "com.github.aae.notes";

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Command {
    Ping,
    Quit,
    LoadNotes,
    SaveNotes,
    ImportNotes,
    ExportNotes,
    ShowAllNotes,
    HideAllNotes,
    LockAll,
    RestoreNotes,
    OpenSettings,
    OpenAbout,
}

#[derive(Debug, Error)]
pub enum NotesAppError {
    // Failed reading source file
    #[error("Failed parsing command: {0}")]
    ParseError(String),
}

const PING: &str = "PING";
const QUIT: &str = "QUIT";
const LOAD: &str = "LOAD";
const SAVE: &str = "SAVE";
const IMPORT: &str = "IMPORT";
const EXPORT: &str = "EXPORT";
const SHOW: &str = "SHOW";
const HIDE: &str = "HIDE";
const LOCK: &str = "LOCK";
const RESTORE: &str = "RESTORE";
const SETTINGS: &str = "SETTINGS";
const ABOUT: &str = "ABOUT";

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Command::Ping => PING,
                Command::Quit => QUIT,
                Command::LoadNotes => LOAD,
                Command::SaveNotes => SAVE,
                Command::ImportNotes => IMPORT,
                Command::ExportNotes => EXPORT,
                Command::ShowAllNotes => SHOW,
                Command::HideAllNotes => HIDE,
                Command::LockAll => LOCK,
                Command::RestoreNotes => RESTORE,
                Command::OpenSettings => SETTINGS,
                Command::OpenAbout => ABOUT,
            }
        )
    }
}

impl FromStr for Command {
    type Err = NotesAppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            PING => Ok(Self::Ping),
            QUIT => Ok(Self::Quit),
            LOAD => Ok(Self::LoadNotes),
            SAVE => Ok(Self::SaveNotes),
            IMPORT => Ok(Self::ImportNotes),
            EXPORT => Ok(Self::ExportNotes),
            SHOW => Ok(Self::ShowAllNotes),
            HIDE => Ok(Self::HideAllNotes),
            LOCK => Ok(Self::LockAll),
            RESTORE => Ok(Self::RestoreNotes),
            SETTINGS => Ok(Self::OpenSettings),
            ABOUT => Ok(Self::OpenAbout),
            _ => Err(NotesAppError::ParseError(s.to_string())),
        }
    }
}
