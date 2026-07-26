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
pub const fn text_color(for_light_theme: bool) -> Srgba {
    if for_light_theme {
        Srgba::new(0.08, 0.08, 0.08, 1.0)
    } else {
        Srgba::new(0.75, 0.75, 0.75, 1.0)
    }
}

#[inline]
pub const fn background_color(for_light_theme: bool) -> Srgba {
    if for_light_theme {
        Srgba::new(0.9, 0.9, 0.9, 1.0)
    } else {
        Srgba::new(0.08, 0.08, 0.08, 1.0)
    }
}

pub fn with_background(
    child: Element<'_, Message>,
    bgcolor: Color,
    is_light: bool,
) -> Element<'_, Message> {
    widget::container(child)
        .class(cosmic::style::Container::custom(move |theme: &Theme| {
            let cosmic = theme.cosmic();
            iced::widget::container::Style {
                icon_color: Some(Color::from(text_color(is_light))),
                text_color: Some(Color::from(text_color(is_light))),
                background: Some(iced::Background::Color(bgcolor)),
                border: iced::Border {
                    radius: cosmic.corner_radii.radius_s.into(),
                    ..Default::default()
                },
                shadow: iced::Shadow::default(),
                snap: false, // Whether the container should be snapped to the pixel grid
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
