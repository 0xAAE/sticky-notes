use super::service::Message;
use crate::{fl, icons::IconSet};
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::{self, about::About};

pub struct AboutWindow {
    about: About,
}

impl AboutWindow {
    pub fn new() -> Self {
        Self {
            about: About::default().name("name"),
        }
    }

    pub fn build_view<'a>(&'a self, _icons: &IconSet) -> Element<'a, Message> {
        widget::container(widget::about(&self.about, |_url| Message::Ignore))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
