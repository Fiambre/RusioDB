use super::Theme;
use iced::widget::button::{Catalog, Status, Style, StyleFn};
use iced::{Border, Color, Shadow};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// Botón sólido: la acción por defecto para lo que antes caía en el
/// `button::primary` de iced sin `.style()` explícito (Nueva conexión/Nueva
/// consulta/Guardar/Actualizar/Copiar resultados/botones de los banners).
pub fn primary(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let base = super::primary(dark);
    match status {
        Status::Active => Style {
            background: Some(base.into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
        },
        Status::Hovered | Status::Pressed => Style {
            background: Some(lighten(base, 0.08).into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
        },
        Status::Disabled => Style {
            background: Some(Color { a: 0.35, ..base }.into()),
            text_color: Color {
                a: 0.6,
                ..Color::WHITE
            },
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
        },
    }
}

/// Botón de borde, sin relleno — la acción secundaria de un formulario
/// ("Cancelar").
pub fn secondary(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let text_color = super::text(dark);
    let border_color = super::border(dark);
    match status {
        Status::Hovered | Status::Pressed => Style {
            background: Some(
                Color {
                    a: 0.06,
                    ..text_color
                }
                .into(),
            ),
            text_color,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: Shadow::default(),
        },
        Status::Disabled => Style {
            background: None,
            text_color: Color {
                a: 0.5,
                ..text_color
            },
            border: Border {
                color: Color {
                    a: 0.5,
                    ..border_color
                },
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: Shadow::default(),
        },
        Status::Active => Style {
            background: None,
            text_color,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: Shadow::default(),
        },
    }
}

/// Botón sin fondo ni borde en reposo, solo texto — para ítems de menú,
/// hojas del árbol, pestañas y botones de solo ícono.
pub fn text(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let text_color = super::text(dark);
    match status {
        Status::Hovered | Status::Pressed => Style {
            background: Some(
                Color {
                    a: 0.08,
                    ..text_color
                }
                .into(),
            ),
            text_color,
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
        },
        Status::Disabled => Style {
            background: None,
            text_color: Color {
                a: 0.4,
                ..text_color
            },
            border: Border::default(),
            shadow: Shadow::default(),
        },
        Status::Active => Style {
            background: None,
            text_color,
            border: Border::default(),
            shadow: Shadow::default(),
        },
    }
}

/// Botón de acento para la única acción principal de cada pantalla (ver
/// [`super::accent`]) — "Conectar", "Ejecutar", "Nueva conexión" del estado
/// vacío.
pub fn accent(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let (background, text_color) = match status {
        Status::Hovered | Status::Pressed => (super::accent_strong(dark), Color::WHITE),
        Status::Disabled => (
            Color {
                a: 0.35,
                ..super::accent(dark)
            },
            Color {
                a: 0.6,
                ..Color::WHITE
            },
        ),
        Status::Active => (super::accent(dark), Color::WHITE),
    };
    Style {
        background: Some(background.into()),
        text_color,
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
    }
}

/// Botón "fantasma": borde y fondo tenue siempre visibles (no solo al pasar
/// el mouse), para una acción secundaria puntual en un encabezado de sección
/// (ej. "Nueva" junto a "Conexiones") — con `text` no tendría fondo ni borde
/// en reposo y se confundiría con texto plano.
pub fn ghost(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let accent = super::accent(dark);
    let (background_alpha, border_color) = match status {
        Status::Hovered | Status::Pressed => (0.22, accent),
        Status::Disabled => (
            0.04,
            Color {
                a: 0.4,
                ..super::border(dark)
            },
        ),
        Status::Active => (0.1, accent),
    };
    Style {
        background: Some(
            Color {
                a: background_alpha,
                ..accent
            }
            .into(),
        ),
        text_color: accent,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
    }
}

/// Fila de conexión seleccionada en el árbol: un fondo traslúcido con radio
/// de esquina más marcado, para que la selección se lea como una "píldora"
/// insertada en el panel en vez de un bloque sólido de punta a punta.
pub fn selected_row(theme: &Theme, status: Status) -> Style {
    let dark = theme.dark;
    let accent = super::accent(dark);
    let alpha = match status {
        Status::Hovered | Status::Pressed => 0.28,
        Status::Disabled => 0.1,
        Status::Active => 0.18,
    };
    Style {
        background: Some(Color { a: alpha, ..accent }.into()),
        text_color: super::text(dark),
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
    }
}

fn lighten(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r + amount).min(1.0),
        g: (color.g + amount).min(1.0),
        b: (color.b + amount).min(1.0),
        a: color.a,
    }
}
