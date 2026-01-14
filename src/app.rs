// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;
use std::ops::Not;

use crate::config::Config;
use crate::fl;
use crate::notes::{INVISIBLE_TEXT, NoteData, NoteStyle, NotesCollection};
use cosmic::prelude::*;
use cosmic::{
    cosmic_config::{self, ConfigSet, CosmicConfigEntry},
    iced::{
        self, Color, Event, Length, Point, Size, Subscription,
        core::mouse::Button as MouseButton,
        event::Status as EventStatus,
        mouse::Event as MouseEvent,
        widget::container as iced_container,
        widget::{column, row},
        window::{self, Event as WindowEvent, Id, Position},
    },
    style,
    widget::{self, about::About, menu},
};
use uuid::Uuid;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");
const DEF_DATA_FILE: &str = ".config/indicator-stickynotes";

// embedded SVG bytes
const ICON_UNLOCKED: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/changes-allow-symbolic.svg");
const ICON_LOCKED: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/changes-prevent-symbolic.svg");
const ICON_NEW: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/document-new-symbolic.svg");
const ICON_DELETE: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/edit-delete-symbolic.svg");
const ICON_EDIT: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/edit-symbolic.svg");
const ICON_DOWN: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/pan-down-symbolic.svg");
const ICON_PIN: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/pin-symbolic.svg");

// system wide installed icons
const XDG_UNLOCKED: &str = "changes-allow-symbolic";
const XDG_LOCKED: &str = "changes-prevent-symbolic";
const XDG_NEW: &str = "document-new-symbolic";
const XDG_DELETE: &str = "edit-delete-symbolic";
const XDG_EDIT: &str = "edit-symbolic";
const XDG_DOWN: &str = "pan-down-symbolic";
const XDG_PIN: &str = "pin-symbolic";

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
#[allow(clippy::zero_sized_map_values)] // key_binds: HashMap<menu::KeyBind, MenuAction>: map with zero-sized value type
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The about page for this app.
    about: About,
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
}

struct WindowContext {
    note_id: Uuid,
    content: String,
}

struct EditContext {
    content: widget::text_editor::Content,
    note_id: Uuid,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    LaunchUrl(String),
    UpdateConfig(Config),
    // Windows
    NewWindow(Id, Uuid), // (window_id, note_id)
    // After menu actions
    LoadNotes,
    SaveNotes,
    ImportNotes,
    ExportNotes,
    // notes collection load result shared for Load and Import
    LoadNotesCompleted(NotesCollection),
    LoadNotesFailed(String), // error message
    // export notes result
    ExportNotesCompleted,
    ExportNotesFailed(String), // error message
    // Edit currently selected (displayed) note, contains id of the note
    StartEditNote(Uuid),
    StopEditNote,
    Edit(widget::text_editor::Action),
    // iced "system" events handling
    AppWindowEvent((Id, WindowEvent)),
    AppMouseEvent((Id, MouseEvent)),
    // response on window::get_position() request
    WindowPositionResponse((Id, Option<Point>)),
    // note image button actions
    NoteLock(Id, bool),
    NotePin(Id, bool),
    NoteEdit(Id, bool),
    NoteColor(Id),
    NoteNew,
    NoteDelete(Id),
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
        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

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
            about,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config,
            notes,
            editing: None,
            windows: HashMap::new(),
            cursor_window: None,
        };

        // Create a startup commands: spawn note windows and (optionally) import indicator-stickynotes data
        let mut startup_tasks: Vec<Task<cosmic::Action<Message>>> = app.spawn_windows();
        // Import notes: if notes is default and empty (so, it was not loaded from config)
        // and if indicator-stickynotes is set try import from it
        if app.notes.is_default() {
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
        Self::build_undesired_view()
    }

    /// Constructs views for other windows.
    fn view_window(&self, id: Id) -> Element<'_, Self::Message> {
        if let Some(window_context) = self.windows.get(&id) {
            let note_bg = self
                .notes
                .try_get_note_style(window_context.note_id)
                .map(|style| style.bgcolor);

            // using embedded SVG icons
            const ICON_SIZE: u16 = 16;
            let lock = widget::icon::from_svg_bytes(ICON_UNLOCKED);
            let _unlock = widget::icon::from_svg_bytes(ICON_LOCKED);
            let pin = widget::icon::from_svg_bytes(ICON_PIN);
            let edit = widget::icon::from_svg_bytes(ICON_EDIT);
            let down = widget::icon::from_svg_bytes(ICON_DOWN);
            let create = widget::icon::from_svg_bytes(ICON_NEW);
            let delete = widget::icon::from_svg_bytes(ICON_DELETE);

            // or using system XDG icons by names
            // let lock = widget::icon::from_name(XDG_UNLOCKED);
            // let _unlock = widget::icon::from_name(XDG_LOCKED);
            // let pin = widget::icon::from_name(XDG_PIN);
            // let edit = widget::icon::from_name(XDG_EDIT);
            // let down = widget::icon::from_name(XDG_DOWN);
            // let create = widget::icon::from_name(XDG_NEW);
            // let delete = widget::icon::from_name(XDG_DELETE);

            let note_toolbar = widget::row::with_capacity(7)
                .spacing(cosmic::theme::spacing().space_s)
                .push(
                    lock.apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NoteLock(id, true))
                        .width(Length::Shrink),
                )
                .push(
                    pin.apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NotePin(id, true))
                        .width(Length::Shrink),
                )
                .push(
                    edit.apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NoteEdit(id, true))
                        .width(Length::Shrink),
                )
                .push(
                    down.apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NoteColor(id))
                        .width(Length::Shrink),
                )
                .push(
                    create
                        .apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NoteNew)
                        .width(Length::Shrink),
                )
                .push(widget::horizontal_space().width(Length::Fill))
                .push(
                    delete
                        .apply(widget::button::icon)
                        .icon_size(ICON_SIZE)
                        .on_press(Message::NoteDelete(id))
                        .width(Length::Shrink),
                );

            let note_content = widget::column::with_capacity(2)
                .width(Length::Fill)
                .height(Length::Fill)
                .push(widget::text(&window_context.content));

            let window_interior = column![note_toolbar, note_content];

            let window_content = widget::container(window_interior)
                .class(style::Container::custom(move |theme: &Theme| {
                    let cosmic = theme.cosmic();
                    iced_container::Style {
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

            // to display header bar above content:
            // let focused = self
            //     .core()
            //     .focused_window()
            //     .map(|i| i == id)
            //     .unwrap_or(false);
            // column![
            //     widget::header_bar()
            //         .start(widget::text(format!("Id: {id}")))
            //         .focused(focused),
            //     window_content
            // ]
            // .into()

            // display only content without header bar:
            column![window_content].into()
        } else {
            Self::build_undesired_view()
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
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateConfig(config) => {
                self.config = config;
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },

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

            Message::LoadNotesCompleted(notes) => {
                self.notes = notes;
                return cosmic::task::batch(self.spawn_windows());
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
            Message::NewWindow(window_id, note_id) => {
                self.on_new_note_window(window_id, note_id);
            }

            // messages related to edit note mode
            Message::StartEditNote(note_id) => {
                self.on_start_edit(note_id);
            }

            Message::StopEditNote => {
                self.on_finish_edit();
            }

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
                    note.set_position(point.x as usize, point.y as usize);
                }
            }

            Message::NoteLock(id, is_on) => {
                println!("{id}: {}", if is_on { "lock note" } else { "unlock note" });
            }

            Message::NotePin(id, is_on) => {
                println!("{id}: {}", if is_on { "pin note" } else { "unpin note" });
            }

            Message::NoteEdit(id, is_on) => {
                println!(
                    "{id}: {}",
                    if is_on {
                        "begin edit note"
                    } else {
                        "stop edit and save note"
                    }
                );
            }

            Message::NoteColor(id) => {
                println!("{id}: change style");
            }

            Message::NoteNew => {
                println!("new note");
            }

            Message::NoteDelete(id) => {
                println!("{id}: delete note");
            }
        }
        Task::none()
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        if self.notes.is_changed()
            && let Err(e) = self.save_notes()
        {
            eprint!("Failed saving notes on exit: {e}");
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

    fn try_get_active_note_mut(&mut self) -> Option<&mut NoteData> {
        self.cursor_window.and_then(|id| self.try_get_note_mut(id))
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(edit) = &self.editing {
            window_title.push_str(" — ");
            window_title.push_str(edit.note_id.to_string().as_str());
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
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

    fn on_start_edit(&mut self, note_id: Uuid) {
        if let Some(note) = self.notes.try_get_note(&note_id) {
            self.editing = Some(EditContext {
                content: widget::text_editor::Content::with_text(note.get_content()),
                note_id,
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
                    note.set_size(size.width as usize, size.height as usize);
                }
            }
            WindowEvent::Moved(point) => {
                if let Some(note) = self.try_get_note_mut(id) {
                    note.set_position(point.x as usize, point.y as usize);
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn spawn_windows(&mut self) -> Vec<Task<cosmic::Action<Message>>> {
        let existing_windows = std::mem::take(&mut self.windows);
        let mut commands: Vec<_> = existing_windows.into_keys().map(window::close).collect();
        commands.extend(self.notes.get_all_notes_mut().map(|(note_id, note)| {
            let (id, spawn_window) = window::open(window::Settings {
                position: Position::Specific(Point::new(note.left() as f32, note.top() as f32)),
                size: Size::new(note.width() as f32, note.height() as f32),
                decorations: false,
                ..Default::default()
            });
            note.assign_window(id);
            self.windows.insert(
                id,
                WindowContext {
                    note_id: *note_id,
                    content: note.get_content().to_string(),
                },
            );
            let note_id = *note_id;
            spawn_window.map(move |id| cosmic::Action::App(Message::NewWindow(id, note_id)))
        }));
        commands
    }

    fn on_new_note_window(&self, _window_id: Id, _note_id: Uuid) {}

    fn build_undesired_view() -> Element<'static, Message> {
        widget::column::with_capacity(1)
            .push(widget::text(INVISIBLE_TEXT))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    // fn build_header<'a>(&self, note: &'a NoteData) -> Element<'a, Message> {
    //     widget::row::with_capacity(2)
    //         .align_y(Alignment::Center)
    //         .width(Length::Fill)
    //         .push(widget::text::title1(note.get_title()).width(Length::Fill))
    //         .into()
    // }

    // fn build_content<'a>(&'a self, note_id: &Uuid, note: &'a NoteData) -> Element<'a, Message> {
    //     // read-only note
    //     if let Some(context) = &self.editing {
    //         widget::column::with_capacity(2)
    //             .align_x(Alignment::Start)
    //             .push(
    //                 widget::text_editor(&context.content)
    //                     .on_action(Message::Edit)
    //                     .height(Length::Fill),
    //             )
    //             .push(
    //                 widget::button::text("Save")
    //                     .on_press(Message::StopEditNote)
    //                     .height(Length::Shrink),
    //             )
    //             .into()
    //     } else {
    //         widget::column::with_capacity(2)
    //             .align_x(Alignment::Start)
    //             .push(widget::text::text(note.get_content()).height(Length::Fill))
    //             .push(
    //                 widget::button::text("Edit")
    //                     .on_press(Message::StartEditNote(*note_id))
    //                     .height(Length::Shrink),
    //             )
    //             .into()
    //     }
    // }

    // fn build_info<'a>(
    //     note_id: &'a Uuid,
    //     note: &'a NoteData,
    //     style: &'a NoteStyle,
    // ) -> Element<'a, Message> {
    //     let space_s = cosmic::theme::spacing().space_s;
    //     widget::column::with_capacity(5)
    //         .align_x(Alignment::Start)
    //         .height(Length::Shrink)
    //         .push(
    //             widget::row::with_capacity(2)
    //                 .height(Length::Shrink)
    //                 .push(widget::text::text("id: "))
    //                 .push(widget::text::text(note_id.to_string())),
    //         )
    //         .push(
    //             widget::row::with_capacity(2)
    //                 .height(Length::Shrink)
    //                 .push(widget::text::text("modified: "))
    //                 .push(widget::text::text(note.get_modified().to_rfc2822())),
    //         )
    //         .push(
    //             widget::row::with_capacity(6)
    //                 .height(Length::Shrink)
    //                 .push(widget::text::text("style: "))
    //                 .push(widget::text::text(&style.name))
    //                 .push(widget::text::text(", font "))
    //                 .push(widget::text::text(&style.font_name))
    //                 .push(widget::text::text(", background "))
    //                 .push(widget::text::text(format!("{:?}", style.bgcolor))),
    //         )
    //         .push(
    //             widget::row::with_capacity(4)
    //                 .height(Length::Shrink)
    //                 .push(widget::text::text("geometry: "))
    //                 .push(widget::text::text(format!(
    //                     "{}, {}",
    //                     note.left(),
    //                     note.top()
    //                 )))
    //                 .spacing(space_s)
    //                 .push(widget::text::text("x"))
    //                 .spacing(space_s)
    //                 .push(widget::text::text(format!(
    //                     "{}, {}",
    //                     note.width(),
    //                     note.height()
    //                 ))),
    //         )
    //         .push(
    //             widget::row::with_capacity(4)
    //                 .height(Length::Shrink)
    //                 .push(widget::text::text("visible: "))
    //                 .push(widget::text::text(format!("{}", note.is_visible)))
    //                 .push(widget::text::text(" locked: "))
    //                 .push(widget::text::text(format!("{}", note.is_locked))),
    //         )
    //         .into()
    // }
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
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
