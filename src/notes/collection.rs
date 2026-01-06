use super::{NoteData, NoteStyle, import};
use cosmic::{
    cosmic_theme::palette::{Hsv, convert::FromColorUnclamped as _, rgb::Rgb},
    iced::Color,
};
use std::{
    collections::{HashMap, hash_map::Iter},
    path::Path,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NotesCollection {
    notes: HashMap<Uuid, NoteData>,
    styles: HashMap<Uuid, NoteStyle>,
    default_style: Uuid,
}

impl From<import::NotesDatabase> for NotesCollection {
    fn from(value: import::NotesDatabase) -> Self {
        // import notes data
        let notes = value
            .notes
            .into_iter()
            .map(|src| {
                (
                    src.uuid,
                    NoteData::new_from_import(src, value.properties.all_visible),
                )
            })
            .collect();
        // import note styles
        let styles: HashMap<Uuid, NoteStyle> = value
            .categories
            .into_iter()
            .map(|(id, cat)| {
                let hsv = match cat.bgcolor_hsv.get(0..3) {
                    Some([h, s, v]) => Hsv::new_srgb(*h, *s, *v),
                    Some([h, s]) => Hsv::new_srgb(*h, *s, Hsv::<f32>::min_value()),
                    Some([h]) => {
                        Hsv::new_srgb(*h, Hsv::<f32>::min_saturation(), Hsv::<f32>::min_value())
                    }
                    _ => Hsv::default(),
                };
                let rgb = Rgb::from_color_unclamped(hsv).into_components();
                (
                    id,
                    NoteStyle {
                        name: cat.name,
                        font_name: cat.font,
                        bgcolor: Color::from_rgb(rgb.0, rgb.1, rgb.2),
                    },
                )
            })
            .collect();
        // finalize notes collection
        let mut instance = Self {
            notes,
            styles,
            default_style: value.properties.default_cat,
        };
        // ensure default_style is correct
        instance.ensure_default_style();
        instance
    }
}

impl NotesCollection {
    pub async fn try_import<P: AsRef<Path> + std::fmt::Debug>(
        data_file: P,
    ) -> anyhow::Result<Self> {
        Ok(import::try_import_indicator_stickynotes(data_file)
            .await?
            .into())
    }

    pub fn try_get_note(&self, id: &Uuid) -> Option<&NoteData> {
        self.notes.get(id)
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn get_all_notes(&self) -> Iter<'_, Uuid, NoteData> {
        self.notes.iter()
    }

    pub fn new_note(&mut self) -> Uuid {
        let id = Uuid::new_v4();
        self.notes.insert(id, NoteData::new(self.default_style));
        id
    }

    pub fn get_style_or_default(&self, style_id: &Uuid) -> Option<&NoteStyle> {
        self.styles
            .get(style_id)
            .or_else(|| self.styles.get(&self.default_style))
    }

    fn ensure_default_style(&mut self) {
        // ensure default_style is correct
        if !self.styles.contains_key(&self.default_style) {
            // ensure at least one style is in the collection
            if self.styles.is_empty() {
                self.styles.insert(Uuid::new_v4(), NoteStyle::default());
            }
            // select random default style whish exists:
            self.default_style = self
                .styles
                .keys()
                .next()
                .copied()
                // unwrap() also is safe here:
                .unwrap_or_else(Uuid::new_v4);
        }
    }
}

impl Default for NotesCollection {
    fn default() -> Self {
        // instantiat edefault note style
        let default_style = Uuid::new_v4();
        let styles = HashMap::from_iter([(default_style, NoteStyle::default())]);
        // create note with default style
        let notes = HashMap::from_iter([(Uuid::new_v4(), NoteData::new(default_style))]);
        Self {
            notes,
            styles,
            default_style,
        }
    }
}
