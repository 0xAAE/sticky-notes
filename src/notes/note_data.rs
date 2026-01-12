use super::{
    DEF_NOTE_HEIGHT, DEF_NOTE_WIDTH, EMTPY_TITLE, MAX_TITLE_CHARS, NO_TITLE,
    indicator_stickynotes as import,
};
use chrono::{DateTime, Local, Utc};
use cosmic::iced::window::Id;
use uuid::Uuid;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Default, PartialEq)]
pub struct NoteData {
    content: String,
    modified: DateTime<Utc>,
    pub style: Uuid,
    position: (usize, usize),
    size: (usize, usize),
    pub is_locked: bool,
    pub is_visible: bool,
    #[serde(skip)]
    is_dirty: bool,
    #[serde(skip)]
    window_id: Option<Id>,
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
            is_dirty: false,
            window_id: None,
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
            modified: src.last_modified.into(),
            style: src.cat,
            position,
            size,
            is_locked: src.properties.locked,
            is_visible,
            is_dirty: false,
            window_id: None,
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

    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.modified = Utc::now();
        self.is_dirty = true;
    }

    pub fn get_modified(&self) -> DateTime<Local> {
        self.modified.into()
    }

    pub fn left(&self) -> usize {
        self.position.0
    }

    pub fn top(&self) -> usize {
        self.position.1
    }

    pub fn width(&self) -> usize {
        self.size.0
    }

    pub fn height(&self) -> usize {
        self.size.1
    }

    pub fn set_position(&mut self, left: usize, top: usize) {
        if self.position != (left, top) {
            self.position = (left, top);
            self.is_dirty = true;
        }
    }

    pub fn set_size(&mut self, width: usize, height: usize) {
        if self.size != (width, height) {
            self.size = (width, height);
            self.is_dirty = true;
        }
    }

    pub fn is_changed(&self) -> bool {
        self.is_dirty
    }

    pub fn commit(&mut self) {
        self.is_dirty = false;
    }

    pub fn assign_window(&mut self, window_id: Id) {
        self.window_id = Some(window_id);
    }

    pub fn try_get_window_id(&self) -> Option<Id> {
        self.window_id
    }
}
