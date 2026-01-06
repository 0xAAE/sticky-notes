use super::{DEF_NOTE_STYLE_FONT, DEF_NOTE_STYLE_NAME};
use cosmic::iced::Color;

#[derive(Clone, Debug)]
pub struct NoteStyle {
    pub name: String,
    pub font_name: String,
    pub bgcolor: Color,
}

impl Default for NoteStyle {
    fn default() -> Self {
        Self {
            name: DEF_NOTE_STYLE_NAME.to_string(),
            font_name: DEF_NOTE_STYLE_FONT.to_string(),
            bgcolor: Color::WHITE,
        }
    }
}
