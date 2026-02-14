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
    iced::{Color, Length, window::Id},
    widget::{self, text_editor::Action},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StickyWindowError {
    #[error("already in edit mode")]
    AlreadyEditing,
    #[error("not in edit mode")]
    EditingIsOff,
}

pub struct StickyWindow {
    note_id: Uuid,
    edit_context: Option<EditContext>,
    style_names: Option<Vec<String>>,
    icon_size: u16,
}

struct EditContext {
    /// currently edited content
    content: widget::text_editor::Content,
}

impl StickyWindow {
    pub fn new(note_id: Uuid, icon_size: u16) -> Self {
        Self {
            note_id,
            edit_context: None,
            style_names: None,
            icon_size,
        }
    }

    pub fn get_note_id(&self) -> Uuid {
        self.note_id
    }

    pub fn start_edit(&mut self, init_content: &str) -> Result<(), StickyWindowError> {
        if self.edit_context.is_some() {
            Err(StickyWindowError::AlreadyEditing)
        } else {
            self.edit_context = Some(EditContext {
                content: widget::text_editor::Content::with_text(init_content),
            });
            Ok(())
        }
    }

    pub fn finish_edit(&mut self) -> Result<String, StickyWindowError> {
        self.edit_context
            .take()
            .map(|context| context.content.text())
            .ok_or(StickyWindowError::EditingIsOff)
    }

    pub fn do_edit_action(&mut self, action: Action) -> Result<(), StickyWindowError> {
        self.edit_context
            .as_mut()
            .map(|context| context.content.perform(action))
            .ok_or(StickyWindowError::EditingIsOff)
    }

    pub fn allow_select_style(&mut self, style_names: Vec<String>) {
        self.style_names = Some(style_names);
    }

    pub fn disable_select_style(&mut self) {
        self.style_names = None;
    }

    #[allow(clippy::too_many_lines)]
    pub fn build_view<'a>(
        &'a self,
        window_id: Id,
        notes: &'a NotesCollection,
        icons: &IconSet,
    ) -> Element<'a, Message> {
        if let Some(edit_context) = &self.edit_context {
            let bgcolor = notes
                .try_get_note_style(self.get_note_id())
                .map_or(Color::WHITE, NoteStyle::get_background_color);

            let note_toolbar = widget::row::with_capacity(1).push(
                icons
                    .checked()
                    .apply(widget::button::icon)
                    .icon_size(self.icon_size)
                    .on_press(Message::NoteEdit(window_id, false))
                    .width(Length::Shrink),
            );

            let note_content = widget::container(
                widget::text_editor(&edit_context.content)
                    .on_action(move |act| Message::Edit(window_id, act))
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill);

            with_background(
                widget::column::with_capacity(2)
                    .push(note_toolbar)
                    .push(note_content)
                    .into(),
                bgcolor,
            )
        } else if let Ok(note) = notes.try_get_note(&self.note_id)
            && let Ok(style) = notes.try_get_style(&note.style())
        {
            let is_locked = note.is_locked();

            let mut note_toolbar = widget::row::with_capacity(7)
                .spacing(cosmic::theme::spacing().space_s)
                .push(
                    if is_locked {
                        icons.unlock()
                    } else {
                        icons.lock()
                    }
                    .apply(widget::button::icon)
                    .icon_size(self.icon_size)
                    .on_press(Message::NoteLock(window_id, !is_locked))
                    .width(Length::Shrink),
                );
            if !is_locked {
                note_toolbar = note_toolbar.push(
                    icons
                        .edit()
                        .apply(widget::button::icon)
                        .icon_size(self.icon_size)
                        .on_press(Message::NoteEdit(window_id, true))
                        .width(Length::Shrink),
                );
                if let Some(styles) = &self.style_names {
                    // add style pick list
                    note_toolbar = note_toolbar.push(
                        widget::dropdown(
                            styles,
                            notes
                                .try_get_note_style_index(self.note_id)
                                .map_err(|e| tracing::error!("failed to get style index: {e}"))
                                .ok(),
                            move |index| Message::NoteStyleSelected(window_id, index),
                        )
                        .placeholder(fl!("select-default-style")),
                    );
                } else {
                    // add button "down"
                    note_toolbar = note_toolbar.push(
                        icons
                            .down()
                            .apply(widget::button::icon)
                            .icon_size(self.icon_size)
                            .on_press(Message::NoteStyle(window_id))
                            .width(Length::Shrink),
                    );
                }
                note_toolbar = note_toolbar.push(
                    icons
                        .delete()
                        .apply(widget::button::icon)
                        .icon_size(self.icon_size)
                        .on_press(Message::NoteDelete(window_id))
                        .width(Length::Shrink),
                );
            }
            note_toolbar = note_toolbar
                .push(widget::horizontal_space().width(Length::Fill))
                .push(
                    icons
                        .create()
                        .apply(widget::button::icon)
                        .icon_size(self.icon_size)
                        .on_press(Message::NoteNew)
                        .width(Length::Shrink),
                );

            let note_content = widget::column::with_capacity(2)
                .width(Length::Fill)
                .height(Length::Fill)
                .push(
                    widget::text(note.get_content())
                        .font(cosmic_font(style.get_font().style))
                        .size(style.get_font().size),
                );

            with_background(
                widget::column::with_capacity(2)
                    .push(note_toolbar)
                    .push(note_content)
                    .into(),
                style.get_background_color(),
            )
        } else {
            // build problem view
            widget::text("problem-text").into()
        }
    }
}
