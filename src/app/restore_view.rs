use super::{service::Message, utils::with_background};
use crate::{
    fl,
    icons::IconSet,
    notes::{NoteData, NotesCollection},
};
use cosmic::prelude::*;
use cosmic::{
    iced::{Length, widget::keyed_column},
    widget,
};
use uuid::Uuid;

pub fn build_restore_view<'a>(
    notes: &'a NotesCollection,
    icons: &IconSet,
    icon_size: u16,
) -> Element<'a, Message> {
    widget::column::with_capacity(2)
        .spacing(cosmic::theme::spacing().space_m)
        .push(widget::text(fl!("recently-deleted-description")))
        .push(
            widget::scrollable(keyed_column(notes.iter_deleted_notes().map(
                |(note_id, note)| {
                    (
                        *note_id,
                        build_note_list_item(*note_id, note, notes, icons, icon_size),
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

fn build_note_list_item<'a>(
    note_id: Uuid,
    note: &'a NoteData,
    notes: &NotesCollection,
    icons: &IconSet,
    icon_size: u16,
) -> Element<'a, Message> {
    if let Ok(style) = notes.try_get_note_style(note_id) {
        let child = widget::row::with_capacity(2)
            .spacing(cosmic::theme::spacing().space_s)
            .width(Length::Fill)
            .push(widget::text(note.get_title()).width(Length::Fill))
            .push(
                icons
                    .undo()
                    .apply(widget::button::icon)
                    .icon_size(icon_size)
                    .on_press(Message::NoteRestore(note_id))
                    .width(Length::Shrink),
            )
            .into();
        with_background(child, style.get_background_color(), style.is_light())
    } else {
        widget::text("problem-text").into()
    }
}
