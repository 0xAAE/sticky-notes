// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;
use std::ops::Not;

use crate::{
    config::Config,
    fl, icons,
    notes::{INVISIBLE_TEXT, NoteData, NotesCollection},
};
use cosmic::{
    cosmic_config::{self, ConfigSet, CosmicConfigEntry},
    iced::{
        self, Color, Event, Length, Point, Size, Subscription,
        core::mouse::Button as MouseButton,
        event::Status as EventStatus,
        mouse::Event as MouseEvent,
        window::{self, Event as WindowEvent, Id, Position},
    },
    widget::{self, menu},
};
use cosmic::{iced::Alignment, prelude::*};
use uuid::Uuid;

// const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");
const DEF_DATA_FILE: &str = ".config/indicator-stickynotes";
const ICON_SIZE: u16 = 16;

#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const fn to_usize(v: f32) -> usize {
    v as usize
}

#[inline]
#[allow(clippy::cast_precision_loss)]
const fn to_f32(v: usize) -> f32 {
    v as f32
}

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
    /// currentluy edited content
    editing: Option<EditContext>,
    /// windows by ID
    windows: HashMap<Id, WindowContext>,
    cursor_window: Option<Id>,
    #[cfg(not(feature = "xdg_icons"))]
    icons: icons::IconSet,
    #[cfg(feature = "xdg_icons")]
    icons: icons::IconSet,
}

struct WindowContext {
    note_id: Uuid,
    select_style: Option<Vec<String>>,
}

struct EditContext {
    content: widget::text_editor::Content,
    note_id: Uuid,
    window_id: Id,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    UpdateConfig(Config),
    // Windows
    NewWindow(Id, Uuid), // (window_id, note_id)
    // After menu actions
    LoadNotes,
    SaveNotes,
    ImportNotes,
    ExportNotes,
    SetDefaultStyle(usize), // set deafault style by index
    // notes collection load result shared for Load and Import
    LoadNotesCompleted(NotesCollection),
    LoadNotesFailed(String), // error message
    // export notes result
    ExportNotesCompleted,
    ExportNotesFailed(String), // error message
    // redirect editor actions to the edit context
    Edit(widget::text_editor::Action),
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
            editing: None,
            windows: HashMap::new(),
            cursor_window: None,
            icons: icons::IconSet::new(),
        };

        // Create a startup commands: spawn note windows and (optionally) import indicator-stickynotes data
        let mut startup_tasks: Vec<Task<cosmic::Action<Message>>> = app.spawn_sticky_windows();
        // Import notes: if notes is default and empty (so, it was not loaded from config)
        // and if indicator-stickynotes is set try import from it
        if app.notes.is_default_collection() {
            // try read import_file name from config or construct default path to indicator-stickynotes data file
            let import_file = app.config.import_file.clone();
            startup_tasks.push(cosmic::task::future(Self::import_notes(import_file)));
        }

        (app, cosmic::task::batch(startup_tasks))
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let import_available = !self.config.import_file.is_empty();
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
        if let Some(window_context) = self.windows.get(&id) {
            let note_bg = self
                .notes
                .try_get_note_style(window_context.note_id)
                .map(|style| style.bgcolor);

            let window_interior = self.build_sticky_window_interior(id, window_context);

            let window_content = widget::container(window_interior)
                .class(cosmic::style::Container::custom(move |theme: &Theme| {
                    let cosmic = theme.cosmic();
                    iced::widget::container::Style {
                        icon_color: Some(Color::from(cosmic.background.on)),
                        text_color: Some(Color::from(cosmic.background.on)),
                        background: Some(iced::Background::Color(if let Some(bg) = note_bg {
                            bg
                        } else {
                            cosmic.background.base.into()
                        })),
                        border: iced::Border {
                            radius: cosmic.corner_radii.radius_s.into(),
                            ..Default::default()
                        },
                        shadow: iced::Shadow::default(),
                    }
                }))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .padding(cosmic::theme::spacing().space_s);

            iced::widget::column![window_content].into()
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
            iced::event::listen_with(|evt, status, id| {
                if status == EventStatus::Ignored {
                    match evt {
                        Event::Mouse(MouseEvent::CursorMoved { .. })
                        | Event::Window(WindowEvent::RedrawRequested(_)) => None,
                        Event::Mouse(mouse_event) => {
                            Some(Message::AppMouseEvent((id, mouse_event)))
                        }
                        Event::Window(window_event) => {
                            Some(Message::AppWindowEvent((id, window_event)))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
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
                if self.notes.is_changed() {
                    // todo: ask to overwrite unsaved notes
                    eprintln!("drop unsaved changes while loading collection");
                }
                self.editing = None;
                self.notes = Self::load_notes_or_default(&self.config);
            }

            Message::SaveNotes => {
                if let Err(e) = self.save_notes() {
                    eprintln!("Failed saving notes: {e}");
                }
                self.editing = None;
            }

            Message::ImportNotes => {
                if self.notes.is_changed() {
                    // todo: ask to overwrite unsaved notes
                    eprintln!("drop unsaved changes while importing collection");
                }
                self.editing = None;
                let import_file = self.config.import_file.clone();
                // opposite to other cases return real task instead of none()
                return cosmic::task::future(Self::import_notes(import_file));
            }

            Message::ExportNotes => {
                self.editing = None;
                let export_file = self.config.import_file.clone();
                let notes = self.notes.clone();
                return cosmic::task::future(Self::export_notes(export_file, notes));
            }

            Message::SetDefaultStyle(style_index) => {
                if !self.notes.try_set_default_style_by_index(style_index) {
                    eprintln!(
                        "failed cghanging default sticly window style to {style_index}: no such style"
                    );
                }
            }

            Message::LoadNotesCompleted(notes) => {
                self.notes = notes;
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
                    WindowContext {
                        note_id,
                        select_style: None,
                    },
                );
            }

            // redirect edit actions to the edit context
            Message::Edit(action) => {
                if let Some(context) = &mut self.editing {
                    context.content.perform(action);
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
                    if let Some(context) = self.windows.get(&id) {
                        self.on_start_edit(id, context.note_id);
                    } else {
                        eprintln!("{id}: note is not found to begin edit");
                    }
                } else {
                    self.on_finish_edit();
                }
            }

            Message::NoteStyle(id) => {
                if let Some(context) = self.windows.get_mut(&id) {
                    context.select_style = Some(self.notes.get_style_names());
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
        }
        Task::none()
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        // finish and stage currently edited note if some
        if self.editing.is_some() {
            self.on_finish_edit();
        }
        // save changes if any to persistent storage
        if self.notes.is_changed()
            && let Err(e) = self.save_notes()
        {
            eprintln!("Failed saving notes on exit: {e}");
        }
        // warn if deleted notes were dropped
        let count_deleted = self.notes.get_all_deleted_notes().count();
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
            .and_then(|context| self.notes.try_get_note_mut(&context.note_id))
    }

    fn get_edit_now(&self, window_id: Id) -> Option<&EditContext> {
        self.editing.as_ref().and_then(|context| {
            if context.window_id == window_id {
                Some(context)
            } else {
                None
            }
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

    fn try_get_import_file(configured_import_file: String) -> Option<String> {
        configured_import_file
            .is_empty()
            .not()
            .then_some(configured_import_file)
            .or_else(|| {
                dirs_next::home_dir().map(|mut home| {
                    home.push(DEF_DATA_FILE);
                    home.display().to_string()
                })
            })
    }

    async fn import_notes(configured_import_file: String) -> Message {
        match Self::try_get_import_file(configured_import_file) {
            Some(import_file) => {
                let import_file_owned = import_file.clone();
                match tokio::task::spawn_blocking(move || {
                    NotesCollection::try_import(import_file_owned)
                })
                .await
                {
                    Ok(task) => match task.await {
                        Ok(v) => Message::LoadNotesCompleted(v),
                        Err(e) => {
                            let msg =
                                format!("failed reading notes from {}: {e}", import_file.as_str());
                            Message::LoadNotesFailed(msg)
                        }
                    },
                    Err(e) => Message::LoadNotesFailed(format!("{e}")),
                }
            }
            None => Message::LoadNotesFailed("No import file is set".to_string()),
        }
    }

    async fn export_notes(configured_export_file: String, notes: NotesCollection) -> Message {
        match Self::try_get_import_file(configured_export_file) {
            Some(export_file) => {
                let export_file_owned = export_file.clone();
                match tokio::task::spawn_blocking(move || {
                    NotesCollection::try_export(export_file_owned, notes)
                })
                .await
                {
                    Ok(task) => match task.await {
                        Ok(()) => Message::ExportNotesCompleted,
                        Err(e) => {
                            let msg =
                                format!("failed reading notes from {}: {e}", export_file.as_str());
                            Message::ExportNotesFailed(msg)
                        }
                    },
                    Err(e) => Message::ExportNotesFailed(format!("{e}")),
                }
            }
            None => Message::ExportNotesFailed("No export file is set".to_string()),
        }
    }

    fn on_new_note_window(&mut self) -> Task<cosmic::Action<Message>> {
        if self.editing.is_some() {
            self.on_finish_edit();
        }
        let note_id = self.notes.new_note();
        if let Some(note) = self.notes.try_get_note_mut(&note_id) {
            let (id, task) = Self::spawn_sticky_window(&note_id, note);
            self.on_start_edit(id, note_id);
            task
        } else {
            Task::none()
        }
    }

    fn on_change_note_locking(&mut self, window_id: Id, is_on: bool) {
        if let Some(note) = self.try_get_note_mut(window_id) {
            if is_on {
                note.set_locking(true);
            } else {
                note.set_locking(false);
            }
        } else {
            println!("{window_id}: note is not found to change locking");
        }
    }

    fn on_start_edit(&mut self, window_id: Id, note_id: Uuid) {
        if let Some(note) = self.notes.try_get_note(&note_id) {
            self.editing = Some(EditContext {
                content: widget::text_editor::Content::with_text(note.get_content()),
                note_id,
                window_id,
            });
        } else {
            eprintln!("failed start editing: note {note_id} is not found");
        }
    }

    fn on_finish_edit(&mut self) {
        if let Some(context) = &self.editing {
            if let Some(note) = self.notes.try_get_note_mut(&context.note_id) {
                note.set_content(context.content.text());
            } else {
                eprintln!(
                    "failed to update note {} with text {}",
                    context.note_id,
                    context.content.text()
                );
            }
            self.editing = None;
        }
    }

    fn on_style_selected(&mut self, id: Id, style_index: usize) {
        if let Some(context) = self.windows.get_mut(&id) {
            context.select_style = None;
            if !self
                .notes
                .try_set_note_style_by_index(&context.note_id, style_index)
            {
                eprintln!("{id}: selectd style was not found to assign to sticky window");
            }
        } else {
            eprintln!("{id}: sticky window is not found to change style");
        }
    }

    fn on_delete_note(&mut self, id: Id) -> Task<cosmic::Action<Message>> {
        if let Some(context) = self.windows.remove(&id) {
            self.notes.delete_note(context.note_id);
            window::close(id)
        } else {
            Task::none()
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
            _ => {}
        }
        Task::none()
    }

    fn spawn_sticky_windows(&mut self) -> Vec<Task<cosmic::Action<Message>>> {
        let existing_windows = std::mem::take(&mut self.windows);
        let mut commands: Vec<_> = existing_windows.into_keys().map(window::close).collect();
        commands.extend(self.notes.get_all_notes_mut().map(|(note_id, note)| {
            let (_, spawn_window) = Self::spawn_sticky_window(note_id, note);
            spawn_window
        }));
        commands
    }

    fn spawn_sticky_window(
        note_id: &Uuid,
        note: &mut NoteData,
    ) -> (Id, Task<cosmic::Action<Message>>) {
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

    fn build_main_view(&self) -> Element<'static, Message> {
        let styles = self.notes.get_style_names();
        if styles.is_empty() {
            eprintln!("Not any sticky window style is available");
            return widget::column::with_capacity(1)
                .push(widget::text(INVISIBLE_TEXT))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        let default_style_index = self.notes.default_style_index();
        widget::column::with_capacity(2)
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
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    #[allow(clippy::too_many_lines)]
    fn build_sticky_window_interior<'a>(
        &'a self,
        id: Id,
        window_context: &'a WindowContext,
    ) -> Element<'a, Message> {
        if let Some(edit_context) = self.get_edit_now(id) {
            let note_toolbar = widget::row::with_capacity(1).push(
                self.icons
                    .edit()
                    .apply(widget::button::icon)
                    .icon_size(ICON_SIZE)
                    .on_press(Message::NoteEdit(id, false))
                    .width(Length::Shrink),
            );

            let note_content = widget::container(
                widget::text_editor(&edit_context.content)
                    .on_action(Message::Edit)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill);

            widget::column::with_capacity(2)
                .push(note_toolbar)
                .push(note_content)
                .into()
        } else {
            let is_locked = self
                .notes
                .try_get_note(&window_context.note_id)
                .is_some_and(NoteData::is_locked);

            let mut note_toolbar = widget::row::with_capacity(7)
                .spacing(cosmic::theme::spacing().space_s)
                .push(
                    if is_locked {
                        self.icons.unlock()
                    } else {
                        self.icons.lock()
                    }
                    .apply(widget::button::icon)
                    .icon_size(ICON_SIZE)
                    .on_press(Message::NoteLock(id, !is_locked))
                    .width(Length::Shrink),
                );
            if !is_locked {
                note_toolbar = note_toolbar.push(
                    self.icons
                        .edit()
                        .apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NoteEdit(id, true))
                        .width(Length::Shrink),
                );
                if let Some(styles) = &window_context.select_style {
                    // add style pick list
                    note_toolbar = note_toolbar.push(
                        widget::dropdown(
                            styles,
                            self.notes.try_get_note_style_index(&window_context.note_id),
                            move |index| Message::NoteSyleSelected(id, index),
                        )
                        .placeholder(fl!("select-default-style")),
                    );
                } else {
                    // add button "down"
                    note_toolbar = note_toolbar.push(
                        self.icons
                            .down()
                            .apply(widget::button::icon)
                            .icon_size(ICON_SIZE)
                            .on_press(Message::NoteStyle(id))
                            .width(Length::Shrink),
                    );
                }
                note_toolbar = note_toolbar.push(
                    self.icons
                        .delete()
                        .apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NoteDelete(id))
                        .width(Length::Shrink),
                );
            }
            note_toolbar = note_toolbar
                .push(widget::horizontal_space().width(Length::Fill))
                .push(
                    self.icons
                        .create()
                        .apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NoteNew)
                        .width(Length::Shrink),
                );

            let note_content = widget::column::with_capacity(2)
                .width(Length::Fill)
                .height(Length::Fill)
                .push(widget::text(
                    if let Some(note) = self.notes.try_get_note(&window_context.note_id) {
                        note.get_content()
                    } else {
                        INVISIBLE_TEXT
                    },
                ));

            widget::column::with_capacity(2)
                .push(note_toolbar)
                .push(note_content)
                .into()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    Load,
    Save,
    Import,
    Export,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::Load => Message::LoadNotes,
            MenuAction::Save => Message::SaveNotes,
            MenuAction::Import => Message::ImportNotes,
            MenuAction::Export => Message::ExportNotes,
        }
    }
}
