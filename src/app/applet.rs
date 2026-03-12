use crate::{
    app::{Command, build_main_popup_view},
    config::Config,
    icons,
};
use cosmic::prelude::*;
use cosmic::{
    applet,
    cosmic_config::{self, CosmicConfigEntry},
    dbus_activation::DbusActivationInterfaceProxy,
    desktop,
    iced::{
        self, Limits, Subscription, theme,
        window::{self, Id},
    },
    widget,
};
use std::{collections::HashMap, time::Duration};

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    UpdateConfig(Config),
    TogglePopup,
    ClosePopupIfOpen,
    Signal(Command),
    SignalResult(Command, bool), // (command, success or not)
    ZbusConnection(zbus::Result<zbus::Connection>),
    DbusProxy(zbus::Result<DbusActivationInterfaceProxy<'static>>),
    Timeout(u64), // waiting for u64 milliseconds have completed
}

pub struct AppletModel {
    // Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    // Configuration data that persists between application runs.
    config: Config,
    main_popup_id: Option<Id>,
    zbus_connection: Option<zbus::Connection>,
    dbus_proxy: Option<DbusActivationInterfaceProxy<'static>>,
    dbus_object_path: String,
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
    const APP_ID: &'static str = super::APP_ID;

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
                        tracing::error!("error loading app config: {why}");
                    }
                    config
                }
            })
            .unwrap_or_default();

        // Construct the app model with the runtime's core.
        let app = Self {
            core,
            config,
            zbus_connection: None,
            dbus_proxy: None,
            dbus_object_path: format!("/{}", Self::APP_ID.replace('.', "/")),
            main_popup_id: None,
            icons: icons::IconSet::new(),
        };

        let zbus_session_cmd = Task::perform(zbus::Connection::session(), |res| {
            Message::ZbusConnection(res).into()
        });

        (app, zbus_session_cmd)
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
            build_main_popup_view(self.core(), Message::Signal, |_| true)
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
                    for e in update.errors {
                        tracing::error!("config error: {e}");
                    }
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
            Message::UpdateConfig(config) => {
                self.config = config;
            }

            Message::ClosePopupIfOpen => {
                return self.close_popup();
            }

            Message::TogglePopup => {
                if self.main_popup_id.is_some() {
                    return self.close_popup();
                }
                return self.open_popup();
            }

            Message::ZbusConnection(Err(e)) => {
                tracing::error!("failed to connect to session dbus: {e}");
            }

            Message::ZbusConnection(Ok(conn)) => {
                tracing::info!("established connection to dbus");
                self.zbus_connection = Some(conn);
                return self.try_build_dbus_proxy();
            }

            Message::DbusProxy(Err(e)) => {
                tracing::error!("failed building dbus proxy: {e}");
            }

            Message::DbusProxy(Ok(proxy)) => {
                tracing::info!(
                    "successfully built dbus proxy client, testing service availability then"
                );
                self.dbus_proxy = Some(proxy);
                // test service availability, this will try to launch service if unavailable:
                return self.send_command_via_dbus(Command::Connect);
            }

            Message::Signal(command) => {
                tracing::debug!("requested {command}");
                // close popup menu, then send signal via DBus
                return Task::done(Message::ClosePopupIfOpen.into())
                    .chain(self.send_command_via_dbus(command));
            }

            Message::SignalResult(command, success) => {
                if success {
                    tracing::debug!("successfully sent {command}");
                } else {
                    tracing::warn!("failed sending {command}");
                    if command == Command::Connect {
                        // waiting for a while to prevent spamming with attempts to connect the service, then repeat connecting again
                        let pause = self.config.connect_service_pause_ms;
                        let command_clone = command.clone();
                        return Task::future(async move {
                            tracing::warn!("wait for {pause} msec then repeat {command_clone}");
                            tokio::time::sleep(Duration::from_millis(pause)).await;
                            Message::Timeout(pause).into()
                        })
                        .chain(self.send_command_via_dbus(command));
                    }
                }
                if let Command::Quit = command {
                    tracing::info!("finish working due to QUIT was sent to service");
                    return iced::exit();
                }
            }

            Message::Timeout(ms) => {
                tracing::debug!("waiting for {ms} msec have completed");
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<theme::Style> {
        Some(applet::style())
    }
}

impl AppletModel {
    fn close_popup(&mut self) -> Task<cosmic::Action<Message>> {
        if let Some(p) = self.main_popup_id.take() {
            tracing::debug!("destroying popup menu");
            cosmic::iced::platform_specific::shell::commands::popup::destroy_popup(p)
        } else {
            Task::none()
        }
    }

    fn open_popup(&mut self) -> Task<cosmic::Action<Message>> {
        tracing::debug!("build popup menu");
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
            .max_height(500.0)
            .max_width(500.0);
        cosmic::iced::platform_specific::shell::commands::popup::get_popup(popup_settings)
    }

    fn try_build_dbus_proxy(&self) -> Task<cosmic::Action<Message>> {
        if let Some(zbus_conn) = self.zbus_connection.clone() {
            tracing::info!("try building proxy client");
            let path = self.dbus_object_path.clone();
            match DbusActivationInterfaceProxy::builder(&zbus_conn)
                .destination(<Self as cosmic::Application>::APP_ID)
                //.ok()
                .and_then(|b| b.path(path))
                .and_then(|b| b.destination(<Self as cosmic::Application>::APP_ID))
            {
                Ok(proxy_builder) => {
                    return Task::perform(async move { proxy_builder.build().await }, |res| {
                        Message::DbusProxy(res).into()
                    });
                }
                Err(e) => tracing::error!("failed building dbus proxy client: {e}"),
            }
        } else {
            tracing::info!("failed building dbus proxy client: connection is not established yet");
        }
        Task::none()
    }

    fn send_command_via_dbus(&self, command: Command) -> Task<cosmic::Action<Message>> {
        if let Some(mut proxy) = self.dbus_proxy.clone() {
            let command_str = command.to_string();
            let service_exec = self.config.service_bin.clone();
            Task::future(async move {
                if let Err(e) = proxy
                    .activate_action(command_str.as_str(), Vec::new(), HashMap::new())
                    .await
                {
                    tracing::error!("failed sending {command_str}: {e}");
                    if command == Command::Quit {
                        tracing::info!("skip trying to launch service because stop working");
                    } else {
                        //todo: test error before spawning service; valid candidates are: InterfaceNotFound, Failure(e)
                        tracing::info!("trying to launch notes-service binary: {}", &service_exec);
                        desktop::spawn_desktop_exec(
                            service_exec.as_str(),
                            Vec::<(String, String)>::new(),
                            Some(<Self as cosmic::Application>::APP_ID),
                            false,
                        )
                        .await;
                    }
                    Message::SignalResult(command, false).into()
                } else {
                    Message::SignalResult(command, true).into()
                }
            })
        } else {
            Task::none()
        }
    }
}
