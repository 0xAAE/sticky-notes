use std::{
    collections::{
        HashMap,
        hash_map::{Iter, IterMut},
    },
    path::Path,
};

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
    #[serde(skip)]
    is_dirty: bool,
    #[serde[skip]]
    deleted_notes: HashMap<Uuid, NoteData>,
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
            is_dirty: true,                // not saved yet
            deleted_notes: HashMap::new(), // no deleted yet
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
                    locked: note.is_locked(),
                },
                cat: note.style(),
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

    pub fn for_each_note_mut<F>(&mut self, f: F)
    where
        F: Fn(&mut NoteData),
    {
        self.notes.iter_mut().for_each(|(_, note)| f(note));
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn is_changed(&self) -> bool {
        self.is_dirty || self.notes.iter().any(|(_, note)| note.is_changed())
    }

    pub fn commit_changes(&mut self) {
        self.notes.values_mut().for_each(NoteData::commit);
        self.is_dirty = false;
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn get_all_notes(&self) -> Iter<'_, Uuid, NoteData> {
        self.notes.iter()
    }

    pub fn get_all_notes_mut(&mut self) -> IterMut<'_, Uuid, NoteData> {
        self.notes.iter_mut()
    }

    pub fn get_all_styles(&self) -> Iter<'_, Uuid, NoteStyle> {
        self.styles.iter()
    }

    pub fn get_style_names(&self) -> Vec<String> {
        self.get_all_styles()
            .map(|(_id, style)| style.name.clone())
            .collect()
    }

    pub fn try_get_note_style_index(&self, note_id: &Uuid) -> Option<usize> {
        self.try_get_note(note_id).and_then(|note| {
            self.get_all_styles()
                .enumerate()
                .find(|(_, (id, _))| **id == note.style())
                .map(|(index, (_, _))| index)
        })
    }

    pub fn try_set_note_style_by_index(&mut self, note_id: &Uuid, style_index: usize) -> bool {
        if let Some(style_id) = self.get_all_styles().nth(style_index).map(|(id, _)| *id) {
            self.try_get_note_mut(note_id)
                .map(|note| note.set_style(style_id))
                .is_some()
        } else {
            false
        }
    }

    pub fn new_note(&mut self) -> Uuid {
        let id = Uuid::new_v4();
        self.notes.insert(id, NoteData::new(self.default_style));
        id
    }

    pub fn delete_note(&mut self, note_id: Uuid) {
        if let Some((id, note)) = self.notes.remove_entry(&note_id) {
            self.is_dirty = true;
            self.deleted_notes.insert(id, note);
        }
    }

    pub fn get_all_deleted_notes(&self) -> Iter<'_, Uuid, NoteData> {
        self.deleted_notes.iter()
    }

    pub fn restore_deleted_note(&mut self, note_id: Uuid) -> Option<&NoteData> {
        if let Some((id, note)) = self.deleted_notes.remove_entry(&note_id) {
            self.is_dirty = true;
            self.notes.insert(id, note);
            self.notes.get(&id)
        } else {
            None
        }
    }

    pub fn default_style(&self) -> Option<&NoteStyle> {
        self.styles.get(&self.default_style)
    }

    pub fn default_style_index(&self) -> Option<usize> {
        self.default_style()
            .map(|style| &style.name)
            .and_then(|name| {
                self.styles
                    .iter()
                    .enumerate()
                    .find(|(_, (_, v))| &v.name == name)
                    .map(|(i, _)| i)
            })
    }

    pub fn try_set_default_style_by_index(&mut self, style_index: usize) -> bool {
        self.get_all_styles()
            .nth(style_index)
            .map(|(id, _)| *id)
            .map(|id| {
                if self.default_style != id {
                    self.default_style = id;
                    self.is_dirty = true;
                }
            })
            .is_some()
    }

    pub fn get_style(&self, style_id: &Uuid) -> Option<&NoteStyle> {
        self.styles.get(style_id)
    }

    pub fn try_get_note_style(&self, note_id: Uuid) -> Option<&NoteStyle> {
        // the first, search among live notes
        self.try_get_note(&note_id)
            .and_then(|note| self.get_style(&note.style()))
            // otherwise search in deleted notes
            .or_else(|| {
                self.get_all_deleted_notes()
                    .find(|(id, _)| **id == note_id)
                    .and_then(|(_, note)| self.get_style(&note.style()))
            })
            // at the end return default style
            .or_else(|| self.default_style())
    }

    // test if collection looks like instantiated by default()
    pub fn is_default_collection(&self) -> bool {
        self.notes.len() <= 1 && self.styles.len() <= 1
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
                // unwrap() also is safe enough here:
                .unwrap_or_else(Uuid::nil);
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
            is_dirty: false,
            deleted_notes: HashMap::new(),
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
    assert_eq!(expected.len(), result.len());
    assert_eq!(
        expected.get_all_deleted_notes().count(),
        result.get_all_deleted_notes().count()
    );
    assert_eq!(
        expected.get_all_notes().count(),
        result.get_all_notes().count()
    );
    assert_eq!(
        expected.get_all_styles().count(),
        result.get_all_styles().count()
    );
    assert_eq!(expected.default_style, result.default_style);
}

#[test]
fn create_read_update_delete_restore_operations() {
    let mut collection = NotesCollection::default();
    // initial note was automatically created
    assert_eq!(collection.len(), 1);
    // create note
    let note_id = collection.new_note();
    assert_eq!(collection.len(), 2);
    // get mutable ref to note
    let note_mut = collection.try_get_note_mut(&note_id);
    assert!(note_mut.is_some());
    let note_mut = note_mut.unwrap();
    // note is not changed
    assert!(!note_mut.is_changed());
    // set new text
    note_mut.set_content("test text".to_string());
    // note is changed
    assert!(note_mut.is_changed());
    // fix note changes
    note_mut.commit();
    // note is not changed again
    assert!(!note_mut.is_changed());
    // no deleted notes by default
    assert_eq!(collection.get_all_deleted_notes().count(), 0);
    // delete note
    collection.delete_note(note_id);
    // collection is changed
    assert!(collection.is_changed());
    // fix note changes
    collection.commit_changes();
    // collection is not changed
    assert!(!collection.is_changed());
    // can restore deleted note
    assert_eq!(collection.get_all_deleted_notes().count(), 1);
    // restore note
    let result = collection.restore_deleted_note(note_id);
    assert!(result.is_some());
    // collection is changed
    assert!(collection.is_changed());
    // no more notes to restore
    assert_eq!(collection.get_all_deleted_notes().count(), 0);
    // fix note changes
    collection.commit_changes();
    // collection is not changed
    assert!(!collection.is_changed());
}

#[test]
fn for_each_note_mut() {
    const NOTES_COUNT: usize = 10;

    let mut collection = NotesCollection::default();
    assert_eq!(collection.len(), 1); // assume auto created new note with default syle

    // fill up collection with new NOTES_COUNT notes
    for _ in 0..NOTES_COUNT {
        collection.new_note();
    }
    assert_eq!(collection.len(), NOTES_COUNT + 1);

    // test all of notes are visible
    assert!(
        collection
            .get_all_notes()
            .all(|(_, note)| note.is_visible())
    );

    // using for_eahc_not_mut() to hgide all notes
    collection.for_each_note_mut(|note: &mut NoteData| note.set_visibility(false));

    // test all of notes are hidden
    assert!(
        !collection
            .get_all_notes()
            .any(|(_, note)| note.is_visible())
    );
}
