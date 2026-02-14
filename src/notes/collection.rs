use std::{
    collections::{
        HashMap,
        hash_map::{Iter, IterMut},
    },
    path::Path,
};

use super::{
    Font, NoteData, NoteStyle,
    indicator_stickynotes::{
        CategoryProperties as StickyNotesCategoryProperties,
        GlobalProperties as StickyNotesGlobalProperties,
        IndicatorStickyNotesError as StickyNotesError, Note as StickyNotesNote,
        NoteProperties as StickyNotesNoteProperties, NotesDatabase as StickyNotesDatabase,
        parse_font, serialize_font, try_export_indicator_stickynotes,
        try_import_indicator_stickynotes,
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
    #[error("Failed exporting notes: {0}")]
    Export(StickyNotesError),
    // Failed parsing input text
    #[error("Failed parsing notes: {0}")]
    Json(serde_json::Error),
    // must not delete the last (and default) style
    #[error("Cannot delete the last style")]
    DeleteLastStyle,
    #[error("Style {0} is not found")]
    StyleNotFound(Uuid),
    #[error("Style index {0} not found")]
    StyleIndexNotFound(usize),
    #[error("Note {0} is not found")]
    NoteNotFound(Uuid),
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
                    NoteStyle::new(
                        cat.name,
                        parse_font(&cat.font),
                        Color::from_rgb(rgb.0, rgb.1, rgb.2),
                    ),
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
                let hsv = Hsv::from_color_unclamped(Srgb::from(style.get_background_color()));
                (
                    style_id,
                    StickyNotesCategoryProperties {
                        name: style.get_name().to_string(),
                        font: serialize_font(style.get_font()),
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

#[allow(clippy::missing_errors_doc)]
impl NotesCollection {
    // Import/export/save/load

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

    // Collection as itself

    pub fn is_unsaved(&self) -> bool {
        self.is_dirty
            || self.notes.values().any(NoteData::is_changed)
            || self.styles.values().any(NoteStyle::is_changed)
    }

    // test if collection looks like instantiated by default()
    #[must_use]
    pub fn is_default_collection(&self) -> bool {
        self.notes.len() <= 1 && self.styles.len() <= 1
    }

    pub fn commit_changes(&mut self) {
        self.notes.values_mut().for_each(NoteData::commit);
        self.styles.values_mut().for_each(NoteStyle::commit);
        self.is_dirty = false;
    }

    // operations with notes

    #[must_use]
    pub fn get_notes_count(&self) -> usize {
        self.notes.len()
    }

    pub fn try_get_note(&self, note_id: &Uuid) -> Result<&NoteData, NotesCollectionError> {
        self.notes
            .get(note_id)
            .ok_or(NotesCollectionError::NoteNotFound(*note_id))
    }

    pub fn try_get_note_mut(
        &mut self,
        note_id: &Uuid,
    ) -> Result<&mut NoteData, NotesCollectionError> {
        self.notes
            .get_mut(note_id)
            .ok_or(NotesCollectionError::NoteNotFound(*note_id))
    }

    pub fn for_each_note_mut<F>(&mut self, f: F)
    where
        F: Fn(&mut NoteData),
    {
        self.notes.values_mut().for_each(f);
    }

    #[must_use]
    pub fn iter_notes(&self) -> Iter<'_, Uuid, NoteData> {
        self.notes.iter()
    }

    pub fn iter_notes_mut(&mut self) -> IterMut<'_, Uuid, NoteData> {
        self.notes.iter_mut()
    }

    #[must_use]
    pub fn iter_deleted_notes(&self) -> Iter<'_, Uuid, NoteData> {
        self.deleted_notes.iter()
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

    pub fn try_restore_deleted_note(
        &mut self,
        note_id: Uuid,
    ) -> Result<&NoteData, NotesCollectionError> {
        if let Some((id, note)) = self.deleted_notes.remove_entry(&note_id) {
            self.is_dirty = true;
            self.notes.insert(id, note);
            self.notes
                .get(&id)
                .ok_or(NotesCollectionError::NoteNotFound(note_id))
        } else {
            Err(NotesCollectionError::NoteNotFound(note_id))
        }
    }

    // operations with styles

    #[must_use]
    pub fn get_styles_count(&self) -> usize {
        self.styles.len()
    }

    #[must_use]
    pub fn iter_styles(&self) -> Iter<'_, Uuid, NoteStyle> {
        self.styles.iter()
    }

    #[must_use]
    pub fn get_style_names(&self) -> Vec<String> {
        self.styles
            .values()
            .map(|style| style.get_name().to_string())
            .collect()
    }

    pub fn try_get_default_style(&self) -> Result<&NoteStyle, NotesCollectionError> {
        self.styles
            .get(&self.default_style)
            .ok_or(NotesCollectionError::StyleNotFound(self.default_style))
    }

    pub fn try_get_default_style_index(&self) -> Result<usize, NotesCollectionError> {
        self.styles
            .keys()
            .enumerate()
            .find_map(|(index, style_id)| (*style_id == self.default_style).then_some(index))
            .ok_or(NotesCollectionError::StyleNotFound(self.default_style))
    }

    pub fn try_set_default_style_by_index(
        &mut self,
        style_index: usize,
    ) -> Result<(), NotesCollectionError> {
        self.styles
            .keys()
            .nth(style_index)
            .map(|id| {
                if self.default_style != *id {
                    self.default_style = *id;
                    self.is_dirty = true;
                }
            })
            .ok_or(NotesCollectionError::StyleIndexNotFound(style_index))
    }

    pub fn try_get_style(&self, style_id: &Uuid) -> Result<&NoteStyle, NotesCollectionError> {
        self.styles
            .get(style_id)
            .ok_or(NotesCollectionError::StyleNotFound(*style_id))
    }

    pub fn try_get_style_mut(
        &mut self,
        style_id: &Uuid,
    ) -> Result<&mut NoteStyle, NotesCollectionError> {
        self.styles
            .get_mut(style_id)
            .ok_or(NotesCollectionError::StyleNotFound(*style_id))
    }

    pub fn for_each_style_mut<F>(&mut self, f: F)
    where
        F: Fn(&mut NoteStyle),
    {
        self.styles.values_mut().for_each(f);
    }

    pub fn new_style(&mut self, name: String) -> Uuid {
        let id = Uuid::new_v4();
        let new_style = if let Ok(source) = self.try_get_default_style() {
            NoteStyle::new(
                name,
                source.get_font().clone(),
                source.get_background_color(),
            )
        } else {
            NoteStyle::new(name, Font::default(), Color::WHITE)
        };
        self.styles.insert(id, new_style);
        id
    }

    pub fn delete_style(&mut self, style_id: Uuid) -> Result<(), NotesCollectionError> {
        if self.styles.len() < 2 {
            Err(NotesCollectionError::DeleteLastStyle)
        } else if self.styles.remove(&style_id).is_some() {
            self.is_dirty = true;
            // if default style is being deleted select another one as default
            if style_id == self.default_style {
                self.default_style = self.styles.keys().next().copied().unwrap_or_default();
            }
            // replace all existing notes style if it is being deleted
            let default_style = self.default_style;
            self.for_each_note_mut(|note| {
                if note.style() == style_id {
                    note.set_style(default_style);
                }
            });
            Ok(())
        } else {
            Err(NotesCollectionError::StyleNotFound(style_id))
        }
    }

    // operations with particular note style

    pub fn try_get_note_style(&self, note_id: Uuid) -> Result<&NoteStyle, NotesCollectionError> {
        // the first, search among live notes
        self.try_get_note(&note_id)
            .and_then(|note| self.try_get_style(&note.style()))
            // otherwise search in deleted notes
            .or_else(|_| {
                self.iter_deleted_notes()
                    .find_map(|(id, note)| {
                        (*id == note_id).then_some(self.try_get_style(&note.style()))
                    })
                    .ok_or(NotesCollectionError::NoteNotFound(note_id))
                    .flatten()
            })
            // at the end return default style
            .or_else(|_| self.try_get_default_style())
    }

    pub fn try_get_note_style_index(&self, note_id: Uuid) -> Result<usize, NotesCollectionError> {
        self.try_get_note(&note_id).and_then(|note| {
            self.styles
                .keys()
                .enumerate()
                .find_map(|(index, id)| (*id == note.style()).then_some(index))
                .ok_or(NotesCollectionError::StyleNotFound(note.style()))
        })
    }

    pub fn try_set_note_style_by_index(
        &mut self,
        note_id: Uuid,
        style_index: usize,
    ) -> Result<(), NotesCollectionError> {
        if let Some(style_id) = self.styles.keys().nth(style_index).copied() {
            self.try_get_note_mut(&note_id)
                .map(|note| note.set_style(style_id))
        } else {
            Err(NotesCollectionError::StyleIndexNotFound(style_index))
        }
    }

    // private methods

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
        // instantiate default note style
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
    assert_eq!(expected.get_notes_count(), result.get_notes_count());
    assert_eq!(
        expected.iter_deleted_notes().count(),
        result.iter_deleted_notes().count()
    );
    assert_eq!(expected.iter_notes().count(), result.iter_notes().count());
    assert_eq!(expected.iter_styles().count(), result.iter_styles().count());
    assert_eq!(expected.default_style, result.default_style);
}

#[test]
fn create_read_update_delete_restore_operations() {
    let mut collection = NotesCollection::default();
    // initial note was automatically created
    assert_eq!(collection.get_notes_count(), 1);
    // create note
    let note_id = collection.new_note();
    assert_eq!(collection.get_notes_count(), 2);
    // get mutable ref to note
    let note_mut = collection.try_get_note_mut(&note_id);
    assert!(note_mut.is_ok());
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
    assert_eq!(collection.iter_deleted_notes().count(), 0);
    // delete note
    collection.delete_note(note_id);
    // collection is changed
    assert!(collection.is_unsaved());
    // fix note changes
    collection.commit_changes();
    // collection is not changed
    assert!(!collection.is_unsaved());
    // can restore deleted note
    assert_eq!(collection.iter_deleted_notes().count(), 1);
    // restore note
    let result = collection.try_restore_deleted_note(note_id);
    assert!(result.is_ok());
    // collection is changed
    assert!(collection.is_unsaved());
    // no more notes to restore
    assert_eq!(collection.iter_deleted_notes().count(), 0);
    // fix note changes
    collection.commit_changes();
    // collection is not changed
    assert!(!collection.is_unsaved());
}

#[test]
fn for_each_note_mut() {
    const NOTES_COUNT: usize = 10;

    let mut collection = NotesCollection::default();
    assert_eq!(collection.get_notes_count(), 1); // assume auto created new note with default syle

    // fill up collection with new NOTES_COUNT notes
    for _ in 0..NOTES_COUNT {
        collection.new_note();
    }
    assert_eq!(collection.get_notes_count(), NOTES_COUNT + 1);

    // test all of notes are visible
    assert!(collection.iter_notes().all(|(_, note)| note.is_visible()));

    // using for_each_not_mut() to hide all notes
    collection.for_each_note_mut(|note: &mut NoteData| note.set_visibility(false));

    // test all of notes are hidden
    assert!(!collection.iter_notes().any(|(_, note)| note.is_visible()));
}
