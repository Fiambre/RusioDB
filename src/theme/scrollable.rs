use super::Theme;
use iced::widget::scrollable::{Catalog, Rail, Scroller, Status, Style, StyleFn};
use iced::{Background, Border, Color};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(thin)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// Barra de scroll fina y sin fondo/riel visible — solo un "thumb"
/// redondeado con el color de borde del tema, en vez de la barra más gruesa
/// con track de fondo que usa iced por defecto.
pub fn thin(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let base = super::border(dark);
    let color = match status {
        Status::Hovered { .. } | Status::Dragged { .. } => super::accent(dark),
        Status::Active => base,
    };
    let rail = Rail {
        background: None,
        border: Border::default(),
        scroller: Scroller {
            color,
            border: Border {
                radius: 8.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
    };
    Style {
        container: iced::widget::container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None::<Background>,
    }
}
