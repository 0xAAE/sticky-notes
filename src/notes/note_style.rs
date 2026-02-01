use super::{DEF_NOTE_STYLE_FONT, DEF_NOTE_STYLE_NAME};
use cosmic::{cosmic_theme::palette::Srgb, iced::Color};
use serde::{Deserialize, Deserializer, Serializer, ser::SerializeTuple};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub struct NoteStyle {
    name: String,
    font_name: String,
    #[serde(
        deserialize_with = "deserialize_from_str",
        serialize_with = "serialize_to_str"
    )]
    bgcolor: Color,
    #[serde(skip)]
    is_dirty: bool,
}

fn deserialize_from_str<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let rgb: [f32; 3] = Deserialize::deserialize(deserializer)?;
    Ok(Color::from(rgb))
}

fn serialize_to_str<S>(value: &Color, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rgb: [f32; 3] = Srgb::from(*value).into();
    let mut serialize_array = serializer.serialize_tuple(3)?;
    for v in rgb {
        serialize_array.serialize_element(&v)?;
    }
    serialize_array.end()
}

impl Default for NoteStyle {
    fn default() -> Self {
        Self {
            name: DEF_NOTE_STYLE_NAME.to_string(),
            font_name: DEF_NOTE_STYLE_FONT.to_string(),
            bgcolor: Color::WHITE,
            is_dirty: false,
        }
    }
}

impl NoteStyle {
    #[must_use]
    pub fn new(name: String, font_name: String, bgcolor: Color) -> Self {
        Self {
            name,
            font_name,
            bgcolor,
            is_dirty: false,
        }
    }

    #[must_use]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn get_font_name(&self) -> &str {
        &self.font_name
    }

    #[must_use]
    pub fn get_background_color(&self) -> Color {
        self.bgcolor
    }

    pub fn set_name(&mut self, name: &str) {
        if self.name != name {
            self.name = name.to_string();
            self.is_dirty = true;
        }
    }

    pub fn set_font_name(&mut self, font_name: &str) {
        if self.font_name != font_name {
            self.font_name = font_name.to_string();
            self.is_dirty = true;
        }
    }

    pub fn set_background_color(&mut self, color: Color) {
        if self.bgcolor != color {
            self.bgcolor = color;
            self.is_dirty = true;
        }
    }

    #[must_use]
    pub fn is_changed(&self) -> bool {
        self.is_dirty
    }

    pub fn commit(&mut self) {
        self.is_dirty = false;
    }
}
