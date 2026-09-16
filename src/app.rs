use crate::{
    connections::{self, ConnectionProfile, ConnectionsFile},
    db::QueryResult,
    drivers::{CatalogObject, Config, Database, Driver, ObjectKind},
    theme,
    updater::{self, UpdateInfo},
};
use iced::widget::{
    button, checkbox, column, container, horizontal_rule, mouse_area, pick_list, row, scrollable,
    svg, text, text_editor, text_input, tooltip, vertical_rule, Column, Space,
};
use iced::{
    alignment::Horizontal, keyboard, window, Alignment, Border, Color, Element, Length, Padding,
    Shadow, Size, Subscription, Task, Theme, Vector,
};
use iced_aw::menu::{Item, Menu, MenuBar, Style as MenuStyle};
use iced_fonts::{Bootstrap, BOOTSTRAP_FONT};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

type SharedDatabase = Arc<Mutex<Database>>;
type ConnectionId = u64;
type Connected = Result<(SharedDatabase, Vec<CatalogObject>), String>;
type Executed = Result<(QueryResult, Result<Vec<CatalogObject>, String>, u128), String>;

const KIND_ORDER: [ObjectKind; 6] = [
    ObjectKind::Table,
    ObjectKind::Collection,
    ObjectKind::View,
    ObjectKind::MaterializedView,
    ObjectKind::Function,
    ObjectKind::Procedure,
];

fn icon<'a>(glyph: Bootstrap) -> iced::widget::Text<'a> {
    text(glyph.to_string()).font(BOOTSTRAP_FONT)
}

const CAT_LOGO: &[u8] = include_bytes!("../assets/cat.svg");

/// Marca de la app: una silueta de gato minimalista, coloreada con el
/// acento del tema (el filtro de color del `Svg` recolorea la silueta
/// entera, así que el color de relleno del propio archivo no importa).
fn logo(dark: bool) -> Element<'static, Message> {
    svg(svg::Handle::from_memory(CAT_LOGO))
        .width(20)
        .height(20)
        .style(move |_theme: &Theme, _status| svg::Style {
            color: Some(theme::accent(dark)),
        })
        .into()
}

fn icon_button<'a>(glyph: Bootstrap, label: &'a str) -> iced::widget::Row<'a, Message> {
    row![icon(glyph).size(14), text(label)]
        .spacing(6)
        .align_y(Alignment::Center)
}

/// Botón chico de solo ícono con tooltip, para acciones de fila (editar/
/// desconectar/eliminar) donde el espacio es angosto.
fn icon_only_button(glyph: Bootstrap, hint: &str, message: Message) -> Element<'_, Message> {
    tooltip(
        button(icon(glyph).size(13))
            .style(button::text)
            .on_press(message),
        container(text(hint).size(12))
            .padding(6)
            .style(container::rounded_box),
        tooltip::Position::Top,
    )
    .into()
}

/// Columna label-arriba/valor-abajo, estilo la franja de estadísticas de un
/// ticker de trading (Binance): etiqueta chica y muted, valor grande y firme.
fn stat<'a>(label: &'a str, value: String, value_color: Option<Color>) -> Column<'a, Message> {
    let mut value_text = text(value).size(16);
    if let Some(color) = value_color {
        value_text = value_text.color(color);
    }
    column![text(label).size(11), value_text].spacing(2)
}

struct Tab {
    title: String,
    editor: text_editor::Content,
    result: QueryResult,
    connection_id: Option<ConnectionId>,
    running: bool,
    last_elapsed_ms: Option<u128>,
}
impl Tab {
    fn new(title: String, sql: &str, connection_id: Option<ConnectionId>) -> Self {
        Self {
            title,
            editor: text_editor::Content::with_text(sql),
            result: QueryResult::default(),
            connection_id,
            running: false,
            last_elapsed_ms: None,
        }
    }
}

enum ConnState {
    Disconnected,
    Connecting,
    Connected {
        database: SharedDatabase,
        catalog: Vec<CatalogObject>,
    },
    Error(String),
}

struct ConnectionEntry {
    id: ConnectionId,
    profile: ConnectionProfile,
    state: ConnState,
    catalog_busy: bool,
}

pub struct App {
    connections: Vec<ConnectionEntry>,
    next_id: u64,
    connections_path: Option<PathBuf>,
    keyring_enabled: bool,
    main_window: Option<window::Id>,
    connection_window: Option<window::Id>,
    main_maximized: bool,
    title_last_click: Option<window::Id>,
    title_last_click_at: Option<Instant>,
    form_error: Option<String>,
    selected_connection: Option<ConnectionId>,
    expanded: HashSet<String>,
    pending_delete: Option<ConnectionId>,
    last_click: Option<ConnectionId>,
    last_click_at: Option<Instant>,
    form: Config,
    editing_profile: Option<ConnectionId>,
    tabs: Vec<Tab>,
    selected: usize,
    status: String,
    show_explorer: bool,
    dark: bool,
    about: bool,
    update_available: Option<UpdateInfo>,
    updating: bool,
}
impl Default for App {
    fn default() -> Self {
        Self {
            connections: vec![],
            next_id: 1,
            connections_path: None,
            keyring_enabled: false,
            main_window: None,
            connection_window: None,
            main_maximized: false,
            title_last_click: None,
            title_last_click_at: None,
            form_error: None,
            selected_connection: None,
            expanded: HashSet::new(),
            pending_delete: None,
            last_click: None,
            last_click_at: None,
            form: Config::default(),
            editing_profile: None,
            tabs: vec![Tab::new("Consulta 1".into(), Driver::Sqlite.sample(), None)],
            selected: 0,
            status: "Crea o selecciona una conexión para comenzar.".into(),
            show_explorer: true,
            dark: true,
            about: false,
            update_available: None,
            updating: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Field {
    Name,
    Path,
    Host,
    Port,
    Database,
    User,
    Password,
}
#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    Driver(Driver),
    Field(Field, String),
    Tls(bool),
    TlsVerify(bool),
    RememberPassword(bool),
    NewConnection,
    EditConnection(ConnectionId),
    CloseConnectionForm,
    SaveProfile,
    ConnectProfile,
    Connected(ConnectionId, Connected),
    SelectConnection(ConnectionId),
    ToggleExpanded(String),
    Disconnect,
    DeleteConnection,
    Refresh,
    CatalogRefreshed(ConnectionId, Result<Vec<CatalogObject>, String>),
    NewQuery,
    SelectTab(usize),
    CloseTab(usize),
    Edit(text_editor::Action),
    Run,
    Preview(ConnectionId, CatalogObject),
    Completed(usize, ConnectionId, Executed),
    CopyResults,
    CopySql,
    Paste,
    Pasted(Option<String>),
    SelectAll,
    ToggleExplorer,
    ToggleTheme,
    About,
    CheckForUpdates,
    UpdateChecked(Result<Option<UpdateInfo>, String>),
    InstallUpdate,
    UpdateDownloaded(Result<PathBuf, String>),
    UpdateApplied(Result<(), String>),
    DismissUpdateBanner,
    Exit,
    WindowClosed(window::Id),
    TitleBarPressed(window::Id),
    Minimize(window::Id),
    ToggleMaximize(window::Id),
}

impl App {
    pub fn load() -> (Self, Task<Message>) {
        let path = connections::default_path();
        let file = path.as_deref().map(connections::load).unwrap_or_default();
        let connections: Vec<ConnectionEntry> = file
            .profiles
            .into_iter()
            .map(|profile| ConnectionEntry {
                id: profile.id,
                profile,
                state: ConnState::Disconnected,
                catalog_busy: false,
            })
            .collect();
        let next_id = file.next_id.max(1);
        let open_form_on_start = connections.is_empty();
        let mut app = Self {
            connections,
            next_id,
            connections_path: path,
            keyring_enabled: true,
            ..Self::default()
        };
        let (main_id, open_main) = window::open(window::Settings {
            size: Size::new(1280.0, 860.0),
            decorations: false,
            ..window::Settings::default()
        });
        app.main_window = Some(main_id);
        let mut task = open_main.map(|_| Message::Noop);
        if open_form_on_start {
            task = Task::batch([task, app.open_connection_window()]);
        }
        task = Task::batch([task, Self::spawn_check_updates()]);
        (app, task)
    }
    pub fn theme(&self, _window: window::Id) -> Theme {
        self.active_theme()
    }
    fn active_theme(&self) -> Theme {
        if self.dark {
            theme::dark()
        } else {
            theme::light()
        }
    }
    pub fn title(&self, window: window::Id) -> String {
        if Some(window) == self.connection_window {
            if self.editing_profile.is_some() {
                "Editar conexión".into()
            } else {
                "Nueva conexión".into()
            }
        } else {
            "RusioDB".into()
        }
    }
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            keyboard::on_key_press(Self::shortcut),
            window::close_events().map(Message::WindowClosed),
        ])
    }
    fn shortcut(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Message> {
        use keyboard::{key::Named, Key};
        match key.as_ref() {
            Key::Named(Named::F5) => Some(Message::Run),
            Key::Named(Named::F6) => Some(Message::Refresh),
            Key::Named(Named::Enter) if modifiers.command() => Some(Message::Run),
            Key::Character("n") if modifiers.command() => Some(if modifiers.shift() {
                Message::NewConnection
            } else {
                Message::NewQuery
            }),
            _ => None,
        }
    }
    fn editor_binding(event: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
        if event.status == text_editor::Status::Focused {
            if let Some(message) = Self::shortcut(event.key.clone(), event.modifiers) {
                return Some(text_editor::Binding::Custom(message));
            }
        }
        text_editor::Binding::from_key_press(event)
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        if !matches!(message, Message::DeleteConnection) {
            self.pending_delete = None;
        }
        match message {
            Message::Driver(driver) => {
                self.form.driver = driver;
                self.form.port = driver.port().into();
            }
            Message::Field(field, value) => {
                *match field {
                    Field::Name => &mut self.form.name,
                    Field::Path => &mut self.form.path,
                    Field::Host => &mut self.form.host,
                    Field::Port => &mut self.form.port,
                    Field::Database => &mut self.form.database,
                    Field::User => &mut self.form.user,
                    Field::Password => &mut self.form.password,
                } = value;
            }
            Message::Tls(value) => self.form.tls = value,
            Message::TlsVerify(value) => self.form.tls_verify = value,
            Message::RememberPassword(value) => self.form.remember_password = value,
            Message::NewConnection => {
                self.form = Config::default();
                self.editing_profile = None;
                self.form_error = None;
                return self.open_connection_window();
            }
            Message::EditConnection(id) => {
                let config = self
                    .connections
                    .iter()
                    .find(|e| e.id == id)
                    .map(|entry| connections::config_from_profile(&entry.profile));
                if let Some(mut config) = config {
                    if self.keyring_enabled {
                        if let Some(password) = connections::load_password(id) {
                            config.password = password;
                        }
                    }
                    self.form = config;
                    self.editing_profile = Some(id);
                    self.form_error = None;
                    return self.open_connection_window();
                }
            }
            Message::CloseConnectionForm => {
                if let Some(id) = self.connection_window.take() {
                    self.editing_profile = None;
                    self.form_error = None;
                    return window::close(id);
                }
            }
            Message::SaveProfile => {
                let id = self.editing_profile.unwrap_or_else(|| self.allocate_id());
                let profile = connections::profile_from_config(id, &self.form);
                self.upsert_profile(profile);
                self.editing_profile = Some(id);
                self.selected_connection = Some(id);
                self.sync_saved_password(id, &self.form.password.clone());
                self.persist_connections();
                self.status = "Conexión guardada.".into();
                if let Some(window_id) = self.connection_window.take() {
                    self.form_error = None;
                    return window::close(window_id);
                }
            }
            Message::ConnectProfile => {
                let id = self.editing_profile.unwrap_or_else(|| self.allocate_id());
                let profile = connections::profile_from_config(id, &self.form);
                let name = profile.name.clone();
                self.upsert_profile(profile);
                self.editing_profile = Some(id);
                self.selected_connection = Some(id);
                self.set_state(id, ConnState::Connecting);
                self.sync_saved_password(id, &self.form.password.clone());
                self.persist_connections();
                let config = self.form.clone();
                self.form.password.clear();
                self.status = format!("Conectando {name}…");
                return Self::spawn_connect(id, config);
            }
            Message::Connected(id, result) => {
                let watching = self.editing_profile == Some(id) && self.connection_window.is_some();
                match result {
                    Ok((database, catalog)) => {
                        if !self.connections.iter().any(|e| e.id == id) {
                            return Self::release(Some(database));
                        }
                        let has_tab = self.tabs.iter().any(|t| t.connection_id == Some(id));
                        let mut name = String::new();
                        let mut driver = Driver::Sqlite;
                        if let Some(entry) = self.connections.iter_mut().find(|e| e.id == id) {
                            entry.state = ConnState::Connected { database, catalog };
                            name = entry.profile.name.clone();
                            driver = entry.profile.driver;
                        }
                        self.expanded.insert(id.to_string());
                        self.status = format!("Conectado a {name}");
                        let mut close_task = Task::none();
                        if watching {
                            if let Some(window_id) = self.connection_window.take() {
                                self.form_error = None;
                                close_task = window::close(window_id);
                            }
                        }
                        if !has_tab {
                            let sql = driver.sample().to_string();
                            let index = self.tabs.len();
                            self.tabs.push(Tab::new(
                                format!("Consulta {}", index + 1),
                                &sql,
                                Some(id),
                            ));
                            if watching {
                                self.selected = index;
                            }
                        }
                        return close_task;
                    }
                    Err(error) => {
                        if let Some(entry) = self.connections.iter_mut().find(|e| e.id == id) {
                            entry.state = ConnState::Error(error.clone());
                        }
                        self.status = format!("Error de conexión: {error}");
                        if watching {
                            self.form_error = Some(error);
                        }
                    }
                }
            }
            Message::SelectConnection(id) => return self.select_connection(id),
            Message::ToggleExpanded(key) => {
                if !self.expanded.remove(&key) {
                    self.expanded.insert(key);
                }
            }
            Message::Disconnect => {
                if let Some(id) = self.selected_connection {
                    if let Some(entry) = self.connections.iter_mut().find(|e| e.id == id) {
                        if matches!(entry.state, ConnState::Connected { .. }) {
                            let previous =
                                std::mem::replace(&mut entry.state, ConnState::Disconnected);
                            let name = entry.profile.name.clone();
                            self.tabs
                                .iter_mut()
                                .filter(|tab| tab.connection_id == Some(id))
                                .for_each(|tab| tab.result = QueryResult::default());
                            self.status = format!("{name} desconectada.");
                            if let ConnState::Connected { database, .. } = previous {
                                return Self::release(Some(database));
                            }
                        }
                    }
                }
            }
            Message::DeleteConnection => {
                if let Some(id) = self.selected_connection {
                    if self.pending_delete == Some(id) {
                        self.pending_delete = None;
                        let index = self.connections.iter().position(|e| e.id == id);
                        if let Some(index) = index {
                            let removed = self.connections.remove(index);
                            self.tabs.iter_mut().for_each(|tab| {
                                if tab.connection_id == Some(id) {
                                    tab.connection_id = None;
                                }
                            });
                            self.selected_connection = None;
                            self.persist_connections();
                            if self.keyring_enabled {
                                connections::delete_password(id);
                            }
                            self.status = format!("{} eliminada.", removed.profile.name);
                            if let ConnState::Connected { database, .. } = removed.state {
                                return Self::release(Some(database));
                            }
                        }
                    } else {
                        self.pending_delete = Some(id);
                        self.status = "Pulsa Eliminar otra vez para confirmar.".into();
                    }
                }
            }
            Message::Refresh => {
                if let Some(id) = self.selected_connection {
                    let ready = self
                        .connections
                        .iter()
                        .find(|e| e.id == id)
                        .and_then(|entry| {
                            if entry.catalog_busy {
                                None
                            } else if let ConnState::Connected { database, .. } = &entry.state {
                                Some(database.clone())
                            } else {
                                None
                            }
                        });
                    if let Some(database) = ready {
                        if let Some(entry) = self.connections.iter_mut().find(|e| e.id == id) {
                            entry.catalog_busy = true;
                        }
                        self.status = "Actualizando catálogo…".into();
                        return Task::perform(
                            async move {
                                let result = tokio::task::spawn_blocking(move || {
                                    database.lock().map_err(|e| e.to_string())?.catalog()
                                })
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()));
                                (id, result)
                            },
                            |(id, result)| Message::CatalogRefreshed(id, result),
                        );
                    }
                }
            }
            Message::CatalogRefreshed(id, result) => {
                if let Some(entry) = self.connections.iter_mut().find(|e| e.id == id) {
                    entry.catalog_busy = false;
                    match result {
                        Ok(catalog) => {
                            if let ConnState::Connected {
                                catalog: current, ..
                            } = &mut entry.state
                            {
                                *current = catalog;
                            }
                            self.status = "Catálogo actualizado.".into();
                        }
                        Err(error) => self.status = format!("Error: {error}"),
                    }
                }
            }
            Message::NewQuery => {
                let connection_id = self.selected_connection;
                self.add_tab("Consulta", "", connection_id);
            }
            Message::SelectTab(index) if index < self.tabs.len() => self.selected = index,
            Message::CloseTab(index) if self.tabs.len() > 1 && index < self.tabs.len() => {
                self.tabs.remove(index);
                if self.selected > index || self.selected >= self.tabs.len() {
                    self.selected = self.selected.saturating_sub(1).min(self.tabs.len() - 1);
                }
            }
            Message::Edit(action) => self.tabs[self.selected].editor.perform(action),
            Message::Preview(id, object) => {
                let driver = self.connections.iter().find(|e| e.id == id).and_then(|e| {
                    matches!(e.state, ConnState::Connected { .. }).then_some(e.profile.driver)
                });
                if let Some(driver) = driver {
                    let sql = object.preview(driver);
                    self.add_tab(&object.label(), &sql, Some(id));
                    return self.run_query();
                }
            }
            Message::Run => return self.run_query(),
            Message::Completed(index, connection_id, result) => {
                if let Some(tab) = self.tabs.get_mut(index) {
                    tab.running = false;
                    match result {
                        Ok((result, catalog, elapsed)) => {
                            self.status = if result.columns.is_empty() {
                                format!("{} filas afectadas · {elapsed} ms", result.affected)
                            } else {
                                format!(
                                    "{} filas{} · {elapsed} ms",
                                    result.rows.len(),
                                    if result.truncated {
                                        " (vista limitada a 500)"
                                    } else {
                                        ""
                                    }
                                )
                            };
                            tab.result = result;
                            tab.last_elapsed_ms = Some(elapsed);
                            match catalog {
                                Ok(catalog) => {
                                    if let Some(entry) =
                                        self.connections.iter_mut().find(|e| e.id == connection_id)
                                    {
                                        if let ConnState::Connected {
                                            catalog: current, ..
                                        } = &mut entry.state
                                        {
                                            *current = catalog;
                                        }
                                    }
                                }
                                Err(error) => self.status.push_str(&format!(
                                    " · No se pudo actualizar el catálogo: {error}"
                                )),
                            }
                        }
                        Err(error) => {
                            tab.result = QueryResult::default();
                            self.status = format!("Error SQL: {error}");
                        }
                    }
                }
            }
            Message::CopyResults => {
                let result = &self.tabs[self.selected].result;
                let mut lines = vec![result.columns.join("\t")];
                lines.extend(result.rows.iter().map(|row| row.join("\t")));
                return iced::clipboard::write(lines.join("\n"));
            }
            Message::CopySql => {
                let editor = &self.tabs[self.selected].editor;
                return iced::clipboard::write(editor.selection().unwrap_or_else(|| editor.text()));
            }
            Message::Paste => return iced::clipboard::read().map(Message::Pasted),
            Message::Pasted(Some(value)) => {
                self.tabs[self.selected]
                    .editor
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        Arc::new(value),
                    )))
            }
            Message::SelectAll => self.tabs[self.selected]
                .editor
                .perform(text_editor::Action::SelectAll),
            Message::ToggleExplorer => self.show_explorer = !self.show_explorer,
            Message::ToggleTheme => self.dark = !self.dark,
            Message::About => self.about = !self.about,
            Message::CheckForUpdates => {
                self.status = "Buscando actualizaciones…".into();
                return Self::spawn_check_updates();
            }
            Message::UpdateChecked(Ok(Some(info))) => {
                self.status = format!("Versión {} disponible.", info.version);
                self.update_available = Some(info);
            }
            Message::UpdateChecked(Ok(None)) => {
                self.status = "Ya tenés la última versión.".into();
            }
            Message::UpdateChecked(Err(error)) => {
                self.status = format!("No se pudo comprobar actualizaciones: {error}");
            }
            Message::DismissUpdateBanner => self.update_available = None,
            Message::InstallUpdate => {
                if let Some(info) = self.update_available.clone() {
                    self.updating = true;
                    self.status = "Descargando actualización…".into();
                    return Self::spawn_download_update(info.download_url);
                }
            }
            Message::UpdateDownloaded(Ok(path)) => {
                self.status = "Instalando actualización…".into();
                return Self::spawn_apply_update(path);
            }
            Message::UpdateDownloaded(Err(error)) => {
                self.updating = false;
                self.status = format!("Error al descargar la actualización: {error}");
            }
            Message::UpdateApplied(Ok(())) => {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                return iced::exit();
            }
            Message::UpdateApplied(Err(error)) => {
                self.updating = false;
                self.status = format!("Error al instalar la actualización: {error}");
            }
            Message::Exit => return iced::exit(),
            Message::WindowClosed(id) => {
                if self.is_main_window(id) {
                    return iced::exit();
                }
                if Some(id) == self.connection_window {
                    self.connection_window = None;
                    self.editing_profile = None;
                    self.form_error = None;
                }
            }
            Message::TitleBarPressed(id) => {
                let now = Instant::now();
                let is_double = self.title_last_click == Some(id)
                    && self
                        .title_last_click_at
                        .map(|previous| now.duration_since(previous) < Duration::from_millis(400))
                        .unwrap_or(false);
                self.title_last_click = Some(id);
                self.title_last_click_at = Some(now);
                if is_double {
                    if self.is_main_window(id) {
                        self.main_maximized = !self.main_maximized;
                        return window::toggle_maximize(id);
                    }
                    return Task::none();
                }
                return window::drag(id);
            }
            Message::Minimize(id) => return window::minimize(id, true),
            Message::ToggleMaximize(id) => {
                self.main_maximized = !self.main_maximized;
                return window::toggle_maximize(id);
            }
            _ => {}
        }
        Task::none()
    }

    fn is_main_window(&self, id: window::Id) -> bool {
        self.main_window == Some(id)
    }
    fn open_connection_window(&mut self) -> Task<Message> {
        if let Some(id) = self.connection_window {
            return window::gain_focus(id);
        }
        let (id, open) = window::open(window::Settings {
            size: Size::new(440.0, 420.0),
            position: window::Position::Centered,
            resizable: false,
            decorations: false,
            ..window::Settings::default()
        });
        self.connection_window = Some(id);
        open.map(|_| Message::Noop)
    }
    /// Guarda o borra la contraseña de una conexión en el almacén de
    /// credenciales del sistema según el checkbox "Guardar contraseña" del
    /// formulario. No hace nada si `keyring_enabled` es falso (tests, o
    /// cualquier estado que no venga de `App::load`).
    fn sync_saved_password(&self, id: ConnectionId, password: &str) {
        if !self.keyring_enabled {
            return;
        }
        if self.form.remember_password && !password.is_empty() {
            let _ = connections::save_password(id, password);
        } else {
            connections::delete_password(id);
        }
    }
    fn allocate_id(&mut self) -> ConnectionId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    fn upsert_profile(&mut self, profile: ConnectionProfile) {
        if let Some(entry) = self.connections.iter_mut().find(|e| e.id == profile.id) {
            entry.profile = profile;
        } else {
            self.connections.push(ConnectionEntry {
                id: profile.id,
                profile,
                state: ConnState::Disconnected,
                catalog_busy: false,
            });
        }
    }
    fn set_state(&mut self, id: ConnectionId, state: ConnState) {
        if let Some(entry) = self.connections.iter_mut().find(|e| e.id == id) {
            entry.state = state;
        }
    }
    fn persist_connections(&self) {
        let Some(path) = &self.connections_path else {
            return;
        };
        let file = ConnectionsFile {
            next_id: self.next_id,
            profiles: self.connections.iter().map(|e| e.profile.clone()).collect(),
        };
        let _ = connections::save(path, &file);
    }
    fn spawn_connect(id: ConnectionId, config: Config) -> Task<Message> {
        Task::perform(
            async move {
                let result = tokio::task::spawn_blocking(move || {
                    let mut database = Database::connect(&config)?;
                    let catalog = database.catalog()?;
                    Ok((Arc::new(Mutex::new(database)), catalog))
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
                (id, result)
            },
            |(id, result)| Message::Connected(id, result),
        )
    }
    fn spawn_check_updates() -> Task<Message> {
        Task::perform(updater::check_for_update(), Message::UpdateChecked)
    }
    fn spawn_download_update(url: String) -> Task<Message> {
        Task::perform(updater::download_update(url), Message::UpdateDownloaded)
    }
    fn spawn_apply_update(new_exe: PathBuf) -> Task<Message> {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || updater::apply_update(&new_exe))
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
            },
            Message::UpdateApplied,
        )
    }
    fn select_connection(&mut self, id: ConnectionId) -> Task<Message> {
        let now = Instant::now();
        let is_double = self.last_click == Some(id)
            && self
                .last_click_at
                .map(|previous| now.duration_since(previous) < Duration::from_millis(400))
                .unwrap_or(false);
        self.last_click = Some(id);
        self.last_click_at = Some(now);
        self.selected_connection = Some(id);
        if !is_double {
            let key = id.to_string();
            if !self.expanded.remove(&key) {
                self.expanded.insert(key);
            }
            return Task::none();
        }
        let Some(entry) = self.connections.iter().find(|e| e.id == id) else {
            return Task::none();
        };
        if matches!(
            entry.state,
            ConnState::Connected { .. } | ConnState::Connecting
        ) {
            return Task::none();
        }
        let mut config = connections::config_from_profile(&entry.profile);
        let name = entry.profile.name.clone();
        let driver = entry.profile.driver;
        let saved_password = self
            .keyring_enabled
            .then(|| connections::load_password(id))
            .flatten();
        if driver == Driver::Sqlite || saved_password.is_some() {
            if let Some(password) = saved_password {
                config.password = password;
            }
            self.set_state(id, ConnState::Connecting);
            self.status = format!("Conectando {name}…");
            return Self::spawn_connect(id, config);
        }
        self.form = config;
        self.editing_profile = Some(id);
        self.form_error = None;
        self.open_connection_window()
    }
    fn release(database: Option<SharedDatabase>) -> Task<Message> {
        Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || drop(database)).await;
            },
            |_| Message::Noop,
        )
    }
    fn add_tab(&mut self, title: &str, sql: &str, connection_id: Option<ConnectionId>) {
        self.tabs.push(Tab::new(
            format!("{title} {}", self.tabs.len() + 1),
            sql,
            connection_id,
        ));
        self.selected = self.tabs.len() - 1;
    }
    fn can_run(&self) -> bool {
        let tab = &self.tabs[self.selected];
        !tab.running
            && tab
                .connection_id
                .map(|id| {
                    self.connections
                        .iter()
                        .any(|e| e.id == id && matches!(e.state, ConnState::Connected { .. }))
                })
                .unwrap_or(false)
    }
    fn refresh_enabled(&self) -> bool {
        self.selected_connection
            .and_then(|id| self.connections.iter().find(|e| e.id == id))
            .map(|e| matches!(e.state, ConnState::Connected { .. }) && !e.catalog_busy)
            .unwrap_or(false)
    }
    fn run_query(&mut self) -> Task<Message> {
        let index = self.selected;
        let Some(connection_id) = self.tabs[index].connection_id else {
            self.status = "Esta pestaña no está asociada a ninguna conexión.".into();
            return Task::none();
        };
        let database = self.connections.iter().find_map(|entry| {
            if entry.id == connection_id {
                if let ConnState::Connected { database, .. } = &entry.state {
                    return Some(database.clone());
                }
            }
            None
        });
        let Some(database) = database else {
            self.status = "Esta conexión no está activa.".into();
            return Task::none();
        };
        if self.tabs[index].running {
            return Task::none();
        }
        self.tabs[index].running = true;
        self.status = "Ejecutando…".into();
        let sql = self.tabs[index].editor.text();
        Task::perform(
            async move {
                let outcome = tokio::task::spawn_blocking(move || {
                    let start = Instant::now();
                    let mut connection = database.lock().map_err(|e| e.to_string())?;
                    let result = connection.execute(&sql)?;
                    let catalog = connection.catalog();
                    Ok((result, catalog, start.elapsed().as_millis()))
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
                (index, connection_id, outcome)
            },
            |(index, connection_id, outcome)| Message::Completed(index, connection_id, outcome),
        )
    }

    fn menus(&self) -> Element<'_, Message> {
        let selected_entry = self
            .selected_connection
            .and_then(|id| self.connections.iter().find(|e| e.id == id));
        let connected = matches!(
            selected_entry.map(|e| &e.state),
            Some(ConnState::Connected { .. })
        );
        let catalog_busy = selected_entry.map(|e| e.catalog_busy).unwrap_or(false);
        let has_selection = selected_entry.is_some();
        let current_theme = self.active_theme();
        let muted = current_theme.extended_palette().background.strong.color;
        let entry = move |label: &'static str, shortcut: &'static str, message, enabled: bool| {
            let mut row_content = row![text(label).width(Length::Fill)].align_y(Alignment::Center);
            if !shortcut.is_empty() {
                row_content = row_content.push(text(shortcut).size(12).color(muted));
            }
            Item::new(
                button(row_content)
                    .width(260)
                    .style(button::text)
                    .on_press_maybe(enabled.then_some(message)),
            )
        };
        let divider = || Item::new(horizontal_rule(1));
        let group = |label, items| {
            Item::with_menu(
                button(text(label))
                    .padding([6, 10])
                    .style(button::text)
                    .on_press(Message::Noop),
                Menu::new(items).max_width(280.0).spacing(2.0),
            )
        };
        MenuBar::new(vec![
            group(
                "Archivo",
                vec![
                    entry(
                        "Nueva conexión",
                        "Ctrl+Shift+N",
                        Message::NewConnection,
                        true,
                    ),
                    entry("Nueva consulta", "Ctrl+N", Message::NewQuery, true),
                    divider(),
                    entry("Salir", "", Message::Exit, true),
                ],
            ),
            group(
                "Edición",
                vec![
                    entry("Copiar SQL", "", Message::CopySql, true),
                    entry("Pegar en editor", "", Message::Paste, true),
                    entry("Seleccionar todo el SQL", "", Message::SelectAll, true),
                    divider(),
                    entry(
                        "Copiar resultados",
                        "",
                        Message::CopyResults,
                        !self.tabs[self.selected].result.columns.is_empty(),
                    ),
                ],
            ),
            group(
                "Conexión",
                vec![
                    entry("Nueva conexión", "", Message::NewConnection, true),
                    entry(
                        "Editar conexión seleccionada",
                        "",
                        self.selected_connection
                            .map(Message::EditConnection)
                            .unwrap_or(Message::Noop),
                        has_selection,
                    ),
                    entry("Desconectar", "", Message::Disconnect, connected),
                    divider(),
                    entry(
                        "Eliminar conexión",
                        "",
                        Message::DeleteConnection,
                        has_selection,
                    ),
                    divider(),
                    entry(
                        "Actualizar catálogo",
                        "F6",
                        Message::Refresh,
                        connected && !catalog_busy,
                    ),
                ],
            ),
            group(
                "Consulta",
                vec![
                    entry("Nueva consulta", "Ctrl+N", Message::NewQuery, true),
                    entry("Ejecutar SQL", "F5", Message::Run, self.can_run()),
                ],
            ),
            group(
                "Ver",
                vec![
                    entry(
                        "Mostrar / ocultar explorador",
                        "",
                        Message::ToggleExplorer,
                        true,
                    ),
                    entry(
                        "Alternar tema claro / oscuro",
                        "",
                        Message::ToggleTheme,
                        true,
                    ),
                ],
            ),
            group(
                "Ayuda",
                vec![
                    entry("Acerca de RusioDB", "", Message::About, true),
                    entry("Buscar actualizaciones", "", Message::CheckForUpdates, true),
                ],
            ),
        ])
        .style(menu_style(self.dark))
        .into()
    }

    /// Barra de título propia: la ventana no tiene decoraciones nativas del
    /// SO, así que el arrastre, minimizar/maximizar y cerrar se manejan a
    /// mano. Un click simple inicia el arrastre nativo de la ventana
    /// (`window::drag`); un doble click (detectado a mano, iced no lo expone)
    /// alterna maximizado, igual que una barra de título real.
    fn title_bar<'a>(
        &self,
        id: window::Id,
        menus: Option<Element<'a, Message>>,
        label: Option<&str>,
        close: Message,
        show_minmax: bool,
    ) -> Element<'a, Message> {
        let mut leading = row![]
            .spacing(16)
            .align_y(Alignment::Center)
            .height(Length::Fill);
        if let Some(menus) = menus {
            leading = leading.push(logo(self.dark)).push(menus);
        }
        if let Some(label) = label {
            leading = leading.push(text(label.to_string()).size(13));
        }
        let drag_region = mouse_area(Space::new(Length::Fill, Length::Fill))
            .on_press(Message::TitleBarPressed(id));
        leading = leading.push(drag_region);

        let titlebar_button = |glyph: Bootstrap, message: Message, danger: bool| {
            button(icon(glyph).size(12))
                .padding([6, 10])
                .style(move |theme: &Theme, status| {
                    let palette = theme.extended_palette();
                    let mut style = button::text(theme, status);
                    if danger && matches!(status, button::Status::Hovered | button::Status::Pressed)
                    {
                        style.background = Some(palette.danger.base.color.into());
                        style.text_color = palette.danger.base.text;
                    }
                    style
                })
                .on_press(message)
        };

        let mut controls = row![].spacing(2);
        if show_minmax {
            controls = controls.push(titlebar_button(
                Bootstrap::DashLg,
                Message::Minimize(id),
                false,
            ));
            let maximize_icon = if self.main_maximized {
                Bootstrap::FullscreenExit
            } else {
                Bootstrap::ArrowsFullscreen
            };
            controls = controls.push(titlebar_button(
                maximize_icon,
                Message::ToggleMaximize(id),
                false,
            ));
        }
        controls = controls.push(titlebar_button(Bootstrap::XLg, close, true));

        container(
            row![Space::with_width(12), leading, controls]
                .align_y(Alignment::Center)
                .height(36),
        )
        .style(tinted_box(theme::panel(self.dark)))
        .into()
    }

    fn connection_form(&self) -> Element<'_, Message> {
        let input = |label, value, field: Field, secret| {
            text_input(label, value)
                .secure(secret)
                .on_input(move |value| Message::Field(field, value))
        };
        let selector = pick_list(Driver::ALL, Some(self.form.driver), Message::Driver);
        let title = if self.editing_profile.is_some() {
            "Editar conexión"
        } else {
            "Nueva conexión"
        };
        let mut fields = column![
            row![text(title).size(20), selector].spacing(12),
            input("Nombre de la conexión", &self.form.name, Field::Name, false),
        ]
        .spacing(10);
        if self.form.driver == Driver::Sqlite {
            fields = fields.push(input(
                "Ruta SQLite o :memory:",
                &self.form.path,
                Field::Path,
                false,
            ));
        } else {
            fields = fields
                .push(
                    row![
                        input("Servidor", &self.form.host, Field::Host, false),
                        input("Puerto", &self.form.port, Field::Port, false).width(100),
                        input("Base de datos", &self.form.database, Field::Database, false)
                    ]
                    .spacing(10),
                )
                .push(
                    row![
                        input("Usuario", &self.form.user, Field::User, false),
                        input("Contraseña", &self.form.password, Field::Password, true),
                    ]
                    .spacing(10),
                )
                .push(
                    row![
                        checkbox("Guardar contraseña", self.form.remember_password)
                            .on_toggle(Message::RememberPassword),
                        checkbox("TLS", self.form.tls).on_toggle(Message::Tls),
                        checkbox("Verificar certificado", self.form.tls_verify)
                            .on_toggle_maybe(self.form.tls.then_some(Message::TlsVerify)),
                    ]
                    .spacing(16),
                );
        }
        fields = fields.push(
            row![
                button(icon_button(Bootstrap::Save, "Guardar")).on_press(Message::SaveProfile),
                button(icon_button(Bootstrap::PlugFill, "Conectar"))
                    .style(accent_button(self.dark))
                    .on_press(Message::ConnectProfile),
                button(icon_button(Bootstrap::XLg, "Cancelar"))
                    .style(button::secondary)
                    .on_press(Message::CloseConnectionForm),
            ]
            .spacing(12),
        );
        fields = fields.push(
            text("Si guardás la contraseña, queda en el almacén de credenciales del sistema — nunca en el archivo de conexiones.")
                .size(13),
        );
        if let Some(error) = &self.form_error {
            fields = fields.push(
                text(error)
                    .size(13)
                    .color(self.active_theme().extended_palette().danger.base.color),
            );
        }
        container(fields)
            .padding(16)
            .style(card(theme::panel(self.dark), theme::border(self.dark)))
            .into()
    }

    fn connections_tree(&self) -> Element<'_, Message> {
        let current_theme = self.active_theme();
        let palette = current_theme.extended_palette();
        let mut list = column![].spacing(4);
        if self.connections.is_empty() {
            list = list.push(text("Sin conexiones guardadas").size(13));
        }
        for entry in &self.connections {
            let selected = self.selected_connection == Some(entry.id);
            let (status_glyph, status_color) = match &entry.state {
                ConnState::Disconnected => (Bootstrap::Circle, palette.background.strong.color),
                ConnState::Connecting => (Bootstrap::CircleFill, palette.primary.base.color),
                ConnState::Connected { .. } => (Bootstrap::CircleFill, palette.success.base.color),
                ConnState::Error(_) => (Bootstrap::CircleFill, palette.danger.base.color),
            };
            list = list.push(
                row![
                    button(
                        row![
                            icon(status_glyph).size(12).color(status_color),
                            text(&entry.profile.name).size(14)
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .style(if selected {
                        button::primary
                    } else {
                        button::text
                    })
                    .width(Length::Fill)
                    .on_press(Message::SelectConnection(entry.id)),
                    badge(
                        entry.profile.driver.to_string(),
                        driver_badge_color(entry.profile.driver)
                    ),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
            if selected {
                let mut actions = row![icon_only_button(
                    Bootstrap::PencilSquare,
                    "Editar",
                    Message::EditConnection(entry.id)
                )]
                .spacing(4);
                if matches!(entry.state, ConnState::Connected { .. }) {
                    actions = actions.push(icon_only_button(
                        Bootstrap::PlugFill,
                        "Desconectar",
                        Message::Disconnect,
                    ));
                }
                let pending = self.pending_delete == Some(entry.id);
                let delete_icon = if pending {
                    Bootstrap::ExclamationTriangleFill
                } else {
                    Bootstrap::Trash
                };
                let delete_hint = if pending {
                    "¿Confirmar eliminar?"
                } else {
                    "Eliminar"
                };
                actions = actions.push(icon_only_button(
                    delete_icon,
                    delete_hint,
                    Message::DeleteConnection,
                ));
                list = list.push(indent(1, actions.into()));
            }
            if let ConnState::Error(message) = &entry.state {
                list = list.push(indent(1, text(message.as_str()).size(12).into()));
            }
            let node_key = entry.id.to_string();
            if !self.expanded.contains(&node_key) {
                continue;
            }
            if let ConnState::Connected { catalog, .. } = &entry.state {
                if matches!(entry.profile.driver, Driver::Postgres | Driver::Mongo) {
                    let mut schemas: Vec<&str> =
                        catalog.iter().filter_map(|o| o.schema.as_deref()).collect();
                    schemas.sort_unstable();
                    schemas.dedup();
                    for schema in schemas {
                        let schema_key = format!("{node_key}/{schema}");
                        let schema_expanded = self.expanded.contains(&schema_key);
                        let caret = if schema_expanded {
                            Bootstrap::CaretDownFill
                        } else {
                            Bootstrap::CaretRightFill
                        };
                        list = list.push(indent(
                            1,
                            button(
                                row![
                                    icon(caret).size(11),
                                    icon(Bootstrap::FolderFill).size(12),
                                    text(schema).size(13)
                                ]
                                .spacing(6)
                                .align_y(Alignment::Center),
                            )
                            .style(button::text)
                            .on_press(Message::ToggleExpanded(schema_key.clone()))
                            .into(),
                        ));
                        if !schema_expanded {
                            continue;
                        }
                        let objects: Vec<&CatalogObject> = catalog
                            .iter()
                            .filter(|o| o.schema.as_deref() == Some(schema))
                            .collect();
                        list = push_kind_folders(
                            list,
                            &self.expanded,
                            entry.id,
                            &schema_key,
                            2,
                            &objects,
                        );
                    }
                } else {
                    let objects: Vec<&CatalogObject> = catalog.iter().collect();
                    list =
                        push_kind_folders(list, &self.expanded, entry.id, &node_key, 1, &objects);
                }
            }
        }
        list.into()
    }

    pub fn view(&self, window: window::Id) -> Element<'_, Message> {
        if Some(window) == self.connection_window {
            let bar = self.title_bar(
                window,
                None,
                Some(&self.title(window)),
                Message::CloseConnectionForm,
                false,
            );
            return column![bar, self.connection_form()].into();
        }
        let bar = self.title_bar(window, Some(self.menus()), None, Message::Exit, true);
        let toolbar = row![
            text("RusioDB").size(26),
            button(icon_button(Bootstrap::DatabaseAdd, "Nueva conexión"))
                .on_press(Message::NewConnection),
            button("Nueva consulta").on_press(Message::NewQuery),
            button(icon_button(Bootstrap::PlayFill, "Ejecutar  F5"))
                .style(accent_button(self.dark))
                .on_press_maybe(self.can_run().then_some(Message::Run)),
            button(icon_button(Bootstrap::ArrowClockwise, "Actualizar  F6"))
                .on_press_maybe(self.refresh_enabled().then_some(Message::Refresh))
        ]
        .spacing(12);
        let chrome = container(toolbar).padding([10, 14]);
        let mut layout = column![chrome, horizontal_rule(1)].spacing(8);
        if self.about {
            layout = layout.push(
                container(
                    row![
                        text(format!(
                            "RusioDB {} · Rust + Iced · SQLite / PostgreSQL / MySQL / MongoDB",
                            env!("CARGO_PKG_VERSION")
                        )),
                        button("Cerrar").on_press(Message::About)
                    ]
                    .spacing(12),
                )
                .padding(12)
                .style(card(theme::panel(self.dark), theme::border(self.dark))),
            );
        }
        if let Some(info) = &self.update_available {
            layout = layout.push(
                container(
                    row![
                        text(format!("Nueva versión disponible: {}", info.version)),
                        button("Instalar")
                            .on_press_maybe((!self.updating).then_some(Message::InstallUpdate)),
                        button("Cerrar").on_press(Message::DismissUpdateBanner),
                    ]
                    .spacing(12),
                )
                .padding(12)
                .style(card(theme::panel(self.dark), theme::border(self.dark))),
            );
        }
        let any_connected = self
            .connections
            .iter()
            .any(|e| matches!(e.state, ConnState::Connected { .. }));
        let can_close_tabs = self.tabs.len() > 1;
        let mut tab_bar = row![].spacing(4);
        for (index, tab) in self.tabs.iter().enumerate() {
            let label = if tab.running {
                format!("{} ⏳", tab.title)
            } else {
                tab.title.clone()
            };
            let selected = index == self.selected;
            let chip = row![
                button(text(label).size(13))
                    .padding([6, 4])
                    .style(button::text)
                    .on_press(Message::SelectTab(index)),
                button(icon(Bootstrap::XLg).size(9))
                    .padding(4)
                    .style(button::text)
                    .on_press_maybe(can_close_tabs.then_some(Message::CloseTab(index))),
            ]
            .spacing(2)
            .align_y(Alignment::Center);
            tab_bar = tab_bar.push(container(chip).padding([2, 8]).style(move |theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: selected.then_some(palette.primary.weak.color.into()),
                    border: Border {
                        radius: iced::border::Radius {
                            top_left: 6.0,
                            top_right: 6.0,
                            bottom_left: 0.0,
                            bottom_right: 0.0,
                        },
                        ..Border::default()
                    },
                    ..container::Style::default()
                }
            }));
        }
        let tab = &self.tabs[self.selected];
        let current_theme = self.active_theme();
        let palette = current_theme.extended_palette();
        let header_bg = theme::panel(self.dark);
        let alt_row_bg = theme::alt_row(self.dark);
        let muted = palette.background.strong.color;
        let danger = palette.danger.base.color;

        let mut header_row = row![].spacing(1);
        for name in &tab.result.columns {
            header_row = header_row.push(
                container(text(name.to_uppercase()).size(11).color(muted))
                    .padding([6, 10])
                    .width(200),
            );
        }
        let mut grid = column![container(header_row).style(tinted_box(header_bg))].spacing(1);
        for (index, values) in tab.result.rows.iter().enumerate() {
            let mut cells = row![].spacing(1);
            for value in values {
                let is_null = value == "NULL";
                let is_numeric = !is_null && value.trim().parse::<f64>().is_ok();
                let mut cell_text = text(value).size(13);
                if is_null {
                    cell_text = cell_text.color(muted);
                }
                let mut cell = container(cell_text).padding([4, 10]).width(200);
                if is_numeric {
                    cell = cell.align_x(Horizontal::Right);
                }
                cells = cells.push(cell);
            }
            let mut data_row = container(cells);
            if index % 2 == 1 {
                data_row = data_row.style(tinted_box(alt_row_bg));
            }
            grid = grid.push(data_row);
        }
        if tab.result.columns.is_empty() {
            grid = grid.push(text("Ejecuta una consulta para ver los resultados."));
        }
        let results = scrollable(grid)
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .height(Length::Fill);

        let connection_label = tab
            .connection_id
            .and_then(|id| self.connections.iter().find(|e| e.id == id))
            .map(|e| e.profile.name.clone())
            .unwrap_or_else(|| "Sin conexión".into());
        let rows_label = if tab.result.columns.is_empty() {
            tab.result.affected.to_string()
        } else {
            tab.result.rows.len().to_string()
        };
        let elapsed_label = tab
            .last_elapsed_ms
            .map(|ms| format!("{ms} ms"))
            .unwrap_or_else(|| "—".into());
        let stats_row = row![
            stat("CONEXIÓN", connection_label, None),
            stat(
                if tab.result.columns.is_empty() {
                    "AFECTADAS"
                } else {
                    "FILAS"
                },
                rows_label,
                None
            ),
            stat("TIEMPO", elapsed_label, None),
            stat(
                "TRUNCADO",
                if tab.result.truncated { "Sí" } else { "No" }.into(),
                tab.result.truncated.then_some(danger)
            ),
        ]
        .spacing(28);

        let workspace: Element<'_, Message> = if any_connected {
            column![
                scrollable(tab_bar).direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::default()
                )),
                text_editor(&tab.editor)
                    .on_action(Message::Edit)
                    .key_binding(Self::editor_binding)
                    .height(200),
                text("Ejecuta una sentencia por vez · Vista de hasta 500 filas").size(13),
                row![
                    text("Resultados").size(20),
                    stats_row,
                    button("Copiar resultados").on_press_maybe(
                        (!tab.result.columns.is_empty()).then_some(Message::CopyResults)
                    )
                ]
                .spacing(20)
                .align_y(Alignment::Center),
                results
            ]
            .spacing(10)
            .width(Length::Fill)
            .into()
        } else {
            container(
                column![
                    text("Sin conexión activa").size(22),
                    text("Seleccioná una conexión en el panel izquierdo para empezar.").size(14),
                    button(icon_button(Bootstrap::DatabaseAdd, "Nueva conexión"))
                        .style(accent_button(self.dark))
                        .on_press(Message::NewConnection)
                ]
                .spacing(12)
                .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };
        let mut content = row![].spacing(0).height(Length::Fill);
        if self.show_explorer {
            content = content
                .push(
                    container(
                        column![
                            row![
                                text("Conexiones").size(20),
                                button("Nueva")
                                    .style(button::text)
                                    .on_press(Message::NewConnection)
                            ]
                            .spacing(12),
                            scrollable(self.connections_tree()).height(Length::Fill)
                        ]
                        .spacing(12),
                    )
                    .padding(Padding {
                        top: 14.0,
                        right: 14.0,
                        bottom: 14.0,
                        left: 0.0,
                    })
                    .width(260)
                    .height(Length::Fill),
                )
                .push(vertical_rule(1));
        }
        layout = layout.push(
            content.push(
                container(workspace)
                    .padding(Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: 14.0,
                    })
                    .width(Length::Fill),
            ),
        );
        layout = layout.push(horizontal_rule(1)).push(
            container(text(&self.status).size(13))
                .padding([8, 14])
                .width(Length::Fill),
        );
        column![
            bar,
            container(layout).padding(Padding {
                top: 8.0,
                right: 14.0,
                bottom: 14.0,
                left: 14.0,
            })
        ]
        .into()
    }
}

fn tinted_box(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style {
        background: Some(color.into()),
        ..container::Style::default()
    }
}

/// Píldora chica de color (fondo tenue + borde + texto del mismo color),
/// para etiquetas cortas como el motor de una conexión o un estado puntual.
fn badge<'a>(label: impl Into<String>, color: Color) -> Element<'a, Message> {
    container(text(label.into()).size(11))
        .padding([2, 8])
        .style(move |_theme: &Theme| container::Style {
            background: Some(Color { a: 0.16, ..color }.into()),
            border: Border {
                color: Color { a: 0.45, ..color },
                width: 1.0,
                radius: 999.0.into(),
            },
            text_color: Some(color),
            ..container::Style::default()
        })
        .into()
}

fn kind_icon(kind: ObjectKind) -> Bootstrap {
    match kind {
        ObjectKind::Table => Bootstrap::Table,
        ObjectKind::Collection => Bootstrap::Table,
        ObjectKind::View => Bootstrap::EyeFill,
        ObjectKind::MaterializedView => Bootstrap::Layers,
        ObjectKind::Function => Bootstrap::Braces,
        ObjectKind::Procedure => Bootstrap::Gear,
    }
}

fn driver_badge_color(driver: Driver) -> Color {
    match driver {
        Driver::Sqlite => Color::from_rgb8(0x0D, 0x9C, 0x9C),
        Driver::Postgres => Color::from_rgb8(0x33, 0x6D, 0xE0),
        Driver::Mysql => Color::from_rgb8(0xE0, 0x8E, 0x1D),
        Driver::Mongo => Color::from_rgb8(0x47, 0xA2, 0x48),
    }
}

/// Estilo de botón para la única acción de acento de cada pantalla (ver
/// [`theme::accent`]). Mismo tratamiento hover/pressed/disabled que
/// `button::primary`, pero con el color de acento en vez del azul del tema.
fn accent_button(dark: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, status: button::Status| {
        let (background, text_color) = match status {
            button::Status::Hovered | button::Status::Pressed => {
                (theme::accent_strong(dark), Color::WHITE)
            }
            button::Status::Disabled => (
                Color {
                    a: 0.35,
                    ..theme::accent(dark)
                },
                Color {
                    a: 0.6,
                    ..Color::WHITE
                },
            ),
            button::Status::Active => (theme::accent(dark), Color::WHITE),
        };
        button::Style {
            background: Some(background.into()),
            text_color,
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
        }
    }
}

/// Panel tipo tarjeta: fondo, borde sutil, esquinas redondeadas y una sombra
/// leve para dar profundidad — en vez de un bloque de color plano flotando en
/// el vacío.
fn card(background: Color, border_color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style {
        background: Some(background.into()),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        ..container::Style::default()
    }
}

/// Estilo del menú (Archivo/Edición/...): la barra en sí queda transparente
/// porque ya vive dentro de la tarjeta de "chrome"; el desplegable flotante
/// sí recibe fondo, borde y sombra propios, coherentes con el resto de la UI
/// en lugar del gris genérico por defecto de `iced_aw`.
fn menu_style(dark: bool) -> impl Fn(&Theme, iced_aw::style::Status) -> MenuStyle {
    move |theme: &Theme, _status: iced_aw::style::Status| {
        let palette = theme.extended_palette();
        MenuStyle {
            bar_background: Color::TRANSPARENT.into(),
            bar_border: Border::default(),
            bar_shadow: Shadow::default(),
            bar_background_expand: Padding::ZERO,
            menu_background: theme::panel(dark).into(),
            menu_border: Border {
                color: theme::border(dark),
                width: 1.0,
                radius: 8.0.into(),
            },
            menu_shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.28),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
            menu_background_expand: Padding::from(6),
            path: palette.primary.weak.color.into(),
            path_border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
        }
    }
}

fn indent<'a>(depth: usize, content: Element<'a, Message>) -> Element<'a, Message> {
    row![Space::with_width((depth as f32) * 16.0), content].into()
}

fn push_kind_folders<'a>(
    mut list: Column<'a, Message>,
    expanded: &HashSet<String>,
    id: ConnectionId,
    prefix: &str,
    depth: usize,
    objects: &[&'a CatalogObject],
) -> Column<'a, Message> {
    for kind in KIND_ORDER {
        let items: Vec<&CatalogObject> =
            objects.iter().filter(|o| o.kind == kind).copied().collect();
        if items.is_empty() {
            continue;
        }
        let key = format!("{prefix}/{kind:?}");
        let folder_expanded = expanded.contains(&key);
        let caret = if folder_expanded {
            Bootstrap::CaretDownFill
        } else {
            Bootstrap::CaretRightFill
        };
        list = list.push(indent(
            depth,
            button(
                row![
                    icon(caret).size(11),
                    icon(kind_icon(kind)).size(12),
                    text(kind.folder_label()).size(13)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .style(button::text)
            .on_press(Message::ToggleExpanded(key.clone()))
            .into(),
        ));
        if folder_expanded {
            for object in items {
                let leaf = row![
                    icon(kind_icon(kind)).size(11),
                    text(object.name.as_str()).size(13)
                ]
                .spacing(6)
                .align_y(Alignment::Center);
                let row_content: Element<'a, Message> = if kind.is_previewable() {
                    button(leaf)
                        .style(button::text)
                        .width(Length::Fill)
                        .on_press(Message::Preview(id, object.clone()))
                        .into()
                } else {
                    leaf.into()
                };
                list = list.push(indent(depth + 1, row_content));
            }
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected_entry(id: ConnectionId, name: &str) -> ConnectionEntry {
        let mut database = Database::connect(&Config::default()).unwrap();
        let catalog = database.catalog().unwrap();
        ConnectionEntry {
            id,
            profile: ConnectionProfile {
                id,
                name: name.into(),
                driver: Driver::Sqlite,
                path: ":memory:".into(),
                host: String::new(),
                port: String::new(),
                database: String::new(),
                user: String::new(),
                tls: false,
                tls_verify: true,
            },
            state: ConnState::Connected {
                database: Arc::new(Mutex::new(database)),
                catalog,
            },
            catalog_busy: false,
        }
    }

    #[test]
    fn command_enter_runs_sql_instead_of_inserting_a_newline() {
        let modifiers = if cfg!(target_os = "macos") {
            keyboard::Modifiers::LOGO
        } else {
            keyboard::Modifiers::CTRL
        };
        let event = text_editor::KeyPress {
            key: keyboard::Key::Named(keyboard::key::Named::Enter),
            modifiers,
            text: None,
            status: text_editor::Status::Focused,
        };
        assert!(matches!(
            App::editor_binding(event),
            Some(text_editor::Binding::Custom(Message::Run))
        ));
    }
    #[test]
    fn new_query_preserves_existing_sql_and_form_driver_is_independent_of_tabs() {
        let mut app = App::default();
        let original = app.tabs[0].editor.text();
        let _ = app.update(Message::NewQuery);
        assert_eq!(app.tabs[0].editor.text(), original);
        assert_eq!(app.selected, 1);
        let _ = app.update(Message::Driver(Driver::Mysql));
        assert_eq!(app.form.port, "3306");
        assert!(app.connections.is_empty());
    }
    #[test]
    fn closing_a_tab_removes_it_and_keeps_a_valid_selection() {
        let mut app = App::default();
        let _ = app.update(Message::NewQuery);
        let _ = app.update(Message::NewQuery);
        assert_eq!(app.tabs.len(), 3);
        app.selected = 2;
        let _ = app.update(Message::CloseTab(0));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(
            app.selected, 1,
            "debe seguir apuntando a la misma pestaña tras el corrimiento"
        );
    }
    #[test]
    fn closing_the_last_remaining_tab_is_a_no_op() {
        let mut app = App::default();
        let _ = app.update(Message::CloseTab(0));
        assert_eq!(app.tabs.len(), 1);
    }
    #[test]
    fn editing_a_different_profile_is_allowed_while_one_connection_is_connecting() {
        let mut app = App::default();
        app.connections.push(ConnectionEntry {
            id: 1,
            profile: ConnectionProfile {
                id: 1,
                name: "Conectando".into(),
                driver: Driver::Sqlite,
                path: ":memory:".into(),
                host: String::new(),
                port: String::new(),
                database: String::new(),
                user: String::new(),
                tls: false,
                tls_verify: true,
            },
            state: ConnState::Connecting,
            catalog_busy: false,
        });
        let _ = app.update(Message::Field(Field::Name, "Otra".into()));
        assert_eq!(app.form.name, "Otra");
        let _ = app.update(Message::NewQuery);
        assert_eq!(app.tabs.len(), 2);
        let _ = app.update(Message::SelectTab(1));
        assert_eq!(app.selected, 1);
    }
    #[test]
    fn two_tabs_on_different_connections_can_run_simultaneously() {
        let mut app = App::default();
        app.connections.push(connected_entry(1, "A"));
        app.connections.push(connected_entry(2, "B"));
        app.tabs = vec![
            Tab::new("A".into(), "SELECT 1", Some(1)),
            Tab::new("B".into(), "SELECT 1", Some(2)),
        ];
        app.tabs[0].running = true;
        app.selected = 1;
        let _ = app.update(Message::Run);
        assert!(
            app.tabs[0].running,
            "la pestaña A debe seguir marcada como en ejecución"
        );
        assert!(
            app.tabs[1].running,
            "la pestaña B debe pasar a en ejecución"
        );
    }
    #[test]
    fn disconnecting_one_connection_does_not_affect_another() {
        let mut app = App::default();
        app.connections.push(connected_entry(1, "A"));
        app.connections.push(connected_entry(2, "B"));
        app.selected_connection = Some(1);
        let _ = app.update(Message::Disconnect);
        assert!(matches!(
            app.connections.iter().find(|e| e.id == 1).unwrap().state,
            ConnState::Disconnected
        ));
        assert!(matches!(
            app.connections.iter().find(|e| e.id == 2).unwrap().state,
            ConnState::Connected { .. }
        ));
    }
    #[test]
    fn new_connection_opens_a_connection_window_without_touching_main_window() {
        let mut app = App::default();
        let _ = app.update(Message::NewConnection);
        assert!(app.connection_window.is_some());
        assert_eq!(app.main_window, None);
    }
    #[test]
    fn new_connection_twice_reuses_the_existing_connection_window() {
        let mut app = App::default();
        let _ = app.update(Message::NewConnection);
        let first = app.connection_window;
        let _ = app.update(Message::NewConnection);
        assert_eq!(app.connection_window, first);
    }
    #[test]
    fn close_connection_form_clears_dialog_state() {
        let mut app = App::default();
        let _ = app.update(Message::NewConnection);
        app.editing_profile = Some(9);
        app.form_error = Some("x".into());
        let _ = app.update(Message::CloseConnectionForm);
        assert_eq!(app.connection_window, None);
        assert_eq!(app.editing_profile, None);
        assert!(app.form_error.is_none());
    }
    #[test]
    fn save_profile_closes_the_connection_window() {
        let mut app = App::default();
        let _ = app.update(Message::NewConnection);
        app.form.name = "Nueva".into();
        let _ = app.update(Message::SaveProfile);
        assert!(app.connection_window.is_none());
        assert_eq!(app.connections.len(), 1);
    }
    #[test]
    fn connected_error_while_watching_sets_form_error_and_keeps_dialog_open() {
        let mut app = App::default();
        let dialog_id = window::Id::unique();
        app.connection_window = Some(dialog_id);
        app.editing_profile = Some(1);
        app.connections.push(ConnectionEntry {
            id: 1,
            profile: ConnectionProfile {
                id: 1,
                name: "Falla".into(),
                driver: Driver::Postgres,
                path: String::new(),
                host: "localhost".into(),
                port: "5432".into(),
                database: "db".into(),
                user: "user".into(),
                tls: false,
                tls_verify: true,
            },
            state: ConnState::Connecting,
            catalog_busy: false,
        });
        let _ = app.update(Message::Connected(1, Err("boom".into())));
        assert_eq!(app.form_error.as_deref(), Some("boom"));
        assert_eq!(app.connection_window, Some(dialog_id));
    }
    #[test]
    fn connected_success_while_watching_closes_the_dialog() {
        let mut app = App::default();
        let dialog_id = window::Id::unique();
        app.connection_window = Some(dialog_id);
        app.editing_profile = Some(1);
        app.connections.push(ConnectionEntry {
            id: 1,
            profile: ConnectionProfile {
                id: 1,
                name: "Sqlite".into(),
                driver: Driver::Sqlite,
                path: ":memory:".into(),
                host: String::new(),
                port: String::new(),
                database: String::new(),
                user: String::new(),
                tls: false,
                tls_verify: true,
            },
            state: ConnState::Connecting,
            catalog_busy: false,
        });
        let database = Database::connect(&Config::default()).unwrap();
        let catalog = vec![];
        let _ = app.update(Message::Connected(
            1,
            Ok((Arc::new(Mutex::new(database)), catalog)),
        ));
        assert_eq!(app.connection_window, None);
        assert!(app.form_error.is_none());
    }
    #[test]
    fn window_closed_for_connection_window_clears_dialog_state_without_touching_main_window() {
        let mut app = App::default();
        let main_id = window::Id::unique();
        let dialog_id = window::Id::unique();
        app.main_window = Some(main_id);
        app.connection_window = Some(dialog_id);
        app.editing_profile = Some(7);
        let _ = app.update(Message::WindowClosed(dialog_id));
        assert_eq!(app.connection_window, None);
        assert_eq!(app.editing_profile, None);
        assert_eq!(app.main_window, Some(main_id));
    }
    #[test]
    fn is_main_window_predicate_identifies_only_the_main_window_id() {
        let mut app = App::default();
        let main_id = window::Id::unique();
        app.main_window = Some(main_id);
        assert!(app.is_main_window(main_id));
        assert!(!app.is_main_window(window::Id::unique()));
    }
    #[test]
    fn default_app_never_touches_the_real_os_keyring() {
        // Guarda de seguridad: solo `App::load()` (el arranque real) debe
        // habilitar el almacén de credenciales. Si este test fallara, los
        // tests que ejercen SaveProfile/ConnectProfile/DeleteConnection
        // empezarían a escribir contraseñas de prueba en el llavero real del
        // desarrollador.
        assert!(!App::default().keyring_enabled);
    }
    #[test]
    fn default_app_never_checks_for_updates() {
        // Mismo espíritu que el test del keyring: el chequeo de
        // actualizaciones solo se dispara desde `App::load()` (vía
        // `spawn_check_updates` en el `Task::batch` inicial) o desde una
        // acción explícita del usuario (`Message::CheckForUpdates`), nunca
        // como efecto secundario de construir un `App` en un test.
        assert!(App::default().update_available.is_none());
        assert!(!App::default().updating);
    }
}
