use super::Theme;
use iced::widget::container::{Catalog, Style, StyleFn};
use iced::{Border, Color, Shadow, Vector};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(transparent)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn transparent(_theme: &Theme) -> Style {
    Style::default()
}

/// Fondo plano de un color, sin borde — encabezado de grilla, fila alterna,
/// fondo de la barra de título.
pub fn tinted(color: Color) -> impl Fn(&Theme) -> Style {
    move |_theme: &Theme| Style {
        background: Some(color.into()),
        ..Style::default()
    }
}

/// Panel tipo tarjeta: fondo, borde sutil, esquinas redondeadas y una sombra
/// leve para dar profundidad — en vez de un bloque de color plano flotando en
/// el vacío.
pub fn card(background: Color, border_color: Color) -> impl Fn(&Theme) -> Style {
    move |_theme: &Theme| Style {
        background: Some(background.into()),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        ..Style::default()
    }
}

/// Burbuja de tooltip: fondo y borde del tema en vez de
/// `container::rounded_box` (el estilo genérico de iced) — mismo tipo de
/// inconsistencia que ya se corrigió para los divisores de sección.
pub fn tooltip(theme: &Theme) -> Style {
    Style {
        background: Some(super::panel(theme.dark).into()),
        border: Border {
            color: super::border(theme.dark),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Style::default()
    }
}
