use super::service::Message;
use crate::notes::FontStyle;
use cosmic::prelude::*;
use cosmic::{
    font::{self, Font},
    iced::{self, Color},
    widget,
};
use palette::Srgba;

#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub const fn to_usize(v: f32) -> usize {
    v as usize
}

#[inline]
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub const fn to_f32(v: usize) -> f32 {
    v as f32
}

#[inline]
pub const fn text_color() -> Srgba {
    Srgba::new(0.08, 0.08, 0.08, 1.0)
}

pub fn with_background(child: Element<'_, Message>, bgcolor: Color) -> Element<'_, Message> {
    widget::container(child)
        .class(cosmic::style::Container::custom(move |theme: &Theme| {
            let cosmic = theme.cosmic();
            iced::widget::container::Style {
                icon_color: Some(Color::from(text_color())),
                text_color: Some(Color::from(text_color())),
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

pub fn cosmic_font(font_style: FontStyle) -> Font {
    match font_style {
        FontStyle::Default => font::default(),
        FontStyle::Light => font::light(),
        FontStyle::Semibold => font::semibold(),
        FontStyle::Bold => font::bold(),
        FontStyle::Monospace => font::mono(),
    }
}
