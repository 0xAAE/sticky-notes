// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use crate::notes::{INVISIBLE_TEXT, NoteData, NoteStyle, NotesCollection};
use cosmic::cosmic_config::{self, ConfigSet, CosmicConfigEntry};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, about::About, menu};
use std::collections::HashMap;
use std::ops::Not;
use uuid::Uuid;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");
const DEF_DATA_FILE: &str = ".config/indicator-stickynotes";

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
        };

        // Create a startup commands
        let mut startup_tasks = vec![app.update_title()];
        // Import notes: if notes is default and empty and if indicator-stickynotes is set try import from it
        if app.notes.is_default() {
            // try read from config or construct default path to indicator-stickynotes data file
            let import_file = app.config.import_file.clone();
            startup_tasks.push(cosmic::task::future(Self::import_notes(import_file)));
        }

        let commands = cosmic::task::batch(startup_tasks);

        (app, commands)
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
        widget::column::with_capacity(1)
            .push(widget::text(INVISIBLE_TEXT))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
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
                self.on_notes_updated(notes);
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

    fn on_notes_updated(&mut self, notes: NotesCollection) {
        self.build_windows(&notes);
        self.notes = notes;
    }

    fn build_windows(&mut self, _notes: &NotesCollection) {
        // todo: rebuild note windows
    }

    fn build_header<'a>(&self, note: &'a NoteData) -> Element<'a, Message> {
        widget::row::with_capacity(2)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .push(widget::text::title1(note.get_title()).width(Length::Fill))
            .into()
    }

    fn build_content<'a>(&'a self, note_id: &Uuid, note: &'a NoteData) -> Element<'a, Message> {
        // read-only note
        if let Some(context) = &self.editing {
            widget::column::with_capacity(2)
                .align_x(Alignment::Start)
                .push(
                    widget::text_editor(&context.content)
                        .on_action(Message::Edit)
                        .height(Length::Fill),
                )
                .push(
                    widget::button::text("Save")
                        .on_press(Message::StopEditNote)
                        .height(Length::Shrink),
                )
                .into()
        } else {
            widget::column::with_capacity(2)
                .align_x(Alignment::Start)
                .push(widget::text::text(note.get_content()).height(Length::Fill))
                .push(
                    widget::button::text("Edit")
                        .on_press(Message::StartEditNote(*note_id))
                        .height(Length::Shrink),
                )
                .into()
        }
    }

    fn build_info<'a>(
        note_id: &'a Uuid,
        note: &'a NoteData,
        style: &'a NoteStyle,
    ) -> Element<'a, Message> {
        let space_s = cosmic::theme::spacing().space_s;
        widget::column::with_capacity(5)
            .align_x(Alignment::Start)
            .height(Length::Shrink)
            .push(
                widget::row::with_capacity(2)
                    .height(Length::Shrink)
                    .push(widget::text::text("id: "))
                    .push(widget::text::text(note_id.to_string())),
            )
            .push(
                widget::row::with_capacity(2)
                    .height(Length::Shrink)
                    .push(widget::text::text("modified: "))
                    .push(widget::text::text(note.get_modified().to_rfc2822())),
            )
            .push(
                widget::row::with_capacity(6)
                    .height(Length::Shrink)
                    .push(widget::text::text("style: "))
                    .push(widget::text::text(&style.name))
                    .push(widget::text::text(", font "))
                    .push(widget::text::text(&style.font_name))
                    .push(widget::text::text(", background "))
                    .push(widget::text::text(format!("{:?}", style.bgcolor))),
            )
            .push(
                widget::row::with_capacity(4)
                    .height(Length::Shrink)
                    .push(widget::text::text("geometry: "))
                    .push(widget::text::text(format!(
                        "{}, {}",
                        note.left(),
                        note.top()
                    )))
                    .spacing(space_s)
                    .push(widget::text::text("x"))
                    .spacing(space_s)
                    .push(widget::text::text(format!(
                        "{}, {}",
                        note.width(),
                        note.height()
                    ))),
            )
            .push(
                widget::row::with_capacity(4)
                    .height(Length::Shrink)
                    .push(widget::text::text("visible: "))
                    .push(widget::text::text(format!("{}", note.is_visible)))
                    .push(widget::text::text(" locked: "))
                    .push(widget::text::text(format!("{}", note.is_locked))),
            )
            .into()
    }
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
