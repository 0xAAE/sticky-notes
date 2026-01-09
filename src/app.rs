// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use crate::notes::{INVISIBLE_TEXT, NoteData, NoteStyle, NotesCollection};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, ConfigSet, CosmicConfigEntry};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, about::About, icon, menu, nav_bar};
use cosmic::{iced_futures, prelude::*};
use futures_util::SinkExt;
use std::collections::HashMap;
use std::time::Duration;
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
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// The about page for this app.
    about: About,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// Time active
    time: u32,
    /// Toggle the watch subscription
    info_is_active: bool,
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
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    ToggleInfo,
    WatchTick(u32),
    // Loading notes collection
    LoadNotesCompleted(NotesCollection),
    LoadNotesFailed(String),
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
        let notes = if config.notes.is_empty() {
            NotesCollection::default()
        } else {
            NotesCollection::try_read(&config.notes)
                .map_err(|e| {
                    eprintln!(
                        "failed loading notes from {}/{}/notes: {e}",
                        Self::APP_ID,
                        Config::VERSION
                    );
                })
                .unwrap_or_default()
        };

        // Create a nav bar with three page items.
        let nav = Self::build_nav(&notes);

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config,
            time: 0,
            info_is_active: false,
            notes,
            editing: None,
        };

        // Create a startup commands
        let mut startup_tasks = vec![app.update_title()];
        // Import notes: if notes is default and empty and if indicator-stickynotes is set try import from it
        if app.notes.is_default() {
            // try read from config or construct default path to indicator-stickynotes data file
            if app.config.import_file.is_empty()
                && let Some(import_file) = dirs_next::home_dir().map(|mut home| {
                    home.push(DEF_DATA_FILE);
                    home.display().to_string()
                })
            {
                startup_tasks.push(cosmic::task::future(async move {
                    let data_file_clone = import_file.clone();
                    match tokio::task::spawn_blocking(move || {
                        NotesCollection::try_import(data_file_clone)
                    })
                    .await
                    {
                        Ok(task) => match task.await {
                            Ok(v) => Message::LoadNotesCompleted(v),
                            Err(e) => {
                                let msg = format!(
                                    "failed reading notes from {}: {e}",
                                    if import_file.is_empty() {
                                        "<empty>"
                                    } else {
                                        import_file.as_str()
                                    },
                                );
                                Message::LoadNotesFailed(msg)
                            }
                        },
                        Err(e) => Message::LoadNotesFailed(format!("{e}")),
                    }
                }));
            } else {
                eprintln!(
                    "Neither {}/{}/notes nor indicator-stickynotes data file provide notes collection, starting with new one",
                    Self::APP_ID,
                    Config::VERSION
                );
            }
        }

        let commands = cosmic::task::batch(startup_tasks);

        (app, commands)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        let space_s = cosmic::theme::spacing().space_s;
        let page: Element<_> = if let Some(note_id) = self.nav.active_data::<Uuid>()
            && let Some(note) = self.notes.try_get_note(note_id)
            && let Some(style) = self.notes.get_style_or_default(&note.style)
        {
            // combine text + (optional) info into content
            let content = {
                let mut content = widget::column::with_capacity(2)
                    .spacing(space_s)
                    .push(self.build_content(note_id, note));
                if self.info_is_active {
                    content = content
                        .push(widget::divider::horizontal::light())
                        .push(Self::build_info(note_id, note, style));
                }
                widget::container(content).height(Length::Fill)
            };
            // combine title + content into page
            widget::column::with_capacity(2)
                .height(Length::Fill)
                .spacing(space_s)
                .push(self.build_header(note))
                .push(content)
                .into()
        } else {
            // unreachable!();
            // Construct a dummy page wich has been never observed
            let text = widget::row::with_capacity(1)
                .push(widget::text::text(INVISIBLE_TEXT))
                .align_y(Alignment::Start)
                .spacing(space_s);
            widget::column::with_capacity(1)
                .spacing(space_s)
                .height(Length::Fill)
                .push(text)
                .into()
        };

        widget::container(page)
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
        let mut subscriptions = vec![
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

        // Conditionally enables a timer that emits a message every second.
        if self.info_is_active {
            subscriptions.push(Subscription::run(|| {
                iced_futures::stream::channel(1, |mut emitter| async move {
                    let mut time = 1;
                    let mut interval = tokio::time::interval(Duration::from_secs(1));

                    loop {
                        interval.tick().await;
                        _ = emitter.send(Message::WatchTick(time)).await;
                        time += 1;
                    }
                })
            }));
        }

        Subscription::batch(subscriptions)
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::WatchTick(time) => {
                self.time = time;
            }

            Message::ToggleInfo => {
                self.info_is_active = !self.info_is_active;
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },

            Message::LoadNotesCompleted(notes) => {
                self.on_notes_updated(notes);
            }

            Message::LoadNotesFailed(msg) => {
                eprintln!("failed loading notes: {msg}");
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

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        // Activate the page in the model.
        self.nav.activate(id);

        self.update_title()
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        if self.notes.is_changed()
            && let Ok(whole_config) = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
        {
            match self.notes.try_write() {
                Ok(json) => {
                    let tx = whole_config.transaction();
                    if let Err(e) = tx.set("notes", json) {
                        eprintln!("Failed updating notes in config: {e}");
                    }
                    if let Err(e) = tx.commit() {
                        eprintln!("Failed saving config updated: {e}");
                    }
                }
                Err(e) => eprintln!("failed serializing notes: {e}"),
            }
        }
        None
    }
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
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
        self.notes = notes;
        // Create a nav bar with three page items.
        self.nav = Self::build_nav(&self.notes);
    }

    fn build_nav(notes: &NotesCollection) -> nav_bar::Model {
        let mut nav = nav_bar::Model::default();
        for note in notes.get_all_notes() {
            nav.insert()
                .text(note.1.get_title().to_string())
                .data::<Uuid>(*note.0)
                .icon(icon::from_name("applications-science-symbolic"));
        }
        nav.activate_position(0);
        nav
    }

    fn build_header<'a>(&self, note: &'a NoteData) -> Element<'a, Message> {
        let space_s = cosmic::theme::spacing().space_s;
        widget::row::with_capacity(2)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .push(widget::text::title1(note.get_title()).width(Length::Fill))
            .spacing(space_s)
            .push(
                widget::button::text(if self.info_is_active {
                    "Hide info"
                } else {
                    "View info"
                })
                .on_press(Message::ToggleInfo)
                .width(Length::Shrink),
            )
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
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}
