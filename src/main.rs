// Evita que Windows abra una consola junto a la ventana de la app cuando se
// lanza sin una terminal padre (doble clic, acceso directo del instalador).
// No tiene efecto en otras plataformas. La app no imprime nada por
// stdout/stderr, así que no hay pérdida de diagnóstico al ocultarla.
#![windows_subsystem = "windows"]

mod app;
mod connections;
mod db;
mod drivers;
mod theme;
mod updater;

fn main() -> iced::Result {
    iced::daemon(app::App::title, app::App::update, app::App::view)
        .theme(app::App::theme)
        .subscription(app::App::subscription)
        .font(iced_fonts::BOOTSTRAP_FONT_BYTES)
        .run_with(app::App::load)
}
