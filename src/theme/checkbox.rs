use super::Theme;
use iced::widget::checkbox::{Catalog, Status, Style, StyleFn};
use iced::{Border, Color};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let is_checked = match status {
        Status::Active { is_checked }
        | Status::Hovered { is_checked }
        | Status::Disabled { is_checked } => is_checked,
    };
    let border_color = super::border(dark);
    let background = if is_checked {
        super::accent(dark)
    } else {
        super::panel(dark)
    };
    let icon_color = if is_checked {
        Color::WHITE
    } else {
        Color::TRANSPARENT
    };
    Style {
        background: background.into(),
        icon_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: Some(super::text(dark)),
    }
}
