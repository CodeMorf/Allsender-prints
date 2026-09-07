use crate::{api, config, models::RuntimeStatus, printers};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::{mpsc, Mutex};

#[derive(Clone)]
pub struct RuntimeState {
    pub running: Arc<AtomicBool>,
    pub status: Arc<Mutex<RuntimeStatus>>,
    pub inflight: Arc<Mutex<HashSet<i64>>>,
    pub recent_done: Arc<Mutex<HashSet<i64>>>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(RuntimeStatus::default())),
            inflight: Arc::new(Mutex::new(HashSet::new())),
            recent_done: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

pub async fn current(state: &RuntimeState) -> RuntimeStatus {
    state.status.lock().await.clone()
}

pub async fn start(state: RuntimeState) -> Result<RuntimeStatus, String> {
    let cfg = config::load()?;
    if cfg.branch_key.trim().is_empty() {
        return Err("La sucursal debe conectarse primero".into());
    }
    if cfg.printer_mappings.is_empty() {
        return Err("Configura al menos una impresora".into());
    }
    if state.running.swap(true, Ordering::SeqCst) {
        return Ok(current(&state).await);
    }
    {
        let mut status = state.status.lock().await;
        status.running = true;
        status.state = "ready".into();
        status.last_message = Some("Servicio activo".into());
    }
    let worker_state = state.clone();
    tauri::async_runtime::spawn(async move {
        worker_loop(worker_state).await;
    });
    Ok(current(&state).await)
}

pub async fn stop(state: &RuntimeState) -> RuntimeStatus {
    state.running.store(false, Ordering::SeqCst);
    let mut status = state.status.lock().await;
    status.running = false;
    status.state = "paused".into();
    status.last_message = Some("Servicio pausado".into());
    status.clone()
}

async fn worker_loop(state: RuntimeState) {
    let client = match api::background_client() {
        Ok(client) => client,
        Err(error) => {
            tracing::error!("No se pudo crear el cliente: {}", error);
            let mut status = state.status.lock().await;
            status.state = "reconnecting".into();
            status.last_message = Some("Reconectando".into());
            return;
        }
    };

    let (sse_tx, mut sse_rx) = mpsc::channel::<()>(1);
    let sse_state = state.clone();
    let sse_task = tauri::async_runtime::spawn(async move {
        sse_signal_loop(sse_state, sse_tx).await;
    });
    let mut first_pull = true;
    let mut next_poll_ms = 1_500_u64;

    while state.running.load(Ordering::SeqCst) {
        if !first_pull {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(next_poll_ms.clamp(1_500, 30_000))) => {},
                signal = sse_rx.recv() => {
                    if signal.is_none() {
                        break;
                    }
                }
            }
        }
        first_pull = false;

        let cfg = match config::load() {
            Ok(cfg) => cfg,
            Err(error) => {
                tracing::error!("No se pudo leer la configuración: {}", error);
                next_poll_ms = 3_000;
                continue;
            }
        };

        match api::pull_jobs(&client, &cfg).await {
            Ok((jobs, next_poll)) => {
                next_poll_ms = next_poll;
                {
                    let mut status = state.status.lock().await;
                    status.state = if jobs.is_empty() {
                        "ready".into()
                    } else {
                        "printing".into()
                    };
                    status.last_message = if jobs.is_empty() {
                        Some("Sin impresiones pendientes".into())
                    } else {
                        Some(format!("Procesando {} impresión(es)", jobs.len()))
                    };
                }
                for job in jobs {
                    process_job(&client, &cfg, &state, job).await;
                }
            }
            Err(error) if error == "AUTH_REQUIRED" => {
                let mut status = state.status.lock().await;
                status.state = "setup_required".into();
                status.last_message = Some("La sucursal debe conectarse nuevamente".into());
                drop(status);
                next_poll_ms = 3_000;
            }
            Err(error) => {
                tracing::warn!("La conexión se restablecerá automáticamente: {}", error);
                let mut status = state.status.lock().await;
                status.state = "reconnecting".into();
                status.last_message = Some("Reconectando automáticamente".into());
                drop(status);
                next_poll_ms = 3_000;
            }
        }
    }

    sse_task.abort();
}

async fn sse_signal_loop(state: RuntimeState, tx: mpsc::Sender<()>) {
    while state.running.load(Ordering::SeqCst) {
        let cfg = match config::load() {
            Ok(cfg) if !cfg.branch_key.trim().is_empty() => cfg,
            Ok(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                continue;
            }
            Err(error) => {
                tracing::warn!("No se pudo preparar el canal en tiempo real: {}", error);
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                continue;
            }
        };

        match api::wait_for_sse_signal(&cfg).await {
            Ok(()) => {
                let _ = tx.try_send(());
            }
            Err(error) if error == "AUTH_REQUIRED" => {
                tracing::warn!("El canal en tiempo real requiere validar la sucursal nuevamente");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            Err(error) => {
                tracing::debug!("Canal en tiempo real no disponible: {}", error);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

async fn process_job(
    client: &reqwest::Client,
    cfg: &crate::models::AppConfig,
    state: &RuntimeState,
    job: crate::models::PrintJob,
) {
    {
        let recent = state.recent_done.lock().await;
        if recent.contains(&job.id) {
            return;
        }
    }
    {
        let mut inflight = state.inflight.lock().await;
        if !inflight.insert(job.id) {
            return;
        }
    }

    let result = process_job_inner(client, cfg, &job).await;
    match result {
        Ok(printer_name) => {
            if let Err(error) =
                api::mark_job(client, cfg, job.id, "done", Some(&printer_name), None).await
            {
                tracing::warn!(
                    "El trabajo {} se imprimió pero no pudo confirmarse: {}",
                    job.id,
                    error
                );
            }
            let mut recent = state.recent_done.lock().await;
            recent.insert(job.id);
            if recent.len() > 500 {
                recent.clear();
            }
            let mut status = state.status.lock().await;
            status.printed_today += 1;
            status.last_print_at = Some(chrono::Local::now().to_rfc3339());
            status.state = "ready".into();
            status.last_message = Some("Impresión completada".into());
        }
        Err(error) => {
            tracing::error!("Trabajo {} no completado: {}", job.id, error);
            let public_error = "La impresión requiere revisión";
            let _ = api::mark_job(client, cfg, job.id, "failed", None, Some(public_error)).await;
            let mut status = state.status.lock().await;
            status.state = "ready".into();
            status.last_message = Some(public_error.into());
        }
    }
    state.inflight.lock().await.remove(&job.id);
}

async fn process_job_inner(
    client: &reqwest::Client,
    cfg: &crate::models::AppConfig,
    job: &crate::models::PrintJob,
) -> Result<String, String> {
    let printer_id = job
        .printer_id
        .or_else(|| job.printer.as_ref().map(|p| p.id))
        .ok_or_else(|| "El trabajo no tiene un área de impresión".to_string())?;
    let id_key = printer_id.to_string();

    let mut printer_name = cfg
        .printer_mappings
        .get(&id_key)
        .cloned()
        .or_else(|| job.printer.as_ref().and_then(|p| p.printer_name.clone()))
        .or_else(|| job.printer.as_ref().and_then(|p| p.share_name.clone()))
        .ok_or_else(|| "El área no tiene una impresora asignada".to_string())?;

    let url = job
        .image_path
        .clone()
        .or_else(|| {
            job.image_filename.as_ref().map(|name| {
                format!(
                    "{}/user-uploads/print/{}",
                    cfg.domain_url.trim_end_matches('/'),
                    name
                )
            })
        })
        .ok_or_else(|| "El documento no está disponible".to_string())?;

    // Validate KOT multi-area filename convention is preserved (never rewrite server names)
    if let Some(name) = &job.image_filename {
        if name.starts_with("kot-") {
            tracing::info!("KOT multiárea: archivo {}", name);
        }
    }

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 25 * 1024 * 1024 {
        return Err("El documento supera el tamaño permitido".into());
    }
    let extension = url
        .split('?')
        .next()
        .and_then(|v| v.rsplit('.').next())
        .unwrap_or("png");
    let path: PathBuf = config::queue_dir()?.join(format!("restaapp-job-{}.{}", job.id, extension));
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| e.to_string())?;

    let print_path = path.clone();
    let format = job
        .printer
        .as_ref()
        .and_then(|p| p.print_format.clone());
    // A cash drawer is a payment-side effect, not a generic print side
    // effect. KOT jobs (kot-*) and browser pre-accounts must never open it,
    // even when the global or per-printer setting is enabled. The server
    // creates order-* (and split-*) jobs only for the customer account path.
    let is_customer_account_job = job
        .image_filename
        .as_deref()
        .map(|name| {
            let normalized = name.to_ascii_lowercase();
            normalized.starts_with("order-") || normalized.starts_with("split-")
        })
        .unwrap_or(false);
    let drawer_configured = cfg.open_drawer_after_print
        || job
            .printer
            .as_ref()
            .and_then(|p| p.open_cash_drawer)
            .unwrap_or(false);
    let open_drawer = is_customer_account_job && drawer_configured;
    if drawer_configured && !is_customer_account_job {
        tracing::info!(
            "Cajón omitido para trabajo no cobrable: {}",
            job.image_filename.as_deref().unwrap_or("sin-nombre")
        );
    }
    let copies = cfg.copies.max(1);
    let target = printer_name.clone();
    let format_clone = format.clone();

    let mut printed = tokio::task::spawn_blocking(move || {
        printers::print_file_with_options(
            &print_path,
            &target,
            copies,
            format_clone.as_deref(),
            open_drawer,
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    // Try backup mapping if primary fails
    if printed.is_err() {
        if let Some(backup) = cfg.backup_mappings.get(&id_key).cloned() {
            tracing::warn!("Usando impresora de respaldo para área {}", printer_id);
            let print_path2 = path.clone();
            let format2 = format.clone();
            printer_name = backup.clone();
            printed = tokio::task::spawn_blocking(move || {
                printers::print_file_with_options(
                    &print_path2,
                    &backup,
                    copies,
                    format2.as_deref(),
                    open_drawer,
                )
            })
            .await
            .map_err(|e| e.to_string())?;
        } else {
            // Last resort paint fallback for images
            let print_path3 = path.clone();
            let target3 = printer_name.clone();
            printed = tokio::task::spawn_blocking(move || {
                printers::print_file_fallback_paint(&print_path3, &target3)
            })
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    let _ = tokio::fs::remove_file(path).await;
    printed?;
    Ok(printer_name)
}
