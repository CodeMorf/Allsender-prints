use crate::models::AppConfig;
use crate::secure;
use directories::ProjectDirs;
use std::{fs, path::PathBuf};

fn app_dir() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("tech", "Allsender", "RestaAPP Printer")
        .ok_or_else(|| "No se pudo preparar el almacenamiento local".to_string())?;
    let dir = dirs.config_dir().to_path_buf();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn config_path() -> Result<PathBuf, String> {
    Ok(app_dir()?.join("config.json"))
}

pub fn logs_dir() -> Result<PathBuf, String> {
    let dir = app_dir()?.join("logs");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn diagnostics_dir() -> Result<PathBuf, String> {
    let dir = app_dir()?.join("diagnostics");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn queue_dir() -> Result<PathBuf, String> {
    let dir = app_dir()?.join("queue");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Load config and decrypt branch_key for in-memory use.
pub fn load() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        let cfg = AppConfig::default();
        save(&cfg)?;
        return Ok(cfg);
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut cfg: AppConfig = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if cfg.domain_locked || cfg.domain_url.trim().is_empty() {
        cfg.domain_url = crate::models::DOMAIN_URL.into();
    }
    if cfg.device_id.trim().is_empty() {
        cfg.device_id = uuid::Uuid::new_v4().to_string();
    }
    // Decrypt key for runtime
    cfg.branch_key = secure::unprotect(&cfg.branch_key)?;
    // Existing installations can keep an older version in their local config.
    // Always expose the version of the executable that is currently running.
    if cfg.app_version != crate::models::APP_VERSION {
        cfg.app_version = crate::models::APP_VERSION.into();
        if let Err(error) = save(&cfg) {
            tracing::warn!("No se pudo actualizar la versión guardada del agente: {error}");
        }
    }
    Ok(cfg)
}

/// Persist config with DPAPI-protected branch_key.
pub fn save(config: &AppConfig) -> Result<AppConfig, String> {
    let mut safe = config.clone();
    safe.domain_url = crate::models::DOMAIN_URL.into();
    safe.domain_locked = true;
    safe.app_version = crate::models::APP_VERSION.into();
    if safe.device_id.trim().is_empty() {
        safe.device_id = uuid::Uuid::new_v4().to_string();
    }
    // Protect key at rest
    let plain_key = safe.branch_key.clone();
    safe.branch_key = secure::protect(&plain_key)?;
    let raw = serde_json::to_string_pretty(&safe).map_err(|e| e.to_string())?;
    fs::write(config_path()?, raw).map_err(|e| e.to_string())?;
    // Return runtime copy with plain key for UI/session
    safe.branch_key = plain_key;
    Ok(safe)
}

/// Export diagnostic ZIP without full branch key.
pub fn export_diagnostics() -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Write;

    let out_dir = diagnostics_dir()?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let zip_path = out_dir.join(format!("restaapp-printer-diag-{stamp}.zip"));

    let file = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Sanitized config
    let cfg = load().unwrap_or_default();
    let diag = serde_json::json!({
        "app_version": crate::models::APP_VERSION,
        "device_id": cfg.device_id,
        "domain_url": cfg.domain_url,
        "branch_key_masked": secure::mask_key(&cfg.branch_key),
        "locale": cfg.locale,
        "autostart": cfg.autostart,
        "open_drawer_after_print": cfg.open_drawer_after_print,
        "poll_ms": cfg.poll_ms,
        "idle_poll_ms": cfg.idle_poll_ms,
        "printer_mappings": cfg.printer_mappings,
        "backup_mappings": cfg.backup_mappings,
        "copies": cfg.copies,
        "exported_at": chrono::Utc::now().to_rfc3339(),
    });
    let _ = diag;

    zip.start_file("config-sanitized.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        serde_json::to_string_pretty(&diag)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;

    // Recent log tails (last ~200KB per file, max 3 files)
    if let Ok(logs) = logs_dir() {
        let mut entries: Vec<_> = fs::read_dir(&logs)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "log").unwrap_or(false))
            .collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.metadata().map(|m| m.modified().ok()).ok().flatten()));
        for entry in entries.into_iter().take(3) {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "log.log".into());
            let data = fs::read(&path).unwrap_or_default();
            let tail = if data.len() > 200_000 {
                &data[data.len() - 200_000..]
            } else {
                &data[..]
            };
            // Redact possible full keys if present
            let text = String::from_utf8_lossy(tail).replace(&cfg.branch_key, &secure::mask_key(&cfg.branch_key));
            zip.start_file(format!("logs/{name}"), options)
                .map_err(|e| e.to_string())?;
            zip.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    // Environment note
    zip.start_file("environment.txt", options)
        .map_err(|e| e.to_string())?;
    let env_note = format!(
        "os={}\narch=x64\napp={}\n",
        std::env::consts::OS,
        crate::models::APP_VERSION
    );
    zip.write_all(env_note.as_bytes())
        .map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;

    // SHA256 sidecar
    let bytes = fs::read(&zip_path).map_err(|e| e.to_string())?;
    let hash = Sha256::digest(&bytes);
    let hash_hex = hex::encode(hash);
    fs::write(
        zip_path.with_extension("zip.sha256"),
        format!("{hash_hex}  {}\n", zip_path.file_name().unwrap().to_string_lossy()),
    )
    .ok();

    tracing::info!("Diagnóstico exportado (clave enmascarada)");
    Ok(zip_path.to_string_lossy().to_string())
}
