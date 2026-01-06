pub use collection::NotesCollection;
use note_data::NoteData;
use note_style::NoteStyle;

mod collection;
mod import;
mod note_data;
mod note_style;

const DEF_NOTE_STYLE_NAME: &str = "White";
const DEF_NOTE_STYLE_FONT: &str = "Open Sans";
const EMTPY_TITLE: &str = "<Empty>";
const NO_TITLE: &str = "Untitled";
const NO_CONTENT: &str = "click inside to begin edit the content";
const MAX_TITLE_CHARS: usize = 12;
const DEF_NOTE_WIDTH: usize = 400;
const DEF_NOTE_HEIGHT: usize = 300;
pub const INVISIBLE_TEXT: &str = "No one should expect seeing this text";
