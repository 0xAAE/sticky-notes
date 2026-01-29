use super::utils::with_background;
use crate::{
    app::Message,
    fl,
    icons::IconSet,
    notes::{NoteData, NoteStyle, NotesCollection},
};
use cosmic::prelude::*;
use cosmic::{
    iced::{Color, Length, widget::keyed_column},
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
                        build_note_list_item(
                            *note_id,
                            note,
                            notes
                                .try_get_note_style(*note_id)
                                .map(NoteStyle::get_background_color)
                                .ok(),
                            icons,
                            icon_size,
                        ),
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
    bgcolor: Option<Color>,
    icons: &IconSet,
    icon_size: u16,
) -> Element<'a, Message> {
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
    if let Some(note_bg) = bgcolor {
        with_background(child, note_bg)
    } else {
        child
    }
}
