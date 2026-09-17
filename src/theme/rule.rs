use super::Theme;
use iced::widget::rule::{Catalog, FillMode, Style, StyleFn};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

/// Único estilo de divisor de toda la app: el mismo `border()` hecho a mano
/// que ya delimita menús y tarjetas, en vez de `palette.background.strong`
/// (el color auto-derivado por defecto de iced, que se veía deslavado).
/// Al ser el default del `Catalog`, ningún `horizontal_rule`/`vertical_rule`
/// necesita un `.style(...)` explícito.
pub fn default(theme: &Theme) -> Style {
    Style {
        color: super::border(theme.dark),
        width: 1,
        radius: 0.0.into(),
        fill_mode: FillMode::Full,
    }
}
