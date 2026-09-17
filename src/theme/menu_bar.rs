//! `Catalog` de `iced_aw` para la barra de menú (Archivo/Edición/...) — un
//! trait distinto del que usa el desplegable interno de `pick_list`
//! ([`super::overlay_menu`]), sin relación entre ambos.
use super::Theme;
use iced::{Border, Color, Padding, Shadow, Vector};
use iced_aw::style::menu_bar::{Catalog, Style};
use iced_aw::style::{Status, StyleFn};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self, Style>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// La barra en sí queda transparente porque ya vive dentro de la tarjeta de
/// "chrome"; el desplegable flotante sí recibe fondo, borde y sombra propios,
/// coherentes con el resto de la UI en lugar del gris genérico por defecto
/// de `iced_aw`.
pub fn default(theme: &Theme, _status: Status) -> Style {
    let dark = theme.dark;
    Style {
        bar_background: Color::TRANSPARENT.into(),
        bar_border: Border::default(),
        bar_shadow: Shadow::default(),
        bar_background_expand: Padding::ZERO,
        menu_background: super::panel(dark).into(),
        menu_border: Border {
            color: super::border(dark),
            width: 1.0,
            radius: 8.0.into(),
        },
        menu_shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.28),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        menu_background_expand: Padding::from(6),
        path: super::primary_weak(dark).into(),
        path_border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
    }
}
