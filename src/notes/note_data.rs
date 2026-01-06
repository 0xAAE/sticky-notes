use super::{DEF_NOTE_HEIGHT, DEF_NOTE_WIDTH, EMTPY_TITLE, MAX_TITLE_CHARS, NO_TITLE, import};
use chrono::{DateTime, Local, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct NoteData {
    content: String,
    modified: DateTime<Utc>,
    style: Uuid,
    position: (usize, usize),
    size: (usize, usize),
    is_locked: bool,
    is_visible: bool,
}

impl NoteData {
    pub fn new(style: Uuid) -> Self {
        Self {
            content: String::new(),
            modified: Utc::now(),
            position: (0, 0),
            size: (DEF_NOTE_WIDTH, DEF_NOTE_HEIGHT),
            style,
            is_locked: false,
            is_visible: true,
        }
    }

    pub fn new_from_import(src: import::Note, is_visible: bool) -> Self {
        let position: (usize, usize) = match src.properties.position.get(0..2) {
            Some([first, second]) => (*first, *second),
            Some([first]) => (*first, 0),
            _ => (0, 0),
        };
        let size = match src.properties.size.get(0..2) {
            Some([first, second]) => (*first, *second),
            Some([first]) => (*first, 1),
            _ => (1, 1),
        };
        Self {
            content: src.body,
            modified: src.last_modified,
            style: src.cat,
            position,
            size,
            is_locked: src.properties.locked,
            is_visible,
        }
    }

    pub fn get_title(&self) -> &str {
        if self.content.is_empty() {
            EMTPY_TITLE
        } else {
            self.content.lines().next().map_or(NO_TITLE, |line| {
                match line.char_indices().nth(MAX_TITLE_CHARS) {
                    None => line,
                    Some((byte_index, _)) => &line[..byte_index],
                }
            })
        }
    }

    pub fn get_content(&self) -> &str {
        self.content.as_str()
    }

    pub fn get_modified(&self) -> DateTime<Local> {
        self.modified.into()
    }

    pub fn get_style(&self) -> &Uuid {
        &self.style
    }

    pub fn set_style(&mut self, style: Uuid) {
        self.style = style;
    }
}
