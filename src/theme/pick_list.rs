use super::Theme;
use iced::widget::pick_list::{Catalog, Status, Style, StyleFn};
use iced::Border;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> <Self as Catalog>::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &<Self as Catalog>::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

pub fn default(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let border_color = match status {
        Status::Hovered | Status::Opened => super::accent(dark),
        Status::Active => super::border(dark),
    };
    Style {
        text_color: super::text(dark),
        placeholder_color: super::muted(dark),
        handle_color: super::text(dark),
        background: super::panel(dark).into(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
    }
}
