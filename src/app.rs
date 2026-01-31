// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;

use crate::{
    config::Config,
    fl, icons,
    notes::{NoteData, NoteStyle, NotesCollection},
};
use cosmic::{
    applet,
    cosmic_config::{self, ConfigSet, CosmicConfigEntry},
    iced::{
        self, Alignment, Color, Event, Point, Size, Subscription,
        core::mouse::Button as MouseButton,
        event::Status as EventStatus,
        mouse::Event as MouseEvent,
        widget::column,
        window::{self, Event as WindowEvent, Id, Position},
    },
    widget,
};
use cosmic::{iced::Limits, prelude::*};
use edit_style::EditStyleDialog;
use restore_view::build_restore_view;
use settings_view::build_settings_view;
use sticky_window::StickyWindow;
use utils::with_background;
pub use utils::{to_f32, to_usize};
use uuid::Uuid;

mod edit_style;
mod restore_view;
mod settings_view;
mod sticky_window;
mod styles_view;
mod utils;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    // Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Configuration data that persists between application runs.
    config: Config,
    // Collection of notes & styles
    notes: NotesCollection,
    main_popup_id: Option<Id>,
    settings_window_id: Option<Id>,
    edit_style: Option<(Id, EditStyleDialog)>,
    restore_window_id: Option<Id>,
    // sticky windows by ID
    sticky_windows: HashMap<Id, StickyWindow>,
    // Window is under cursor at the moment
    cursor_window: Option<Id>,
    #[cfg(not(feature = "xdg_icons"))]
    icons: icons::IconSet,
    #[cfg(feature = "xdg_icons")]
    icons: icons::IconSet,
}

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
            // Optional configuration file for an application.
            config,
            notes,
            main_popup_id: None,
            settings_window_id: None,
            edit_style: None,
            restore_window_id: None,
            sticky_windows: HashMap::new(),
            cursor_window: None,
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

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button_from_handle(self.icons.notes())
            .on_press_down(Message::TogglePopup)
            .into()
    }

    /// Constructs views for other windows.
    fn view_window(&self, id: Id) -> Element<'_, Self::Message> {
        if let Some(sticky_window) = self.sticky_windows.get(&id) {
            let note_bg = self
                .notes
                .try_get_note_style(sticky_window.get_note_id())
                .map_or(Color::WHITE, NoteStyle::get_background_color);
            let window_content = sticky_window.build_view(id, &self.notes, &self.icons);
            let window_view = with_background(window_content, note_bg);
            iced::widget::column![window_view].into()
        } else if let Some(window_id) = self.restore_window_id
            && window_id == id
        {
            widget::container(build_restore_view(
                &self.notes,
                &self.icons,
                self.config.toolbar_icon_size,
            ))
            .class(cosmic::style::Container::Background)
            .padding(cosmic::theme::spacing().space_s)
            .into()
        } else if let Some(window_id) = self.settings_window_id
            && window_id == id
        {
            widget::container(build_settings_view(
                &self.notes,
                &self.icons,
                self.config.toolbar_icon_size,
            ))
            .class(cosmic::style::Container::Background)
            .padding(cosmic::theme::spacing().space_s)
            .into()
        } else if let Some((window_id, dialog)) = &self.edit_style
            && *window_id == id
        {
            widget::container(dialog.build_dialog_view())
                .class(cosmic::style::Container::Background)
                .padding(cosmic::theme::spacing().space_s)
                .into()
        } else if let Some(window_id) = self.main_popup_id
            && window_id == id
        {
            self.build_main_popup_view()
        } else {
            widget::text("").into()
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

            Message::Quit => {
                self.on_quit();
                return iced::exit();
            }

            Message::TogglePopup => {
                if let Some(p) = self.main_popup_id.take() {
                    return cosmic::iced::platform_specific::shell::commands::popup::destroy_popup(
                        p,
                    );
                }
                let new_id = window::Id::unique();
                self.main_popup_id.replace(new_id);
                let mut popup_settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().unwrap(),
                    new_id,
                    Some((500, 500)),
                    None,
                    None,
                );
                popup_settings.positioner.size_limits = Limits::NONE
                    .min_width(100.0)
                    .min_height(100.0)
                    .max_height(400.0)
                    .max_width(500.0);
                return cosmic::iced::platform_specific::shell::commands::popup::get_popup(
                    popup_settings,
                );
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
                return self.on_change_notes_visibility(on);
            }

            Message::LockAll => {
                self.notes.for_each_note_mut(|note| note.set_locking(true));
            }

            Message::RestoreNotes => {
                return self.spawn_restore_notes_window();
            }

            Message::SetDefaultStyle(style_index) => {
                if let Err(e) = self.notes.try_set_default_style_by_index(style_index) {
                    eprintln!("failed changing default style: {e}");
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
            Message::StickyWindowCreated(id, note_id) => {
                self.sticky_windows.insert(
                    id,
                    StickyWindow::new(note_id, self.config.toolbar_icon_size),
                );
                if let Ok(note) = self.notes.try_get_note(&note_id) {
                    return self.set_window_title(note.get_title().to_string(), id);
                }
            }

            Message::RestoreWindowCreated(id) => {
                self.restore_window_id = Some(id);
                return self.set_window_title(fl!("recently-deleted-title"), id);
            }

            Message::SettingsWindowCreated(id) => {
                self.settings_window_id = Some(id);
                return self.set_window_title(fl!("settings-title"), id);
            }

            Message::EditStyleWindowCreated(window_id, style_id) => {
                match self.notes.try_get_style(&style_id) {
                    Ok(style) => {
                        self.edit_style = Some((window_id, EditStyleDialog::new(style_id, style)));
                        return self.set_window_title(fl!("create-new-style"), window_id);
                    }
                    Err(e) => eprint!("Failed to edit style: {e}"),
                }
            }

            // redirect edit actions to the edit context
            Message::Edit(window_id, action) => {
                if let Some(sticky_window) = self.sticky_windows.get_mut(&window_id)
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
                if let Some(point) = location {
                    match self.try_get_note_mut(id) {
                        Ok(note) => note.set_position(to_usize(point.x), to_usize(point.y)),
                        Err(e) => eprintln!("Failed to update position: {e}"),
                    }
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
                if let Some(sticky_window) = self.sticky_windows.get_mut(&id) {
                    sticky_window.allow_select_style(self.notes.get_style_names());
                } else {
                    eprintln!("{id}: sticky window is not found to change style");
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

            Message::StyleNew => {
                return self.on_new_style();
            }

            Message::StyleEdit(style_id) => {
                return self.spawn_edit_style_window(style_id);
            }

            Message::StyleDelete(style_id) => {
                self.on_delete_style(style_id);
            }

            Message::EditStyleUpdate => {
                if let Some((window_id, dialog)) = self.edit_style.take() {
                    self.on_style_updated(
                        dialog.get_id(),
                        dialog.get_name(),
                        dialog.get_font_name(),
                        dialog.get_background_color(),
                    );
                    return window::close(window_id);
                }
            }

            Message::EditStyleCancel => {
                if let Some((window_id, dialog)) = self.edit_style.take() {
                    if let Err(e) = self.notes.delete_style(dialog.get_id()) {
                        eprintln!("Failed to delete new style: {e}");
                    }
                    return window::close(window_id);
                }
            }

            Message::InputStyleName(value) => {
                if let Some((_window_id, dialog)) = &mut self.edit_style {
                    dialog.update_name(value);
                }
            }

            Message::ColorUpdate(event) => {
                if let Some((_window_id, dialog)) = &mut self.edit_style {
                    return dialog.on_color_picker_update(event);
                }
            }

            Message::OpenSettings => {
                return Self::spawn_settings_window();
            }
        }
        Task::none()
    }

    /// Called when a window is resized.
    fn on_window_resize(&mut self, id: window::Id, width: f32, height: f32) {
        if self.sticky_windows.contains_key(&id) {
            match self.try_get_note_mut(id) {
                Ok(note) => {
                    note.set_size(to_usize(width), to_usize(height));
                }
                Err(e) => eprintln!("Failed to update sticky window size: {e}"),
            }
        }
    }

    /// Called when the escape key is pressed.
    fn on_escape(&mut self) -> Task<cosmic::Action<Self::Message>> {
        if let Some(window_id) = self.core.focused_window()
            && let Some(window) = self.sticky_windows.get_mut(&window_id)
            && let Err(e) = window.finish_edit()
        {
            eprintln!("Erro while cancelling edit: {e}");
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(applet::style())
    }
}

impl AppModel {
    fn on_quit(&mut self) {
        // save changes if any to persistent storage
        if self.notes.is_unsaved() {
            if let Err(e) = self.save_notes() {
                eprintln!("Failed saving notes on exit: {e}");
            } else {
                println!("Notes collection was changed, save");
            }
        } else {
            println!("Notes collection is unchanged, skip saving");
        }
        // warn if deleted notes were dropped
        let count_deleted = self.notes.iter_deleted_notes().count();
        if count_deleted > 0 {
            //TODO: what about saving deleted notes too? Maybe with their TTLs
            println!("Finally drop deleted notes on exit: {count_deleted}");
        }
    }

    fn try_get_note_mut(&mut self, window_id: Id) -> Result<&mut NoteData, String> {
        self.sticky_windows
            .get(&window_id)
            .ok_or_else(|| format!("Sticky window {window_id} is not found"))
            .and_then(|sticky_window| {
                self.notes
                    .try_get_note_mut(&sticky_window.get_note_id())
                    .map_err(|e| e.to_string())
            })
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
        match self.notes.try_get_note_mut(&note_id) {
            Ok(note) => {
                let (window_id, task) = Self::spawn_sticky_window(&note_id, note);
                task.chain(
                    cosmic::Task::done(Message::NoteEdit(window_id, true))
                        .map(cosmic::Action::from),
                )
            }
            Err(e) => {
                eprintln!("Failed to create new note: {e}");
                Task::none()
            }
        }
    }

    fn on_restore_note(&mut self, note_id: Uuid) -> Task<cosmic::Action<Message>> {
        match self.notes.try_restore_deleted_note(note_id) {
            Ok(note) => {
                let (_id, task) = Self::spawn_sticky_window(&note_id, note);
                task
            }
            Err(e) => {
                eprintln!("Failed to restore note: {e}");
                Task::none()
            }
        }
    }

    fn on_change_note_locking(&mut self, window_id: Id, is_on: bool) {
        match self.try_get_note_mut(window_id) {
            Ok(note) => {
                note.set_locking(is_on);
            }
            Err(e) => eprintln!("Failed to change note locking: {e}"),
        }
    }

    fn on_change_notes_visibility(&mut self, on: bool) -> Task<cosmic::Action<Message>> {
        self.notes.for_each_note_mut(|note| note.set_visibility(on));
        if on {
            cosmic::task::batch(self.spawn_sticky_windows())
        } else {
            cosmic::task::batch(self.close_sticky_windows())
        }
    }

    fn on_start_edit(&mut self, window_id: Id) {
        if let Some(sticky_window) = self.sticky_windows.get_mut(&window_id) {
            if let Ok(note) = self.notes.try_get_note(&sticky_window.get_note_id())
                && let Err(e) = sticky_window.start_edit(note.get_content())
            {
                eprintln!("[{window_id}] failed to start edit: {e}");
            }
        } else {
            eprintln!("[{window_id}] failed to start edit: sticky window is not found");
        }
    }

    fn on_finish_edit(&mut self, window_id: Id) {
        if let Some(sticky_window) = self.sticky_windows.get_mut(&window_id) {
            if let Ok(note) = self.notes.try_get_note_mut(&sticky_window.get_note_id()) {
                match sticky_window.finish_edit() {
                    Ok(text) => note.set_content(text),
                    Err(e) => eprintln!("[{window_id}] failed to finish edit: {e}"),
                }
            }
        } else {
            eprintln!("[{window_id}] failed to finish edit: sticky window is not found");
        }
    }

    fn on_style_selected(&mut self, window_id: Id, style_index: usize) {
        if let Some(sticky_window) = self.sticky_windows.get_mut(&window_id) {
            sticky_window.disable_select_style();
            if let Err(e) = self
                .notes
                .try_set_note_style_by_index(sticky_window.get_note_id(), style_index)
            {
                eprintln!("[{window_id}] Failed select style: {e}");
            }
        } else {
            eprintln!("[{window_id}] sticky window is not found to change style");
        }
    }

    fn on_delete_note(&mut self, id: Id) -> Task<cosmic::Action<Message>> {
        if let Some(sticky_window) = self.sticky_windows.remove(&id) {
            self.notes.delete_note(sticky_window.get_note_id());
            window::close(id)
        } else {
            Task::none()
        }
    }

    fn on_new_style(&mut self) -> Task<cosmic::Action<Message>> {
        let name = format!(
            "{}-{}",
            fl!("new-style-name"),
            self.notes.get_styles_count()
        );
        let style_id = self.notes.new_style(name);
        // turn off style selectors for each sticky windows
        self.sticky_windows
            .values_mut()
            .for_each(StickyWindow::disable_select_style);
        self.spawn_edit_style_window(style_id)
    }

    fn on_delete_style(&mut self, style_id: Uuid) {
        match self.notes.delete_style(style_id) {
            Ok(()) => {
                // as default style might be changed turn off style selectors in all of the sticky windows
                self.sticky_windows
                    .values_mut()
                    .for_each(StickyWindow::disable_select_style);
            }
            Err(e) => {
                eprintln!("Failed to delete style: {e}");
            }
        }
    }

    fn on_style_updated(&mut self, style_id: Uuid, name: &str, font_name: &str, bgcolor: Color) {
        match self.notes.try_get_style_mut(&style_id) {
            Ok(style) => {
                style.set_name(name);
                style.set_font_name(font_name);
                style.set_background_color(bgcolor);
            }
            Err(e) => eprintln!("Failed to update style: {e}"),
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
            // WindowEvent::Resized(size) => is handled by on_window_resize() override
            WindowEvent::Moved(point) => {
                if self.sticky_windows.contains_key(&id) {
                    match self.try_get_note_mut(id) {
                        Ok(note) => {
                            note.set_position(to_usize(point.x), to_usize(point.y));
                        }
                        Err(e) => eprintln!("Failed to update sticky window position: {e}"),
                    }
                }
            }
            // do nothing with CloseRequested at the moment:
            // WindowEvent::CloseRequested => {}
            WindowEvent::Closed => {
                if let Some(window_id) = self.restore_window_id
                    && window_id == id
                {
                    // restore window has closed, forget its id
                    self.restore_window_id = None;
                } else if let Some(window_id) = self.settings_window_id
                    && window_id == id
                {
                    self.settings_window_id = None;
                } else if let Some((window_id, _)) = &self.edit_style
                    && *window_id == id
                {
                    self.edit_style = None;
                } else if let Some(window_id) = self.main_popup_id
                    && window_id == id
                {
                    self.main_popup_id = None;
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
        let existing_windows = std::mem::take(&mut self.sticky_windows);
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
            spawn_window
                .map(move |id| cosmic::Action::App(Message::StickyWindowCreated(id, note_id))),
        )
    }

    fn spawn_restore_notes_window(&self) -> Task<cosmic::Action<Message>> {
        let (_id, spawn_window) = window::open(window::Settings {
            size: self.config.restore_notes_size(),
            ..Default::default()
        });
        spawn_window.map(|id| cosmic::Action::App(Message::RestoreWindowCreated(id)))
    }

    fn spawn_settings_window() -> Task<cosmic::Action<Message>> {
        let (_id, spawn_window) = window::open(window::Settings::default());
        spawn_window.map(|id| cosmic::Action::App(Message::SettingsWindowCreated(id)))
    }

    fn spawn_edit_style_window(&self, style_id: Uuid) -> Task<cosmic::Action<Message>> {
        let (_id, spawn_window) = window::open(window::Settings {
            size: self.config.edit_style_size(),
            ..Default::default()
        });
        spawn_window
            .map(move |id| cosmic::Action::App(Message::EditStyleWindowCreated(id, style_id)))
    }

    fn close_sticky_windows(&mut self) -> Vec<Task<cosmic::Action<Message>>> {
        let existing_windows = std::mem::take(&mut self.sticky_windows);
        existing_windows
            .into_keys()
            .map(window::close)
            .collect::<Vec<Task<cosmic::Action<Message>>>>()
    }

    fn close_all_windows(&mut self) -> Task<cosmic::Action<Message>> {
        let mut commands = self.close_sticky_windows();
        if let Some(restore_id) = self.restore_window_id {
            commands.push(window::close(restore_id));
        }
        if let Some(settings_id) = self.settings_window_id {
            commands.push(window::close(settings_id));
        }
        if let Some((edit_style_id, _)) = self.edit_style {
            commands.push(window::close(edit_style_id));
        }
        cosmic::task::batch(commands)
    }

    fn build_main_popup_view(&self) -> Element<'_, Message> {
        // let import_available = !self.config.import_file.is_empty();
        // let hide_avail = self.notes.iter_notes().any(|(_, note)| note.is_visible());
        // let show_avail = self.notes.iter_notes().any(|(_, note)| !note.is_visible());
        // let lock_avail = self.notes.iter_notes().any(|(_, note)| !note.is_locked());
        // let restore_avail = self.notes.iter_deleted_notes().next().is_some();

        let save_load = column![
            applet::menu_button(widget::text::body(fl!("load"))).on_press(Message::LoadNotes),
            applet::menu_button(widget::text::body(fl!("save"))).on_press(Message::SaveNotes),
        ];

        let import_export = column![
            applet::menu_button(widget::text::body(fl!("import"))).on_press(Message::ImportNotes),
            applet::menu_button(widget::text::body(fl!("export"))).on_press(Message::ExportNotes),
        ];

        let show_lock = column![
            applet::menu_button(widget::text::body(fl!("hide-all")))
                .on_press(Message::SetAllVisible(false)),
            applet::menu_button(widget::text::body(fl!("show-all")))
                .on_press(Message::SetAllVisible(true)),
            applet::menu_button(widget::text::body(fl!("lock-all"))).on_press(Message::LockAll),
        ];

        let settings_restore = column![
            applet::menu_button(widget::text::body(fl!("restore-notes")))
                .on_press(Message::RestoreNotes),
            applet::menu_button(widget::text::body(fl!("settings")))
                .on_press(Message::OpenSettings),
            //TODO: add "about" item
            applet::menu_button(widget::text::body(fl!("quit"))).on_press(Message::Quit),
        ];

        let spacing = cosmic::theme::spacing();
        let content = column![
            save_load,
            applet::padded_control(widget::divider::horizontal::default())
                .padding([spacing.space_xxs, spacing.space_s]),
            import_export,
            applet::padded_control(widget::divider::horizontal::default())
                .padding([spacing.space_xxs, spacing.space_s]),
            show_lock,
            applet::padded_control(widget::divider::horizontal::default())
                .padding([spacing.space_xxs, spacing.space_s]),
            settings_restore
        ]
        .align_x(Alignment::Start)
        .padding([8, 0]);

        self.core
            .applet
            .popup_container(content)
            .max_height(500.)
            .max_width(500.)
            .into()
    }
}
