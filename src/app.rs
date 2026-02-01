// SPDX-License-Identifier: MPL-2.0

use crate::{config::Config, notes::NotesCollection};
use cosmic::{
    iced::{
        Point,
        mouse::Event as MouseEvent,
        window::{Event as WindowEvent, Id},
    },
    widget,
};
use uuid::Uuid;
pub use {applet::AppletModel, service::ServiceModel, utils::to_f32};

mod applet;
mod edit_style;
mod restore_view;
mod service;
mod settings_view;
mod sticky_window;
mod styles_view;
mod utils;

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    UpdateConfig(Config),
    Quit,
    // Windows
    TogglePopup,
    StickyWindowCreated(Id, Uuid), // (window_id, note_id)
    RestoreWindowCreated(Id),
    SettingsWindowCreated(Id),
    EditStyleWindowCreated(Id, Uuid), // (window_id, style_id)
    // After menu actions
    LoadNotes,
    SaveNotes,
    ImportNotes,
    ExportNotes,
    SetAllVisible(bool), // on / off
    LockAll,
    RestoreNotes,
    // settings actions
    SetDefaultStyle(usize), // set deafault style by index
    // notes collection load result shared for Load and Import
    LoadNotesCompleted(NotesCollection),
    LoadNotesFailed(String), // error message
    // export notes result
    ExportNotesCompleted,
    ExportNotesFailed(String), // error message
    // redirect editor actions to the edit context
    Edit(Id, widget::text_editor::Action),
    // iced "system" events handling
    AppWindowEvent((Id, WindowEvent)),
    AppMouseEvent((Id, MouseEvent)),
    // response on window::get_position() request
    WindowPositionResponse((Id, Option<Point>)),
    // note image button actions
    NoteLock(Id, bool),          // lock / unlock note
    NoteEdit(Id, bool),          // edit / save note content
    NoteStyle(Id),               // select style (background, font) for sticky window
    NoteSyleSelected(Id, usize), // style (background, font) for sticky window was selected by index in styles collection
    NoteNew,                     // create new note with default syle and begin edit
    NoteDelete(Id),              // delete note
    NoteRestore(Uuid),           // restore note
    // styles view button actions
    StyleNew,               // add new style
    StyleEdit(Uuid),        // edit style by style_id
    StyleDelete(Uuid),      // delete style by style_id
    EditStyleUpdate,        // Ok was pressed in edit style dialog
    EditStyleCancel,        // Cancel was pressed in edit style dialog
    InputStyleName(String), // update currently edited style name
    ColorUpdate(widget::color_picker::ColorPickerUpdate),
    // Settings
    OpenSettings,
}
