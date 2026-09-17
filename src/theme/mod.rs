//! Paleta "Slate" propia de RusioDB y el tipo `Theme` de la app.
//!
//! En vez de `iced::Theme::custom(...)` (que deriva colores auxiliares como
//! `background.weak/strong` en espacio lineal, mezclando hacia el texto de
//! una forma que se ve deslavada — ver el historial de `panel`/`border` más
//! abajo), `Theme` acá es un simple `{ dark: bool }` y cada widget usado
//! implementa su propio trait `Catalog` en un archivo de este módulo (mismo
//! patrón que usa https://github.com/squidowl/halloy, otra app de
//! escritorio en Iced). Así todo widget sale con los colores hechos a mano
//! por defecto, sin depender de que cada sitio de uso recuerde llamar
//! `.style(...)` — la clase de bug que ya aparecía dos veces en este
//! proyecto (paneles, después los divisores de sección).
use iced::daemon::{Appearance, DefaultStyle};
use iced::Color;

pub mod button;
pub mod checkbox;
pub mod container;
pub mod menu_bar;
pub mod overlay_menu;
pub mod pick_list;
pub mod rule;
pub mod scrollable;
pub mod svg;
pub mod text;
pub mod text_editor;
pub mod text_input;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Theme {
    pub dark: bool,
}

impl DefaultStyle for Theme {
    fn default_style(&self) -> Appearance {
        Appearance {
            background_color: background(self.dark),
            text_color: text(self.dark),
        }
    }
}

pub fn background(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x12, 0x15, 0x1B)
    } else {
        Color::from_rgb8(0xF7, 0xF8, 0xFA)
    }
}

pub fn text(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0xE4, 0xE7, 0xEC)
    } else {
        Color::from_rgb8(0x1B, 0x1F, 0x27)
    }
}

pub fn primary(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x4E, 0xA1, 0xF7)
    } else {
        Color::from_rgb8(0x34, 0x65, 0xD9)
    }
}

/// Tinte translúcido de [`primary`] para fondos "seleccionado" chicos (chip
/// de pestaña activa, indicador de ítem activo del menú desplegable).
/// Reemplaza lo que antes era `theme.extended_palette().primary.weak`,
/// auto-derivado por iced.
pub fn primary_weak(dark: bool) -> Color {
    Color {
        a: 0.18,
        ..primary(dark)
    }
}

pub fn success(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x22, 0xC3, 0xA6)
    } else {
        Color::from_rgb8(0x1B, 0x8A, 0x6B)
    }
}

pub fn danger(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0xE5, 0x48, 0x4D)
    } else {
        Color::from_rgb8(0xC5, 0x30, 0x30)
    }
}

/// Texto/ícono atenuado: punto de "desconectado" en el árbol, celdas `NULL`
/// en la grilla, atajos de teclado en el menú. Reemplaza lo que antes era
/// `theme.extended_palette().background.strong`, auto-derivado.
pub fn muted(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0x6B, 0x74, 0x80)
    } else {
        Color::from_rgb8(0x8A, 0x93, 0x9E)
    }
}

/// Fondo de panel (barra lateral, formularios, encabezado de grilla). Se define
/// a mano en vez de derivarlo de `background`: con un texto casi blanco sobre
/// un fondo casi negro (tema oscuro), la mezcla automática de iced hacia el
/// color de texto da un gris mucho más claro de lo esperado — no un panel sutil.
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

/// Borde sutil para los paneles tipo tarjeta y los divisores de sección: un
/// paso de contraste sobre [`panel`], nunca sobre el fondo de página.
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
