use super::{DEF_NOTE_STYLE_FONT, DEF_NOTE_STYLE_NAME};
use cosmic::{cosmic_theme::palette::Srgb, iced::Color};
use serde::{Deserialize, Deserializer, Serializer, ser::SerializeTuple};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub struct NoteStyle {
    pub name: String,
    pub font_name: String,
    #[serde(
        deserialize_with = "deserialize_from_str",
        serialize_with = "serialize_to_str"
    )]
    pub bgcolor: Color,
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
        }
    }
}
