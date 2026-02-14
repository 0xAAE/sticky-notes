use super::{
    service::Message,
    utils::{cosmic_font, with_background},
};
use crate::{
    fl,
    notes::{Font, FontStyle, NoteStyle},
};
use cosmic::prelude::*;
use cosmic::{
    iced::{Alignment, Color, Length},
    widget::{self, color_picker::ColorPickerUpdate},
};
use palette::FromColor;
use uuid::Uuid;

const MIN_FONT_SIZE: u16 = 6;
const MAX_FONT_SIZE: u16 = 72;

pub struct EditStyleDialog {
    style_id: Uuid,
    name: String,
    font: Font,
    bgcolor: Color,
    color_picker_model: widget::ColorPickerModel,
    avail_fonts: Vec<String>,
    font_size_text: String,
}

impl EditStyleDialog {
    pub fn new(style_id: Uuid, style: &NoteStyle) -> Self {
        let font = style.get_font().clone();
        let font_size_text = font.size.to_string();
        Self {
            style_id,
            name: style.get_name().to_string(),
            font,
            bgcolor: style.get_background_color(),
            color_picker_model: widget::ColorPickerModel::new(
                fl!("edit-style-hex"),
                fl!("edit-style-rgb"),
                Some(style.get_background_color()),
                Some(style.get_background_color()),
            ),
            avail_fonts: get_avail_fonts().iter().map(ToString::to_string).collect(),
            font_size_text,
        }
    }

    pub fn update_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn update_font_style(&mut self, font_style: FontStyle) {
        self.font.style = font_style;
    }

    pub fn update_font_size(&mut self, font_size: u16) {
        self.font.size = font_size;
        self.font_size_text = font_size.to_string();
    }

    pub fn get_id(&self) -> Uuid {
        self.style_id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_font(&self) -> Font {
        self.font.clone()
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
                // so attach subsequent event AppliedColor message to get another one call
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
        widget::column::with_capacity(4)
            .spacing(cosmic::theme::spacing().space_m)
            .push(
                widget::row::with_capacity(1).push(
                    widget::text_input("", &self.name)
                        .label(fl!("edit-style-name"))
                        .on_input(Message::InputStyleName),
                ),
            )
            .push(
                widget::row::with_capacity(4)
                    .spacing(cosmic::theme::spacing().space_m)
                    .align_y(Alignment::Center)
                    .push(widget::text(fl!("edit-style-font")))
                    .push(widget::dropdown(
                        &self.avail_fonts,
                        self.try_get_current_font_index(),
                        move |selected_index| {
                            Message::FontStyleUpdate(
                                get_avail_fonts()
                                    .get(selected_index)
                                    .copied()
                                    .unwrap_or_default(),
                            )
                        },
                    ))
                    .push(widget::text(fl!("edit-style-font-size")))
                    .push(widget::spin_button::vertical(
                        &self.font_size_text,
                        //todo: how to conditionally compile depending on #[cfg(feature = "libcosmic/a11y")]
                        &self.font_size_text, // is required if 'a11y' feature is on
                        self.font.size,
                        1,
                        MIN_FONT_SIZE,
                        MAX_FONT_SIZE,
                        Message::FontSizeUpdate,
                    )),
            )
            .push(
                widget::text(fl!("edit-style-font-sample"))
                    .font(cosmic_font(self.font.style))
                    .size(self.font.size),
            )
            .push(
                widget::column::with_capacity(2)
                    .spacing(cosmic::theme::spacing().space_m)
                    .push(widget::text(fl!("edit-style-bg")).align_y(Alignment::Center))
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

    fn try_get_current_font_index(&self) -> Option<usize> {
        get_avail_fonts()
            .iter()
            .enumerate()
            .find_map(|(index, style)| (*style == self.font.style).then_some(index))
    }
}

const fn get_avail_fonts() -> &'static [FontStyle] {
    &[
        FontStyle::Default,
        FontStyle::Light,
        FontStyle::Semibold,
        FontStyle::Bold,
        FontStyle::Monospace,
    ]
}
