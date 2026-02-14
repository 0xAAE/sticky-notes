// SPDX-License-Identifier: MPL-2.0
use cosmic::{
    cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    iced::Size,
};

use crate::app::to_f32;

const DEF_DATA_FILE: &str = ".config/indicator-stickynotes";
const DEF_SERVICE_BIN: &str = "/usr/local/bin/notes-service";
const ICON_SIZE: u16 = 16;

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    pub import_file: String,
    pub notes: String,
    pub service_bin: String,
    pub restore_notes_width: usize,
    pub restore_notes_height: usize,
    pub edit_style_width: usize,
    pub edit_style_height: usize,
    pub about_width: usize,
    pub about_height: usize,
    pub toolbar_icon_size: u16,
    pub note_min_width: usize,
    pub note_min_height: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            import_file: dirs_next::home_dir().map_or_else(
                || DEF_DATA_FILE.to_string(),
                |mut home| {
                    home.push(DEF_DATA_FILE);
                    home.display().to_string()
                },
            ),
            notes: String::new(),
            service_bin: DEF_SERVICE_BIN.to_string(),
            restore_notes_width: 480,
            restore_notes_height: 400,
            edit_style_width: 480,
            edit_style_height: 800,
            about_width: 480,
            about_height: 840,
            note_min_width: 64,
            note_min_height: 64,
            toolbar_icon_size: ICON_SIZE,
        }
    }
}

impl Config {
    #[must_use]
    pub fn restore_notes_size(&self) -> Size {
        Size::new(
            to_f32(self.restore_notes_width),
            to_f32(self.restore_notes_height),
        )
    }

    #[must_use]
    pub fn edit_style_size(&self) -> Size {
        Size::new(
            to_f32(self.edit_style_width),
            to_f32(self.edit_style_height),
        )
    }

    #[must_use]
    pub fn about_size(&self) -> Size {
        Size::new(to_f32(self.about_width), to_f32(self.about_height))
    }

    #[must_use]
    pub fn sticky_window_min_width(&self) -> usize {
        self.note_min_width
    }

    #[must_use]
    pub fn sticky_window_min_height(&self) -> usize {
        self.note_min_height
    }
}
