use chrono::{DateTime, Local, Utc};
use std::{collections::HashMap, path::Path};
use uuid::Uuid;

mod import;

const EMTPY_TITLE: &str = "<Empty>";
pub const NO_TITLE: &str = "Untitled";
pub const NO_CONTENT: &str = "click inside to begin edit the content";
const MAX_TITLE_CHARS: usize = 12;

pub type NotesCollection = HashMap<Uuid, Note>;

pub async fn try_load<P: AsRef<Path> + std::fmt::Debug>(
    data_file: P,
) -> anyhow::Result<NotesCollection> {
    let parsed = import::try_import_indicator_stickynotes(data_file).await?;
    Ok(parsed.into_iter().map(Into::into).collect())
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct Note {
    content: String,
    modified: DateTime<Utc>,
    metadata: NoteMetadata,
}

impl Note {
    pub fn get_title(&self) -> &str {
        if self.content.is_empty() {
            EMTPY_TITLE
        } else {
            self.content.lines().next().map_or(NO_TITLE, |line| {
                line.get(0..MAX_TITLE_CHARS).unwrap_or(NO_TITLE)
            })
        }
    }

    pub fn get_content(&self) -> &str {
        self.content.as_str()
    }

    pub fn get_modified(&self) -> DateTime<Local> {
        self.modified.into()
    }
}

// Convert import::Note into (key, value) i.e. into (uuid, Note)
impl From<import::Note> for (Uuid, Note) {
    fn from(value: import::Note) -> Self {
        (
            value.uuid,
            Note {
                content: value.body,
                modified: value.last_modified,
                metadata: value.properties.into(),
            },
        )
    }
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
struct NoteMetadata {
    position: (usize, usize),
    size: (usize, usize),
    is_locked: bool,
}

impl From<import::NoteProperties> for NoteMetadata {
    fn from(value: import::NoteProperties) -> Self {
        let position: (usize, usize) = match value.position.get(0..2) {
            Some([first, second]) => (*first, *second),
            Some([first]) => (*first, 0),
            _ => (0, 0),
        };
        let size = match value.size.get(0..2) {
            Some([first, second]) => (*first, *second),
            Some([first]) => (*first, 1),
            _ => (1, 1),
        };
        Self {
            position,
            size,
            is_locked: value.locked,
        }
    }
}
