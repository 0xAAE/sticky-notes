use super::{
    service::Message,
    utils::{cosmic_font, with_background},
};
use crate::{
    fl,
    icons::IconSet,
    notes::{NoteStyle, NotesCollection},
};
use cosmic::prelude::*;
use cosmic::{
    iced::{Length, widget::keyed_column},
    widget,
};
use uuid::Uuid;

pub fn build_styles_list_view<'a>(
    notes: &'a NotesCollection,
    icons: &IconSet,
    icon_size: u16,
) -> Element<'a, Message> {
    widget::column::with_capacity(2)
        .spacing(cosmic::theme::spacing().space_m)
        .push(widget::text(fl!("styles-list-description")))
        .push(
            widget::scrollable(keyed_column(notes.iter_styles().map(
                |(style_id, style)| {
                    (
                        *style_id,
                        build_style_list_item(*style_id, style, icons, icon_size),
                    )
                },
            )))
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn build_style_list_item<'a>(
    style_id: Uuid,
    style: &'a NoteStyle,
    icons: &IconSet,
    icon_size: u16,
) -> Element<'a, Message> {
    let child = widget::row::with_capacity(3)
        .spacing(cosmic::theme::spacing().space_s)
        .width(Length::Fill)
        .push(
            widget::text(format!(
                "{}, {}: {}",
                style.get_name(),
                fl!("view-style-font"),
                style.get_font().style
            ))
            .font(cosmic_font(style.get_font().style))
            .size(style.get_font().size)
            .width(Length::Fill),
        )
        .push(
            icons
                .edit()
                .apply(widget::button::icon)
                .icon_size(icon_size)
                .on_press(Message::StyleEdit(style_id))
                .width(Length::Shrink),
        )
        .push(
            icons
                .delete()
                .apply(widget::button::icon)
                .icon_size(icon_size)
                .on_press(Message::StyleDelete(style_id))
                .width(Length::Shrink),
        )
        .into();
    with_background(child, style.get_background_color())
}
