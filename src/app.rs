// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use crate::note::{NotesCollection, try_load};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, about::About, icon, menu, nav_bar};
use cosmic::{iced_futures, prelude::*};
use futures_util::SinkExt;
use std::collections::HashMap;
use std::time::Duration;

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
    watch_is_active: bool,
    /// Content itself
    notes: NotesCollection,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    LaunchUrl(String),
    ToggleContextPage(ContextPage),
    ToggleWatch,
    UpdateConfig(Config),
    WatchTick(u32),
    //
    LoadNotesCompleted(NotesCollection),
    LoadNotesFailed(String),
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
        // Create a nav bar with three page items.
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("page-id", num = 1))
            .data::<Page>(Page::Page1)
            .icon(icon::from_name("applications-science-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("page-id", num = 2))
            .data::<Page>(Page::Page2)
            .icon(icon::from_name("applications-system-symbolic"));

        nav.insert()
            .text(fl!("page-id", num = 3))
            .data::<Page>(Page::Page3)
            .icon(icon::from_name("applications-games-symbolic"));

        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
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
                .unwrap_or_default(),
            time: 0,
            watch_is_active: false,
            notes: NotesCollection::new(),
        };

        // Create a startup commands
        let data_file = if app.config.data_file.is_empty() {
            dirs_next::home_dir()
                .map(|mut home| {
                    home.push(DEF_DATA_FILE);
                    home.display().to_string()
                })
                .unwrap_or_default()
        } else {
            app.config.data_file.clone()
        };
        let commands = cosmic::task::batch(vec![
            app.update_title(),
            cosmic::task::future(async move {
                let data_file_clone = data_file.clone();
                match tokio::task::spawn_blocking(move || try_load(data_file_clone)).await {
                    Ok(task) => {
                        let v = match task.await {
                            Ok(v) => v,
                            Err(e) => {
                                eprintln!(
                                    "failed reading notes from {}: {e}, {}",
                                    if data_file.is_empty() {
                                        "<empty>"
                                    } else {
                                        data_file.as_str()
                                    },
                                    e.source().map_or_else(
                                        || "no source error was provided".to_string(),
                                        ToString::to_string
                                    )
                                );
                                NotesCollection::new()
                            }
                        };
                        Message::LoadNotesCompleted(v)
                    }
                    Err(e) => Message::LoadNotesFailed(format!("{e}")),
                }
                // tokio::task::spawn_blocking(move || try_load(DEF_DATA_FILE))
                //     .await
                //     .map(|task| {
                //         task.await
                //             .map(|v| Message::LoadNotesCompleted(v))
                //             .map_err(|e| Message::LoadNotesFailed(format!("{e}")))
                //     })
                //     .map_err(|e| Message::LoadNotesFailed(format!("{e}")))
            }),
        ]);

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
        let content: Element<_> = match self.nav.active_data::<Page>().unwrap() {
            Page::Page1 => {
                let header = widget::row::with_capacity(2)
                    .push(widget::text::title1(fl!("welcome")))
                    .push(widget::text::title3(fl!("page-id", num = 1)))
                    .align_y(Alignment::End)
                    .spacing(space_s);

                let counter_label = ["Watch: ", self.time.to_string().as_str()].concat();
                let section = cosmic::widget::settings::section().add(
                    cosmic::widget::settings::item::builder(counter_label).control(
                        widget::button::text(if self.watch_is_active {
                            "Stop"
                        } else {
                            "Start"
                        })
                        .on_press(Message::ToggleWatch),
                    ),
                );

                widget::column::with_capacity(2)
                    .push(header)
                    .push(section)
                    .spacing(space_s)
                    .height(Length::Fill)
                    .into()
            }

            Page::Page2 => {
                let header = widget::row::with_capacity(2)
                    .push(widget::text::title1(fl!("welcome")))
                    .push(widget::text::title3(fl!("page-id", num = 2)))
                    .align_y(Alignment::End)
                    .spacing(space_s);

                widget::column::with_capacity(1)
                    .push(header)
                    .spacing(space_s)
                    .height(Length::Fill)
                    .into()
            }

            Page::Page3 => {
                let header = widget::row::with_capacity(2)
                    .push(widget::text::title1(fl!("welcome")))
                    .push(widget::text::title3(fl!("page-id", num = 3)))
                    .align_y(Alignment::End)
                    .spacing(space_s);

                widget::column::with_capacity(1)
                    .push(header)
                    .spacing(space_s)
                    .height(Length::Fill)
                    .into()
            }
        };

        widget::container(content)
            .width(600)
            .height(Length::Fill)
            .apply(widget::container)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
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
        if self.watch_is_active {
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

            Message::ToggleWatch => {
                self.watch_is_active = !self.watch_is_active;
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
                if !self.notes.is_empty() {
                    eprintln!(
                        "overwriting existing ({}) notes with just loaded ones",
                        notes.len()
                    );
                }
                self.notes = notes;
            }

            Message::LoadNotesFailed(msg) => eprint!("failed loading notes: {msg}"),
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        // Activate the page in the model.
        self.nav.activate(id);

        self.update_title()
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
}

/// The page to display in the application.
pub enum Page {
    Page1,
    Page2,
    Page3,
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
