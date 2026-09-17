use super::Theme;
use iced::widget::text_editor::{Catalog, Status, Style, StyleFn};
use iced::Border;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

pub fn default(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let border_color = match status {
        Status::Focused => super::accent(dark),
        _ => super::border(dark),
    };
    Style {
        background: super::panel(dark).into(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        icon: super::muted(dark),
        placeholder: super::muted(dark),
        value: super::text(dark),
        selection: super::primary_weak(dark),
    }
}
