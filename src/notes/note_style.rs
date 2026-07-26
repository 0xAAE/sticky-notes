use super::{DEF_NOTE_FONT_SIZE, DEF_NOTE_STYLE_NAME};
use cosmic::{cosmic_theme::palette::Srgb, iced::Color};
use serde::{Deserialize, Deserializer, Serializer, ser::SerializeTuple};

/// The style defines how to adjust font to display a text
#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug, Default, PartialEq)]
pub enum FontStyle {
    #[default]
    Default,
    Light,
    Semibold,
    Bold,
    Monospace,
}

impl std::fmt::Display for FontStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontStyle::Default => write!(f, "Default"),
            FontStyle::Light => write!(f, "Light"),
            FontStyle::Semibold => write!(f, "Semibold"),
            FontStyle::Bold => write!(f, "Bold"),
            FontStyle::Monospace => write!(f, "Monospace"),
        }
    }
}

/// The set of font parameters to display a text
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub struct Font {
    pub style: FontStyle,
    pub size: u16,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            style: FontStyle::default(),
            size: DEF_NOTE_FONT_SIZE,
        }
    }
}

/// The style to use when display a sticky note
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub struct NoteStyle {
    name: String,
    font: Font,
    #[serde(deserialize_with = "color_from_str", serialize_with = "color_to_str")]
    bgcolor: Color,
    #[serde(default)]
    is_dark: bool,
    #[serde(skip)]
    is_dirty: bool,
}

fn color_from_str<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let rgb: [f32; 3] = Deserialize::deserialize(deserializer)?;
    Ok(Color::from(rgb))
}

fn color_to_str<S>(value: &Color, serializer: S) -> Result<S::Ok, S::Error>
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
            font: Font::default(),
            bgcolor: Color::WHITE,
            is_dark: false,
            is_dirty: false,
        }
    }
}

impl NoteStyle {
    #[must_use]
    pub fn new(name: String, font: Font, bgcolor: Color) -> Self {
        Self {
            name,
            font,
            bgcolor,
            is_dark: false,
            is_dirty: false,
        }
    }

    #[must_use]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn get_font(&self) -> &Font {
        &self.font
    }

    #[must_use]
    pub fn is_light(&self) -> bool {
        !self.is_dark
    }

    #[must_use]
    pub fn get_background_color(&self) -> Color {
        self.bgcolor
    }

    pub fn set_name(&mut self, name: &str) {
        if self.name != name {
            tracing::debug!("(*) unsaved style: renamed {} into {name}", self.name);
            self.name = name.to_string();
            self.is_dirty = true;
        }
    }

    pub fn set_font(&mut self, font: Font) {
        if self.font != font {
            tracing::debug!("(*) unsaved style: font changed");
            self.font = font;
            self.is_dirty = true;
        }
    }

    pub fn set_background_color(&mut self, color: Color) {
        if self.bgcolor != color {
            tracing::debug!("(*) unsaved style: color changed");
            self.bgcolor = color;
            self.is_dirty = true;
        }
    }

    pub fn set_is_dark(&mut self, is_dark: bool) {
        if self.is_dark != is_dark {
            tracing::debug!("(*) unsaved style: is_dark changed");
            self.is_dark = is_dark;
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
