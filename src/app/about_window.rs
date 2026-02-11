use super::service::Message;
use crate::{fl, icons::IconSet};
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::{self, about::About};

const THIS_REPO: &str = env!("CARGO_PKG_REPOSITORY");
const AUTHOR_PROFILE: &str = "https://github.com/0xAAE";
const AUTHOR_EMAIL: &str = "avramenko.ae@yandex.ru";
const INSPIRED_REPO: &str = "https://github.com/umangv/indicator-stickynotes";
const COSMIC_REPO: &str = "https://github.com/pop-os/cosmic-epoch";
const MPL2_URL: &str = "https://www.mozilla.org/en-US/MPL/2.0";

pub struct AboutWindow {
    about: About,
}

impl AboutWindow {
    pub fn new(icons: &IconSet) -> Self {
        Self {
            about: About::default()
                .icon(icons.notes())
                .name(fl!("app-title"))
                .author(fl!("about-author"))
                .version(env!("CARGO_PKG_VERSION"))
                .comments(fl!("about-comments"))
                .copyright(fl!("about-copyright"))
                .license(env!("CARGO_PKG_LICENSE"))
                .license_url(MPL2_URL)
                .developers(vec![(fl!("about-author").as_str(), AUTHOR_EMAIL)])
                .links(vec![
                    (fl!("project-repo"), THIS_REPO),
                    (fl!("author-profile"), AUTHOR_PROFILE),
                    (fl!("inspired-repo"), INSPIRED_REPO),
                    (fl!("cosmic-repo"), COSMIC_REPO),
                ]),
        }
    }

    pub fn build_view(&self) -> Element<'_, Message> {
        widget::container(widget::about(&self.about, |url| {
            Message::OpenUrl(url.to_string())
        }))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
