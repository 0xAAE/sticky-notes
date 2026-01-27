use super::{Message, utils::with_background};
use crate::fl;
use crate::notes::NoteStyle;
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::color_picker::ColorPickerUpdate;
use cosmic::{iced::Color, widget};
use palette::FromColor;
use uuid::Uuid;

pub struct EditStyleDialog {
    style_id: Uuid,
    name: String,
    font_name: String,
    bgcolor: Color,
    color_picker_model: widget::ColorPickerModel,
}

impl EditStyleDialog {
    pub fn new(style_id: Uuid, style: &NoteStyle) -> Self {
        Self {
            style_id,
            name: style.get_name().to_string(),
            font_name: style.get_font_name().to_string(),
            bgcolor: style.get_background_color(),
            color_picker_model: widget::ColorPickerModel::new(
                fl!("edit-style-hex"),
                fl!("edit-style-rgb"),
                Some(style.get_background_color()),
                Some(style.get_background_color()),
            ),
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

    pub fn on_color_picker_update(
        &mut self,
        event: ColorPickerUpdate,
    ) -> cosmic::Task<cosmic::Action<Message>> {
        match event {
            ColorPickerUpdate::ActiveColor(bgcolor_hsv) => {
                // use crate palette for color conversion:
                self.bgcolor = Color::from(palette::Srgb::from_color(bgcolor_hsv));
            }
            ColorPickerUpdate::Reset | ColorPickerUpdate::Input(_) => {
                // cannot restore color until reset has completed in color_picker_model.update(),
                // so attach subsequent event AplliedColor message to get another one call
                return self.color_picker_model.update(event).chain(
                    cosmic::Task::done(Message::ColorUpdate(ColorPickerUpdate::AppliedColor))
                        .map(cosmic::Action::from),
                );
            }
            ColorPickerUpdate::AppliedColor => {
                // this event is come after reset has done (we chained it in reset variant before), so simply apply current (reset) color
                if let Some(bgcolor) = self.color_picker_model.get_applied_color() {
                    self.bgcolor = bgcolor;
                }
            }
            _ => {}
        }
        self.color_picker_model.update(event)
    }

    pub fn build_dialog_view(&self) -> Element<'_, Message> {
        widget::dialog()
            .title(fl!("edit-style-title"))
            .body(fl!("edit-style-comment"))
            .control(with_background(
                self.build_edit_style_control(),
                self.bgcolor,
            ))
            .primary_action(
                widget::button::text(fl!("edit-style-ok")).on_press(Message::EditStyleUpdate),
            )
            .secondary_action(
                widget::button::text(fl!("edit-style-cancel")).on_press(Message::EditStyleCancel),
            )
            .into()
    }

    fn build_edit_style_control(&self) -> Element<'_, Message> {
        widget::column::with_capacity(3)
            .push(
                widget::row::with_capacity(1).push(
                    widget::text_input("", &self.name)
                        .label(fl!("edit-style-name"))
                        .on_input(Message::InputStyleName),
                ),
            )
            .push(
                widget::row::with_capacity(1)
                    .push(widget::text_input("", &self.font_name).label(fl!("edit-style-font"))),
            )
            .push(
                widget::row::with_capacity(2)
                    .push(widget::text(fl!("edit-style-bg")))
                    .push(self.build_color_picker())
                    .height(Length::Fill),
            )
            .into()
    }

    fn build_color_picker(&self) -> Element<'_, Message> {
        self.color_picker_model
            .builder(Message::ColorUpdate)
            .width(Length::Fill)
            .height(Length::Fill)
            .reset_label(fl!("edit-style-bg-reset"))
            .build(
                fl!("edit-style-bg-recent"),
                fl!("edit-style-bg-copy"),
                fl!("edit-style-bg-copied"),
            )
            .into()
    }
}
