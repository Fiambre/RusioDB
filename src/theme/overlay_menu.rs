//! `Catalog` del menú desplegable interno que usa `pick_list` (distinto del
//! menú de la barra Archivo/Edición/..., que es de `iced_aw` — ver
//! [`super::menu_bar`]).
use super::Theme;
use iced::overlay::menu::{Catalog, Style, StyleFn};
use iced::Border;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> <Self as Catalog>::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &<Self as Catalog>::Class<'_>) -> Style {
        class(self)
    }
}

pub fn default(theme: &Theme) -> Style {
    let dark = theme.dark;
    Style {
        background: super::panel(dark).into(),
        border: Border {
            color: super::border(dark),
            width: 1.0,
            radius: 6.0.into(),
        },
        text_color: super::text(dark),
        selected_text_color: super::text(dark),
        selected_background: super::primary_weak(dark).into(),
    }
}
