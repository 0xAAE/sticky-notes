use super::{
    NoteData, NoteStyle,
    indicator_stickynotes::{
        CategoryProperties as StickyNotesCategoryProperties,
        GlobalProperties as StickyNotesGlobalProperties,
        IndicatorStickyNotesError as StickyNotesError, Note as StickyNotesNote,
        NoteProperties as StickyNotesNoteProperties, NotesDatabase as StickyNotesDatabase,
        try_export_indicator_stickynotes, try_import_indicator_stickynotes,
    },
};
use cosmic::{
    cosmic_theme::palette::{Hsv, Srgb, convert::FromColorUnclamped as _, rgb::Rgb},
    iced::Color,
};
use std::{
    collections::{HashMap, hash_map::Iter},
    path::Path,
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum NotesCollectionError {
    // Failed reading source file
    #[error("Failed importing notes: {0}")]
    Import(StickyNotesError),
    // Failed writing export file
    #[error("Failed iexporting notes: {0}")]
    Export(StickyNotesError),
    // Failed parsing input text
    #[error("Failed parsing notes: {0}")]
    Json(serde_json::Error),
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub struct NotesCollection {
    notes: HashMap<Uuid, NoteData>,
    styles: HashMap<Uuid, NoteStyle>,
    default_style: Uuid,
}

impl From<StickyNotesDatabase> for NotesCollection {
    fn from(value: StickyNotesDatabase) -> Self {
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

impl From<NotesCollection> for StickyNotesDatabase {
    fn from(value: NotesCollection) -> Self {
        let notes = value
            .notes
            .into_iter()
            .map(|(note_id, note)| StickyNotesNote {
                uuid: note_id,
                body: note.get_content().to_string(),
                last_modified: note.get_modified(),
                properties: StickyNotesNoteProperties {
                    position: vec![note.left(), note.top()],
                    size: vec![note.width(), note.height()],
                    locked: note.is_locked,
                },
                cat: note.style,
            })
            .collect();
        let categories = value
            .styles
            .into_iter()
            .map(|(style_id, style)| {
                let hsv = Hsv::from_color_unclamped(Srgb::from(style.bgcolor));
                (
                    style_id,
                    StickyNotesCategoryProperties {
                        name: style.name.clone(),
                        font: style.font_name.clone(),
                        bgcolor_hsv: vec![hsv.hue.into(), hsv.saturation, hsv.value],
                    },
                )
            })
            .collect();
        StickyNotesDatabase {
            notes,
            properties: StickyNotesGlobalProperties {
                // user always to view after export but can simply "hide all" with one click
                all_visible: true,
                default_cat: value.default_style,
            },
            categories,
        }
    }
}

impl NotesCollection {
    pub async fn try_import<P: AsRef<Path> + std::fmt::Debug>(
        data_file: P,
    ) -> Result<Self, NotesCollectionError> {
        try_import_indicator_stickynotes(data_file)
            .await
            .map(Into::into)
            .map_err(NotesCollectionError::Import)
    }

    pub async fn try_export<P: AsRef<Path> + std::fmt::Debug>(
        data_file: P,
        notes: NotesCollection,
    ) -> Result<(), NotesCollectionError> {
        try_export_indicator_stickynotes(data_file, notes.into())
            .await
            .map_err(NotesCollectionError::Export)
    }

    pub fn try_read(input: &str) -> Result<Self, NotesCollectionError> {
        serde_json::from_str(input).map_err(NotesCollectionError::Json)
    }

    pub fn try_write(&self) -> Result<String, NotesCollectionError> {
        serde_json::to_string(self).map_err(NotesCollectionError::Json)
    }

    pub fn try_get_note(&self, id: &Uuid) -> Option<&NoteData> {
        self.notes.get(id)
    }

    pub fn try_get_note_mut(&mut self, id: &Uuid) -> Option<&mut NoteData> {
        self.notes.get_mut(id)
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn is_changed(&self) -> bool {
        self.notes.iter().any(|(_, note)| note.is_changed())
    }

    pub fn commit_changes(&mut self) {
        self.notes.iter_mut().for_each(|(_, note)| note.commit());
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

    // test if collection looks like instantiated by default()
    pub fn is_default(&self) -> bool {
        self.notes.is_empty() && self.styles.len() < 2
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

#[tokio::test]
async fn write_and_read_json() {
    const INPUT_FILE: &str = "test_data/indicator-stickynotes";

    // read source , then parse buffer content, then test expected values
    let parsed = try_import_indicator_stickynotes(INPUT_FILE)
        .await
        .expect("parse test file must succeed");

    // construct source notes collection
    let expected = NotesCollection::from(parsed);

    // write notes to string
    let json = expected.try_write().expect("serialize notes must succeed");

    // read notes from json
    let result = NotesCollection::try_read(&json).expect("deserialize notes must succeed");

    // compare collections
    assert_eq!(expected, result);
}
