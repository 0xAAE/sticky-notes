use crate::{app::Message, config::Config, fl, icons};
use cosmic::{
    applet,
    iced::{
        self, Alignment, Subscription,
        widget::column,
        window::{self, Id},
    },
    widget,
};
use cosmic::{iced::Limits, prelude::*};

pub struct AppletModel {
    // Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    main_popup_id: Option<Id>,
    #[cfg(not(feature = "xdg_icons"))]
    icons: icons::IconSet,
    #[cfg(feature = "xdg_icons")]
    icons: icons::IconSet,
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppletModel {
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
        // Construct the app model with the runtime's core.
        let app = Self {
            core,
            main_popup_id: None,
            icons: icons::IconSet::new(),
        };

        (app, Task::none())
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
        if let Some(window_id) = self.main_popup_id
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
            Message::UpdateConfig(_config) => {
                // self.config = config;
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

            Message::LoadNotes => {
                println!("load notes");
            }

            Message::SaveNotes => {
                println!("save notes");
            }

            Message::ImportNotes => {
                println!("import notes");
            }

            Message::ExportNotes => {
                println!("export notes");
            }

            Message::SetAllVisible(flag) => {
                println!("set notes visibility: {flag}");
            }

            Message::LockAll => {
                println!("lock all notes");
            }

            Message::RestoreNotes => {
                println!("restore notes");
            }

            Message::OpenSettings => {
                println!("open settings");
            }

            Message::Quit => {
                //TODO: send "quit" to service
                println!("quit");
                return iced::exit();
            }

            _ => {
                eprintln!("unexpected message {message:?}");
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(applet::style())
    }
}

impl AppletModel {
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
