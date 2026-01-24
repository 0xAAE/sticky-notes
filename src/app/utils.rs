use crate::app::Message;
use cosmic::prelude::*;
use cosmic::{
    iced::{self, Color},
    widget,
};

#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub const fn to_usize(v: f32) -> usize {
    v as usize
}

#[inline]
#[allow(clippy::cast_precision_loss)]
pub const fn to_f32(v: usize) -> f32 {
    v as f32
}

pub fn with_background(child: Element<'_, Message>, bgcolor: Color) -> Element<'_, Message> {
    widget::container(child)
        .class(cosmic::style::Container::custom(move |theme: &Theme| {
            let cosmic = theme.cosmic();
            iced::widget::container::Style {
                icon_color: Some(Color::from(cosmic.background.on)),
                text_color: Some(Color::from(cosmic.background.on)),
                background: Some(iced::Background::Color(bgcolor)),
                border: iced::Border {
                    radius: cosmic.corner_radii.radius_s.into(),
                    ..Default::default()
                },
                shadow: iced::Shadow::default(),
            }
        }))
        .padding(cosmic::theme::spacing().space_xs)
        .into()
}
