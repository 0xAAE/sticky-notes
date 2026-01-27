// SPDX-License-Identifier: MPL-2.0

use cosmic::{
    cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    iced::Size,
};

use crate::app::to_f32;

const DEF_DATA_FILE: &str = ".config/indicator-stickynotes";
const ICON_SIZE: u16 = 16;

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    pub import_file: String,
    pub notes: String,
    pub restore_notes_width: usize,
    pub restore_notes_heigth: usize,
    pub toolbar_icon_size: u16,
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
            restore_notes_width: 480,
            restore_notes_heigth: 400,
            toolbar_icon_size: ICON_SIZE,
        }
    }
}

impl Config {
    pub fn restore_notes_size(&self) -> Size {
        Size::new(
            to_f32(self.restore_notes_width),
            to_f32(self.restore_notes_heigth),
        )
    }
}
