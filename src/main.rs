mod app;
mod connections;
mod db;
mod drivers;
mod theme;

fn main() -> iced::Result {
    iced::daemon(app::App::title, app::App::update, app::App::view)
        .theme(app::App::theme)
        .subscription(app::App::subscription)
        .font(iced_fonts::BOOTSTRAP_FONT_BYTES)
        .run_with(app::App::load)
}
