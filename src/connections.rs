//! Perfiles de conexión guardados, persistidos en un archivo JSON local. La
//! contraseña nunca viaja por ese archivo: si el usuario elige recordarla, se
//! guarda aparte en el almacén de credenciales del sistema operativo (ver
//! [`save_password`]).
use crate::drivers::{Config, Driver};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const KEYRING_SERVICE: &str = "RusioDB";

/// Guarda la contraseña de una conexión en el almacén de credenciales del
/// sistema operativo (Windows Credential Manager / macOS Keychain / Secret
/// Service en Linux) — nunca en `connections.json`.
pub fn save_password(id: u64, password: &str) -> Result<(), String> {
    keyring::Entry::new(KEYRING_SERVICE, &id.to_string())
        .and_then(|entry| entry.set_password(password))
        .map_err(|e| e.to_string())
}

/// Recupera la contraseña guardada para una conexión, si existe. Cualquier
/// error (no hay entrada, el almacén no está disponible, etc.) se trata como
/// "no guardada" en vez de propagarse: el usuario siempre puede tipearla.
pub fn load_password(id: u64) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, &id.to_string())
        .ok()?
        .get_password()
        .ok()
}

/// Elimina la contraseña guardada de una conexión (al borrarla o al
/// desmarcar "Guardar contraseña"). Falta de entrada no es un error.
pub fn delete_password(id: u64) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &id.to_string()) {
        let _ = entry.delete_credential();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub id: u64,
    pub name: String,
    pub driver: Driver,
    pub path: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub user: String,
    pub tls: bool,
    /// Si es `false`, se acepta el certificado TLS del servidor sin validarlo
    /// contra el almacén de certificados del sistema (necesario para
    /// servidores internos con certificados autofirmados). `#[serde(default
    /// = ...)]` porque los `connections.json` guardados antes de agregar este
    /// campo no lo tienen, y el valor seguro por defecto es `true`.
    #[serde(default = "default_tls_verify")]
    pub tls_verify: bool,
}

fn default_tls_verify() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionsFile {
    pub next_id: u64,
    pub profiles: Vec<ConnectionProfile>,
}

pub fn default_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("RusioDB").join("connections.json"))
}

pub fn load(path: &Path) -> ConnectionsFile {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, file: &ConnectionsFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn profile_from_config(id: u64, config: &Config) -> ConnectionProfile {
    let name = if config.name.trim().is_empty() {
        config.label()
    } else {
        config.name.clone()
    };
    ConnectionProfile {
        id,
        name,
        driver: config.driver,
        path: config.path.clone(),
        host: config.host.clone(),
        port: config.port.clone(),
        database: config.database.clone(),
        user: config.user.clone(),
        tls: config.tls,
        tls_verify: config.tls_verify,
    }
}

pub fn config_from_profile(profile: &ConnectionProfile) -> Config {
    Config {
        name: profile.name.clone(),
        driver: profile.driver,
        path: profile.path.clone(),
        host: profile.host.clone(),
        port: profile.port.clone(),
        database: profile.database.clone(),
        user: profile.user.clone(),
        password: String::new(),
        tls: profile.tls,
        tls_verify: profile.tls_verify,
        remember_password: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rusiodb_test_{label}_{}_{}.json",
            std::process::id(),
            label.len()
        ))
    }

    #[test]
    fn round_trip_never_serializes_a_password() {
        let file = ConnectionsFile {
            next_id: 2,
            profiles: vec![ConnectionProfile {
                id: 1,
                name: "Mi Postgres".into(),
                driver: Driver::Postgres,
                path: String::new(),
                host: "localhost".into(),
                port: "5432".into(),
                database: "app".into(),
                user: "admin".into(),
                tls: true,
                tls_verify: true,
            }],
        };
        let json = serde_json::to_string(&file).unwrap();
        assert!(!json.contains("password"));
        let restored: ConnectionsFile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.next_id, 2);
        assert_eq!(restored.profiles[0].name, "Mi Postgres");
    }

    #[test]
    fn deserializing_a_profile_saved_before_tls_verify_existed_defaults_to_verifying() {
        let json = r#"{"id":1,"name":"Vieja","driver":"Postgres","path":"","host":"h","port":"5432","database":"d","user":"u","tls":true}"#;
        let profile: ConnectionProfile = serde_json::from_str(json).unwrap();
        assert!(profile.tls_verify);
    }

    #[test]
    fn load_returns_defaults_when_file_is_missing() {
        let path = temp_path("missing");
        let file = load(&path);
        assert_eq!(file.next_id, 0);
        assert!(file.profiles.is_empty());
    }

    #[test]
    fn save_then_load_round_trips_through_disk() {
        let path = temp_path("roundtrip");
        let file = ConnectionsFile {
            next_id: 5,
            profiles: vec![ConnectionProfile {
                id: 4,
                name: "Local".into(),
                driver: Driver::Sqlite,
                path: ":memory:".into(),
                host: String::new(),
                port: String::new(),
                database: String::new(),
                user: String::new(),
                tls: false,
                tls_verify: true,
            }],
        };
        save(&path, &file).unwrap();
        let restored = load(&path);
        assert_eq!(restored.next_id, 5);
        assert_eq!(restored.profiles[0].name, "Local");
        std::fs::remove_file(&path).unwrap();
    }
}
