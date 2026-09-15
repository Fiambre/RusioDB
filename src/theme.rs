//! Paleta "Slate" propia de RusioDB. `Theme::custom` deriva la paleta extendida
//! (hover/pressed/etc.) que ya usan los estilos de widgets existentes.
use iced::theme::Palette;
use iced::{Color, Theme};

pub fn dark() -> Theme {
    Theme::custom(
        "Slate Dark".to_string(),
        Palette {
            background: Color::from_rgb8(0x12, 0x15, 0x1B),
            text: Color::from_rgb8(0xE4, 0xE7, 0xEC),
            primary: Color::from_rgb8(0x4E, 0xA1, 0xF7),
            success: Color::from_rgb8(0x22, 0xC3, 0xA6),
            danger: Color::from_rgb8(0xE5, 0x48, 0x4D),
        },
    )
}

pub fn light() -> Theme {
    Theme::custom(
        "Slate Light".to_string(),
        Palette {
            background: Color::from_rgb8(0xF7, 0xF8, 0xFA),
            text: Color::from_rgb8(0x1B, 0x1F, 0x27),
            primary: Color::from_rgb8(0x34, 0x65, 0xD9),
            success: Color::from_rgb8(0x1B, 0x8A, 0x6B),
            danger: Color::from_rgb8(0xC5, 0x30, 0x30),
        },
    )
}

/// Fondo de panel (barra lateral, formularios, encabezado de grilla). Se define
/// a mano en vez de usar `theme.extended_palette().background.weak`: Iced mezcla
/// esa variante hacia el color de texto en espacio lineal, y con un texto casi
/// blanco sobre un fondo casi negro (tema oscuro) el resultado es un gris plano
/// mucho más claro de lo esperado — no un panel sutil.
pub fn panel(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x1A, 0x1E, 0x27)
    } else {
        Color::from_rgb8(0xED, 0xEF, 0xF3)
    }
}

/// Sombreado de fila alterna en la grilla de resultados, un paso más sutil que
/// [`panel`].
pub fn alt_row(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x17, 0x1B, 0x23)
    } else {
        Color::from_rgb8(0xF1, 0xF3, 0xF6)
    }
}

/// Borde sutil para los paneles tipo tarjeta: un paso de contraste sobre
/// [`panel`], nunca sobre el fondo de página.
pub fn border(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x2A, 0x30, 0x3C)
    } else {
        Color::from_rgb8(0xE1, 0xE4, 0xEA)
    }
}

/// Color de acento reservado para la ÚNICA acción principal de cada
/// pantalla (ej. "Conectar", "Ejecutar") — distinto del azul que ya usan el
/// resto de los botones, para que resalte por contraste en vez de perderse
/// entre botones del mismo color.
pub fn accent(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x7C, 0x5C, 0xFC)
    } else {
        Color::from_rgb8(0x6D, 0x42, 0xE0)
    }
}

/// Variante de [`accent`] para estados hover/pressed.
pub fn accent_strong(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x8F, 0x73, 0xFF)
    } else {
        Color::from_rgb8(0x5B, 0x33, 0xC4)
    }
}
