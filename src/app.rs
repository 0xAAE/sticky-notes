// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;

use crate::{
    app::styles_view::build_styles_list_view,
    config::Config,
    fl, icons,
    notes::{NoteData, NoteStyle, NotesCollection},
};
use cosmic::prelude::*;
use cosmic::{
    cosmic_config::{self, ConfigSet, CosmicConfigEntry},
    iced::{
        self, Alignment, Color, Event, Length, Point, Size, Subscription,
        core::mouse::Button as MouseButton,
        event::Status as EventStatus,
        mouse::Event as MouseEvent,
        window::{self, Event as WindowEvent, Id, Position},
    },
    widget::{self, menu},
};
use edit_style::EditStyleDialog;
use restore_view::build_restore_view;
use sticky_window::StickyWindow;
use utils::with_background;
pub use utils::{to_f32, to_usize};
use uuid::Uuid;

mod edit_style;
mod restore_view;
mod sticky_window;
mod styles_view;
mod utils;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
#[allow(clippy::zero_sized_map_values)] // key_binds: HashMap<menu::KeyBind, MenuAction>: map with zero-sized value type
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// Content itself
    notes: NotesCollection,
    /// windows by ID
    windows: HashMap<Id, StickyWindow>,
    cursor_window: Option<Id>,
    restore_window: Option<Id>,
    // optional currently edit style defined by its id:
    edit_style_dialog: Option<EditStyleDialog>,
    #[cfg(not(feature = "xdg_icons"))]
    icons: icons::IconSet,
    #[cfg(feature = "xdg_icons")]
    icons: icons::IconSet,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    UpdateConfig(Config),
    // Windows
    NewWindow(Id, Uuid), // (window_id, note_id)
    NewWindowRestore(Id),
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
    StyleEdit(Uuid),        // edit style by style_id
    StyleDelete(Uuid),      // delete style by style_id
    EditStyleUpdate,        // Ok was pressed in edit style dialog
    EditStyleCancel,        // Cancel was pressed in edit style dialog
    InputStyleName(String), // update currently edited style name
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "dev.0xaae.notes-basic";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Load config
        let config = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
            .map(|context| match Config::get_entry(&context) {
                Ok(config) => config,
                Err((errors, config)) => {
                    for why in errors {
                        eprintln!("error loading app config: {why}");
                        //tracing::error!(%why, "error loading app config");
                    }
                    config
                }
            })
            .unwrap_or_default();

        // Load notes from config if config/notes is not empty
        let notes = Self::load_notes_or_default(&config);

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config,
            notes,
            windows: HashMap::new(),
            cursor_window: None,
            restore_window: None,
            edit_style_dialog: None,
            icons: icons::IconSet::new(),
        };

        // Create a startup commands: spawn note windows and (optionally) import indicator-stickynotes data
        let mut startup_tasks: Vec<Task<cosmic::Action<Message>>> = app.spawn_sticky_windows();
        // Import notes: if notes is default and empty (so, it was not loaded from config)
        // and if indicator-stickynotes is set try import from it
        if app.notes.is_default_collection() {
            // try read import_file name from config or construct default path to indicator-stickynotes data file
            startup_tasks.push(cosmic::task::future(Self::import_notes(
                app.config.import_file.clone(),
            )));
        }

        (app, cosmic::task::batch(startup_tasks))
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let import_available = !self.config.import_file.is_empty();
        let hide_avail = self.notes.iter_notes().any(|(_, note)| note.is_visible());
        let show_avail = self.notes.iter_notes().any(|(_, note)| !note.is_visible());
        let lock_avail = self.notes.iter_notes().any(|(_, note)| !note.is_locked());
        let restore_avail = self.notes.iter_deleted_notes().next().is_some();
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("data")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![
                    menu::Item::Button(fl!("load"), None, MenuAction::Load),
                    menu::Item::Button(fl!("save"), None, MenuAction::Save),
                    menu::Item::Divider,
                    if import_available {
                        menu::Item::Button(fl!("import"), None, MenuAction::Import)
                    } else {
                        menu::Item::ButtonDisabled(fl!("import"), None, MenuAction::Import)
                    },
                    if import_available {
                        menu::Item::Button(fl!("export"), None, MenuAction::Export)
                    } else {
                        menu::Item::ButtonDisabled(fl!("export"), None, MenuAction::Export)
                    },
                    menu::Item::Divider,
                    if hide_avail {
                        menu::Item::Button(fl!("hide-all"), None, MenuAction::HideAll)
                    } else {
                        menu::Item::ButtonDisabled(fl!("hide-all"), None, MenuAction::HideAll)
                    },
                    if show_avail {
                        menu::Item::Button(fl!("show-all"), None, MenuAction::ShowAll)
                    } else {
                        menu::Item::ButtonDisabled(fl!("show-all"), None, MenuAction::ShowAll)
                    },
                    if lock_avail {
                        menu::Item::Button(fl!("lock-all"), None, MenuAction::LockAll)
                    } else {
                        menu::Item::ButtonDisabled(fl!("lock-all"), None, MenuAction::LockAll)
                    },
                    if restore_avail {
                        menu::Item::Button(fl!("restore-notes"), None, MenuAction::RestoreNotes)
                    } else {
                        menu::Item::ButtonDisabled(
                            fl!("restore-notes"),
                            None,
                            MenuAction::RestoreNotes,
                        )
                    },
                ],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        self.build_main_view()
    }

    /// Constructs views for other windows.
    fn view_window(&self, id: Id) -> Element<'_, Self::Message> {
        if let Some(sticky_window) = self.windows.get(&id) {
            let note_bg = self
                .notes
                .try_get_note_style(sticky_window.get_note_id())
                .map_or(Color::WHITE, NoteStyle::get_background_color);
            let window_content = sticky_window.build_view(id, &self.notes, &self.icons);
            let window_view = with_background(window_content, note_bg);
            iced::widget::column![window_view].into()
        } else if let Some(restore_id) = self.restore_window
            && restore_id == id
        {
            widget::container(build_restore_view(
                &self.notes,
                &self.icons,
                self.config.toolbar_icon_size,
            ))
            .class(cosmic::style::Container::Background)
            .padding(cosmic::theme::spacing().space_s)
            .into()
        } else {
            self.build_main_view()
        }
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They can be dynamically
    /// stopped and started conditionally based on application state, or persist
    /// indefinitely.
    fn subscription(&self) -> Subscription<Self::Message> {
        // Add subscriptions which are always active.
        let subscriptions = vec![
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
            // subscribe to some interested events from mouse and window:
            iced::event::listen_with(|evt, status, id| match evt {
                Event::Mouse(MouseEvent::CursorMoved { .. })
                | Event::Window(WindowEvent::RedrawRequested(_)) => None,
                Event::Mouse(mouse_event) => {
                    // get Mouse events onpy if unhandled
                    if status == EventStatus::Ignored {
                        Some(Message::AppMouseEvent((id, mouse_event)))
                    } else {
                        None
                    }
                }
                Event::Window(window_event) => {
                    // get Closed & CloseRequested always, others only if unhandled
                    if window_event == WindowEvent::CloseRequested
                        || window_event == WindowEvent::Closed
                        || status == EventStatus::Ignored
                    {
                        Some(Message::AppWindowEvent((id, window_event)))
                    } else {
                        None
                    }
                }
                _ => None,
            }),
        ];
        Subscription::batch(subscriptions)
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateConfig(config) => {
                self.config = config;
            }

            // messages related to loading and saving notes
            Message::LoadNotes => {
                if self.notes.is_unsaved() {
                    // todo: ask to overwrite unsaved notes
                    eprintln!("drop unsaved changes while loading collection");
                }
                self.notes = Self::load_notes_or_default(&self.config);
            }

            Message::SaveNotes => {
                //todo: stop edititng all sticky windows or ask user
                if let Err(e) = self.save_notes() {
                    eprintln!("Failed saving notes: {e}");
                }
            }

            Message::ImportNotes => {
                if self.notes.is_unsaved() {
                    // todo: ask to overwrite unsaved notes
                    eprintln!("drop unsaved changes while importing collection");
                }
                let import_file = self.config.import_file.clone();
                // opposite to other cases return real task instead of none()
                return cosmic::task::future(Self::import_notes(import_file));
            }

            Message::ExportNotes => {
                //todo: stop edititng all sticky windows (?) or ask user about
                let export_file = self.config.import_file.clone();
                let notes = self.notes.clone();
                return cosmic::task::future(Self::export_notes(export_file, notes));
            }

            Message::SetAllVisible(on) => {
                return self.on_set_visibility(on);
            }

            Message::LockAll => {
                self.notes.for_each_note_mut(|note| note.set_locking(true));
            }

            Message::RestoreNotes => {
                return self.spawn_restore_notes_window();
            }

            Message::SetDefaultStyle(style_index) => {
                if !self.notes.try_set_default_style_by_index(style_index) {
                    eprintln!(
                        "failed cghanging default sticly window style to {style_index}: no such style"
                    );
                }
            }

            Message::LoadNotesCompleted(imported) => {
                self.notes = imported;
                return cosmic::task::batch(self.spawn_sticky_windows());
            }

            Message::LoadNotesFailed(msg) => {
                eprintln!("failed loading notes: {msg}");
            }

            Message::ExportNotesCompleted => {
                // nothing to do for now
            }

            Message::ExportNotesFailed(msg) => {
                eprintln!("failed exporting notes: {msg}");
            }

            // message related to windows management
            Message::NewWindow(id, note_id) => {
                self.windows.insert(
                    id,
                    StickyWindow::new(note_id, self.config.toolbar_icon_size),
                );
            }

            Message::NewWindowRestore(id) => {
                self.restore_window = Some(id);
                return self.set_window_title(fl!("recently-deleted-title"), id);
            }

            // redirect edit actions to the edit context
            Message::Edit(window_id, action) => {
                if let Some(sticky_window) = self.windows.get_mut(&window_id)
                    && let Err(e) = sticky_window.do_edit_action(action)
                {
                    eprintln!("failed perform edit: {e}");
                }
            }

            Message::AppMouseEvent((id, event)) => {
                return self.on_mouse_event(id, &event);
            }

            Message::AppWindowEvent((id, event)) => {
                return self.on_window_event(id, &event);
            }

            Message::WindowPositionResponse((id, location)) => {
                if let Some(point) = location
                    && let Some(note) = self.try_get_note_mut(id)
                {
                    note.set_position(to_usize(point.x), to_usize(point.y));
                }
            }

            Message::NoteLock(id, is_on) => {
                self.on_change_note_locking(id, is_on);
            }

            Message::NoteEdit(id, is_on) => {
                if is_on {
                    self.on_start_edit(id);
                } else {
                    self.on_finish_edit(id);
                }
            }

            Message::NoteStyle(id) => {
                if let Some(sticky_window) = self.windows.get_mut(&id) {
                    sticky_window.allow_select_style(self.notes.get_style_names());
                } else {
                    eprintln!("{id}: note is not found to change style");
                }
            }

            Message::NoteSyleSelected(id, style_index) => {
                self.on_style_selected(id, style_index);
            }

            Message::NoteNew => {
                return self.on_new_note_window();
            }

            Message::NoteDelete(id) => {
                return self.on_delete_note(id);
            }

            Message::NoteRestore(note_id) => {
                return self.on_restore_note(note_id);
            }

            Message::StyleEdit(style_id) => {
                if let Some(style) = self.notes.try_get_style(&style_id) {
                    self.edit_style_dialog = Some(EditStyleDialog::new(style_id, style));
                }
            }

            Message::StyleDelete(style_id) => {
                //TODO: implement style deleting
                println!("delete style {style_id}");
            }

            Message::EditStyleUpdate => {
                if let Some(dialog) = self.edit_style_dialog.take() {
                    self.on_style_updated(
                        dialog.get_id(),
                        dialog.get_name(),
                        dialog.get_font_name(),
                        dialog.get_background_color(),
                    );
                }
            }

            Message::EditStyleCancel => {
                self.edit_style_dialog = None;
            }

            Message::InputStyleName(value) => {
                if let Some(dialog) = &mut self.edit_style_dialog {
                    dialog.update_name(value);
                }
            }
        }
        Task::none()
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        self.edit_style_dialog
            .as_ref()
            .map(|dialog| dialog.build_dialog_view())
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        // save changes if any to persistent storage
        if self.notes.is_unsaved()
            && let Err(e) = self.save_notes()
        {
            eprintln!("Failed saving notes on exit: {e}");
        }
        // warn if deleted notes were dropped
        let count_deleted = self.notes.iter_deleted_notes().count();
        if count_deleted > 0 {
            println!("Finally drop deleted notes on exit: {count_deleted}");
        }
        None
    }
}

impl AppModel {
    fn try_get_note_mut(&mut self, window_id: Id) -> Option<&mut NoteData> {
        self.windows
            .get(&window_id)
            .and_then(|sticky_window| self.notes.try_get_note_mut(&sticky_window.get_note_id()))
    }

    fn load_notes_or_default(config: &Config) -> NotesCollection {
        if config.notes.is_empty() {
            NotesCollection::default()
        } else {
            NotesCollection::try_read(&config.notes)
                .map_err(|e| {
                    eprintln!(
                        "failed loading notes from {}/v{}/notes: {e}",
                        <Self as cosmic::Application>::APP_ID,
                        Config::VERSION
                    );
                })
                .unwrap_or_default()
        }
    }

    fn save_notes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.notes.try_write()?;
        let global_config =
            cosmic_config::Config::new(<Self as cosmic::Application>::APP_ID, Config::VERSION)?;
        let tx = global_config.transaction();
        tx.set("notes", json)?;
        tx.commit()?;
        self.notes.commit_changes();
        Ok(())
    }

    async fn import_notes(configured_import_file: String) -> Message {
        if configured_import_file.is_empty() {
            Message::LoadNotesFailed("No import file is set".to_string())
        } else {
            let import_file_owned = configured_import_file.clone();
            match tokio::task::spawn_blocking(move || {
                NotesCollection::try_import(import_file_owned)
            })
            .await
            {
                Ok(task) => match task.await {
                    Ok(v) => Message::LoadNotesCompleted(v),
                    Err(e) => {
                        let msg =
                            format!("failed reading notes from {configured_import_file}: {e}");
                        Message::LoadNotesFailed(msg)
                    }
                },
                Err(e) => Message::LoadNotesFailed(format!("{e}")),
            }
        }
    }

    async fn export_notes(configured_export_file: String, notes: NotesCollection) -> Message {
        if configured_export_file.is_empty() {
            Message::ExportNotesFailed("No export file is set".to_string())
        } else {
            let export_file_owned = configured_export_file.clone();
            match tokio::task::spawn_blocking(move || {
                NotesCollection::try_export(export_file_owned, notes)
            })
            .await
            {
                Ok(task) => match task.await {
                    Ok(()) => Message::ExportNotesCompleted,
                    Err(e) => {
                        let msg =
                            format!("failed reading notes from {configured_export_file}: {e}");
                        Message::ExportNotesFailed(msg)
                    }
                },
                Err(e) => Message::ExportNotesFailed(format!("{e}")),
            }
        }
    }

    fn on_new_note_window(&mut self) -> Task<cosmic::Action<Message>> {
        let note_id = self.notes.new_note();
        if let Some(note) = self.notes.try_get_note_mut(&note_id) {
            let (window_id, task) = Self::spawn_sticky_window(&note_id, note);
            self.on_start_edit(window_id);
            task
        } else {
            Task::none()
        }
    }

    fn on_restore_note(&mut self, note_id: Uuid) -> Task<cosmic::Action<Message>> {
        if let Some(note) = self.notes.try_restore_deleted_note(note_id) {
            let (_id, task) = Self::spawn_sticky_window(&note_id, note);
            task
        } else {
            Task::none()
        }
    }

    fn on_change_note_locking(&mut self, window_id: Id, is_on: bool) {
        if let Some(note) = self.try_get_note_mut(window_id) {
            note.set_locking(is_on);
        } else {
            println!("{window_id}: note is not found to change locking");
        }
    }

    fn on_set_visibility(&mut self, on: bool) -> Task<cosmic::Action<Message>> {
        self.notes.for_each_note_mut(|note| note.set_visibility(on));
        if on {
            cosmic::task::batch(self.spawn_sticky_windows())
        } else {
            cosmic::task::batch(self.close_sticky_windows())
        }
    }

    fn on_start_edit(&mut self, window_id: Id) {
        if let Some(sticky_window) = self.windows.get_mut(&window_id) {
            if let Some(note) = self.notes.try_get_note(&sticky_window.get_note_id())
                && let Err(e) = sticky_window.start_edit(note.get_content())
            {
                eprintln!("[{window_id}] failed to start edit: {e}");
            }
        } else {
            eprintln!("[{window_id}] failed to start edit: sticky window is not found");
        }
    }

    fn on_finish_edit(&mut self, window_id: Id) {
        if let Some(sticky_window) = self.windows.get_mut(&window_id) {
            if let Some(note) = self.notes.try_get_note_mut(&sticky_window.get_note_id()) {
                match sticky_window.finish_edit() {
                    Ok(text) => note.set_content(text),
                    Err(e) => eprintln!("[{window_id}] failed to finish edit: {e}"),
                }
            }
        } else {
            eprintln!("[{window_id}] failed to finish edit: sticky window is not found");
        }
    }

    fn on_style_selected(&mut self, id: Id, style_index: usize) {
        if let Some(sticky_window) = self.windows.get_mut(&id) {
            sticky_window.disable_select_style();
            if !self
                .notes
                .try_set_note_style_by_index(sticky_window.get_note_id(), style_index)
            {
                eprintln!("{id}: selectd style was not found to assign to sticky window");
            }
        } else {
            eprintln!("{id}: sticky window is not found to change style");
        }
    }

    fn on_delete_note(&mut self, id: Id) -> Task<cosmic::Action<Message>> {
        if let Some(sticky_window) = self.windows.remove(&id) {
            self.notes.delete_note(sticky_window.get_note_id());
            window::close(id)
        } else {
            Task::none()
        }
    }

    fn on_style_updated(&mut self, style_id: Uuid, name: &str, font_name: &str, bgcolor: Color) {
        if let Some(style) = self.notes.try_get_style_mut(&style_id) {
            style.set_name(name);
            style.set_font_name(font_name);
            style.set_background_color(bgcolor);
        } else {
            eprintln!("failed to update style {style_id}: style not found");
        }
    }

    fn on_mouse_event(
        &mut self,
        id: Id,
        event: &MouseEvent,
    ) -> Task<cosmic::Action<<AppModel as cosmic::Application>::Message>> {
        match event {
            MouseEvent::ButtonPressed(MouseButton::Left) => {
                if let Some(cursor_id) = self.cursor_window
                    && cursor_id == id
                {
                    return self.core.drag(Some(id));
                }
            }
            MouseEvent::ButtonReleased(MouseButton::Left) => {
                if let Some(cursor_id) = self.cursor_window
                    && cursor_id == id
                {
                    return self
                        .core
                        .drag(None)
                        .chain(window::get_position(id).map(move |pos| {
                            cosmic::Action::App(Message::WindowPositionResponse((id, pos)))
                        }));
                }
            }
            MouseEvent::CursorEntered => {
                self.cursor_window.replace(id);
            }
            _ => {}
        }
        Task::none()
    }

    fn on_window_event(
        &mut self,
        id: Id,
        event: &WindowEvent,
    ) -> Task<cosmic::Action<<AppModel as cosmic::Application>::Message>> {
        match event {
            WindowEvent::Resized(size) => {
                if let Some(note) = self.try_get_note_mut(id) {
                    note.set_size(to_usize(size.width), to_usize(size.height));
                }
            }
            WindowEvent::Moved(point) => {
                if let Some(note) = self.try_get_note_mut(id) {
                    note.set_position(to_usize(point.x), to_usize(point.y));
                }
            }
            // do nothing with CloseRequested at the moment:
            // WindowEvent::CloseRequested => {}
            WindowEvent::Closed => {
                if let Some(restore_id) = self.restore_window
                    && restore_id == id
                {
                    // restore window has closed, forget its id
                    self.restore_window = None;
                } else if let Some(main_id) = self.core.main_window_id()
                    && main_id == id
                {
                    return self.close_all_windows();
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn spawn_sticky_windows(&mut self) -> Vec<Task<cosmic::Action<Message>>> {
        let existing_windows = std::mem::take(&mut self.windows);
        let mut commands: Vec<_> = existing_windows.into_keys().map(window::close).collect();
        commands.extend(self.notes.iter_notes_mut().map(|(note_id, note)| {
            let (_, spawn_window) = Self::spawn_sticky_window(note_id, note);
            spawn_window
        }));
        commands
    }

    fn spawn_sticky_window(note_id: &Uuid, note: &NoteData) -> (Id, Task<cosmic::Action<Message>>) {
        let (id, spawn_window) = window::open(window::Settings {
            position: Position::Specific(Point::new(to_f32(note.left()), to_f32(note.top()))),
            size: Size::new(to_f32(note.width()), to_f32(note.height())),
            decorations: false,
            ..Default::default()
        });
        let note_id = *note_id;
        (
            id,
            spawn_window.map(move |id| cosmic::Action::App(Message::NewWindow(id, note_id))),
        )
    }

    fn spawn_restore_notes_window(&self) -> Task<cosmic::Action<Message>> {
        let (_id, spawn_window) = window::open(window::Settings {
            size: self.config.restore_notes_size(),
            ..Default::default()
        });
        spawn_window.map(|id| cosmic::Action::App(Message::NewWindowRestore(id)))
    }

    fn close_sticky_windows(&mut self) -> Vec<Task<cosmic::Action<Message>>> {
        let existing_windows = std::mem::take(&mut self.windows);
        existing_windows
            .into_keys()
            .map(window::close)
            .collect::<Vec<Task<cosmic::Action<Message>>>>()
    }

    fn close_all_windows(&mut self) -> Task<cosmic::Action<Message>> {
        let mut commands = self.close_sticky_windows();
        if let Some(restore_id) = self.restore_window {
            commands.push(window::close(restore_id));
        }
        cosmic::task::batch(commands)
    }

    fn build_main_view(&self) -> Element<'_, Message> {
        let styles = self.notes.get_style_names();
        if styles.is_empty() {
            eprintln!("Not any sticky window style is available");
            return widget::column::with_capacity(1)
                .push(widget::text(fl!("problem-text")))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        let default_style_index = self.notes.try_get_default_style_index();
        widget::column::with_capacity(3)
            .spacing(cosmic::theme::spacing().space_s)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(widget::divider::horizontal::light())
            .push(
                widget::row::with_capacity(2)
                    .spacing(cosmic::theme::spacing().space_m)
                    .push(widget::text(fl!("select-default-style")))
                    .align_y(Alignment::Center)
                    .push(
                        widget::dropdown(styles, default_style_index, move |index| {
                            Message::SetDefaultStyle(index)
                        })
                        .placeholder("Choose a style..."),
                    ),
            )
            .push(build_styles_list_view(
                &self.notes,
                &self.icons,
                self.config.toolbar_icon_size,
            ))
            .into()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    Load,
    Save,
    Import,
    Export,
    HideAll,
    ShowAll,
    LockAll,
    RestoreNotes,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::Load => Message::LoadNotes,
            MenuAction::Save => Message::SaveNotes,
            MenuAction::Import => Message::ImportNotes,
            MenuAction::Export => Message::ExportNotes,
            MenuAction::ShowAll => Message::SetAllVisible(true),
            MenuAction::HideAll => Message::SetAllVisible(false),
            MenuAction::LockAll => Message::LockAll,
            MenuAction::RestoreNotes => Message::RestoreNotes,
        }
    }
}
