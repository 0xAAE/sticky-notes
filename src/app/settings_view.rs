use super::styles_view::build_styles_list_view;
use crate::icons::IconSet;
use crate::notes::NotesCollection;
use crate::{app::Message, fl};
use cosmic::iced::Alignment;
use cosmic::prelude::*;
use cosmic::{iced::Length, widget};

pub fn build_settings_view<'a>(
    notes: &'a NotesCollection,
    icons: &IconSet,
    icon_size: u16,
) -> Element<'a, Message> {
    let styles = notes.get_style_names();
    if styles.is_empty() {
        eprintln!("Not any sticky window style is available");
        return widget::column::with_capacity(1)
            .push(widget::text(fl!("problem-text")))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
    let default_style_index = notes.try_get_default_style_index().ok();
    widget::column::with_capacity(4)
        .spacing(cosmic::theme::spacing().space_s)
        .width(Length::Fill)
        .height(Length::Fill)
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
        .push(widget::button::text(fl!("create-new-style")).on_press(Message::StyleNew))
        .push(build_styles_list_view(notes, icons, icon_size))
        .into()
}
