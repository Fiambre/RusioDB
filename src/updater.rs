//! Chequeo y aplicación de actualizaciones contra los Releases de GitHub.
//!
//! El parseo/comparación de versión vive en funciones puras
//! (`parse_release_json`) separadas de las llamadas de red, para poder
//! testearlas sin conexión — ver `#[cfg(test)] mod tests` al final.

use std::path::{Path, PathBuf};

const REPO_RELEASES_API: &str = "https://api.github.com/repos/Fiambre/RusioDB/releases/latest";
const ASSET_NAME: &str = "rusiodb-x86_64-pc-windows-msvc.exe";

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateInfo {
    pub version: semver::Version,
    pub download_url: String,
    pub notes: String,
}

/// Compara la versión del release remoto contra `current` y arma un
/// `UpdateInfo` si hay una más nueva disponible para esta plataforma.
/// No hace ninguna llamada de red — recibe el JSON ya obtenido, así se puede
/// testear con un JSON armado a mano.
fn parse_release_json(
    json: &serde_json::Value,
    current: &semver::Version,
) -> Result<Option<UpdateInfo>, String> {
    let tag = json["tag_name"]
        .as_str()
        .ok_or("respuesta inesperada de GitHub: falta tag_name")?;
    let remote = semver::Version::parse(tag.trim_start_matches('v'))
        .map_err(|e| format!("versión remota inválida ({tag}): {e}"))?;
    if remote <= *current {
        return Ok(None);
    }
    let assets = json["assets"]
        .as_array()
        .ok_or("respuesta inesperada de GitHub: falta assets")?;
    let download_url = assets
        .iter()
        .find(|asset| asset["name"].as_str() == Some(ASSET_NAME))
        .and_then(|asset| asset["browser_download_url"].as_str())
        .ok_or_else(|| format!("no se encontró el instalador '{ASSET_NAME}' en la última versión"))?
        .to_string();
    let notes = json["body"].as_str().unwrap_or_default().to_string();
    Ok(Some(UpdateInfo {
        version: remote,
        download_url,
        notes,
    }))
}

/// Consulta el último release de GitHub y devuelve `Some(UpdateInfo)` si hay
/// una versión más nueva que la actual (`CARGO_PKG_VERSION`). Un 404 (todavía
/// no existe ningún release publicado) se trata como "sin actualización",
/// no como error.
pub async fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("RusioDB/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(REPO_RELEASES_API)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(format!("GitHub respondió {}", response.status()));
    }
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    parse_release_json(&json, &current)
}

/// Descarga el asset de la nueva versión a un archivo temporal y devuelve su
/// ruta. La escritura a disco corre en un hilo bloqueante; la descarga en sí
/// es async nativa (no hace falta `spawn_blocking` para el `GET`).
pub async fn download_update(url: String) -> Result<PathBuf, String> {
    let bytes = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    let path = std::env::temp_dir().join("rusiodb-update.exe");
    let write_path = path.clone();
    tokio::task::spawn_blocking(move || std::fs::write(&write_path, &bytes))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(path)
}

/// Reemplaza el ejecutable en curso por el que se descargó. Bloqueante
/// (llamadas de archivo/Win32 síncronas) — el llamador debe correrlo dentro
/// de `spawn_blocking`.
pub fn apply_update(new_exe: &Path) -> Result<(), String> {
    self_replace::self_replace(new_exe).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(new_exe);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn current() -> semver::Version {
        semver::Version::new(0, 1, 0)
    }

    fn release_json(tag: &str, asset_name: &str) -> serde_json::Value {
        json!({
            "tag_name": tag,
            "body": "Notas de la versión",
            "assets": [
                { "name": asset_name, "browser_download_url": "https://example.com/asset.exe" }
            ]
        })
    }

    #[test]
    fn newer_remote_version_yields_update_info() {
        let json = release_json("v0.2.0", ASSET_NAME);
        let info = parse_release_json(&json, &current()).unwrap().unwrap();
        assert_eq!(info.version, semver::Version::new(0, 2, 0));
        assert_eq!(info.download_url, "https://example.com/asset.exe");
        assert_eq!(info.notes, "Notas de la versión");
    }

    #[test]
    fn equal_or_older_remote_version_yields_no_update() {
        assert!(
            parse_release_json(&release_json("v0.1.0", ASSET_NAME), &current())
                .unwrap()
                .is_none()
        );
        assert!(
            parse_release_json(&release_json("v0.0.9", ASSET_NAME), &current())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn missing_asset_for_this_platform_is_an_error() {
        let error = parse_release_json(
            &release_json("v0.2.0", "rusiodb-other-platform.exe"),
            &current(),
        )
        .unwrap_err();
        assert!(error.contains(ASSET_NAME));
    }

    #[test]
    fn malformed_tag_is_an_error() {
        let json = release_json("no-es-un-semver", ASSET_NAME);
        assert!(parse_release_json(&json, &current()).is_err());
    }

    #[test]
    fn missing_tag_name_is_an_error() {
        let json = json!({ "assets": [] });
        assert!(parse_release_json(&json, &current()).is_err());
    }

    #[test]
    #[ignore = "Requiere red: pega contra la API real de GitHub"]
    fn update_check_integration() {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(check_for_update());
        assert!(
            result.is_ok(),
            "el chequeo contra la API real falló: {result:?}"
        );
    }
}
