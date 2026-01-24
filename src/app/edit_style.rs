use super::{Message, utils::with_background};
use crate::notes::NoteStyle;
use cosmic::prelude::*;
use cosmic::{iced::Color, widget};
use uuid::Uuid;

pub struct EditStyleDialog {
    style_id: Uuid,
    name: String,
    font_name: String,
    bgcolor: Color,
}

impl EditStyleDialog {
    pub fn new(style_id: Uuid, style: &NoteStyle) -> Self {
        Self {
            style_id,
            name: style.get_name().to_string(),
            font_name: style.get_font_name().to_string(),
            bgcolor: style.get_background_color(),
        }
    }

    pub fn update_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn get_id(&self) -> Uuid {
        self.style_id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_font_name(&self) -> &str {
        &self.font_name
    }

    pub fn get_background_color(&self) -> Color {
        self.bgcolor
    }

    pub fn build_dialog_view(&self) -> Element<'_, Message> {
        widget::dialog()
            .title("Edit style")
            .body("Edit style body")
            .control(with_background(
                self.build_edit_style_control(),
                self.bgcolor,
            ))
            .primary_action(widget::button::text("Ok").on_press(Message::EditStyleUpdate))
            .secondary_action(widget::button::text("Cancel").on_press(Message::EditStyleCancel))
            .into()
    }

    fn build_edit_style_control(&self) -> Element<'_, Message> {
        widget::column::with_capacity(3)
            .push(
                widget::row::with_capacity(2)
                    .push(widget::text("Name:"))
                    .push(
                        widget::text_input("", &self.name)
                            .label("Note style name")
                            .on_input(Message::InputStyleName),
                    ),
            )
            .push(
                widget::row::with_capacity(2)
                    .push(widget::text("Font:"))
                    .push(widget::text_input("", &self.font_name).label("Note style font name")),
            )
            .push(
                widget::row::with_capacity(2)
                    .push(widget::text("Background:"))
                    .push(widget::text(format!("{:?}", &self.bgcolor))),
            )
            .into()
    }
}
