use crate::models::{ConnectionResult, PrintJob, ServerPrinter};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::time::Duration;

fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(format!("RestaAPP-Printer/{}", crate::models::APP_VERSION))
        .build()
        .map_err(|e| e.to_string())
}

pub async fn test(branch_key: &str) -> Result<ConnectionResult, String> {
    let cfg = crate::config::load()?;
    let response = client()?
        .get(format!("{}/api/test-connection", cfg.domain_url.trim_end_matches('/')))
        .header("X-TABLETRACK-KEY", branch_key.trim())
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("Fallo de red en test-connection: {}", e);
            "No fue posible validar la conexión".to_string()
        })?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Ok(ConnectionResult {
            ok: false,
            message: "La clave no pudo ser validada".into(),
            recommended_poll_ms: None,
            recommended_idle_poll_ms: None,
            branch_name: None,
            restaurant_name: None,
            logo_url: None,
            locale: None,
        });
    }

    if !response.status().is_success() {
        tracing::warn!("test-connection respondió {}", response.status());
        return Ok(ConnectionResult {
            ok: false,
            message: "La conexión no está disponible temporalmente".into(),
            recommended_poll_ms: None,
            recommended_idle_poll_ms: None,
            branch_name: None,
            restaurant_name: None,
            logo_url: None,
            locale: None,
        });
    }

    let value: Value = response.json().await.map_err(|e| e.to_string())?;
    Ok(ConnectionResult {
        ok: value.get("status").and_then(Value::as_str).map(|s| s == "success").unwrap_or(true),
        message: value.get("message").and_then(Value::as_str).unwrap_or("Connection established").into(),
        recommended_poll_ms: value.get("recommended_poll_ms").and_then(Value::as_u64),
        recommended_idle_poll_ms: value.get("recommended_idle_poll_ms").and_then(Value::as_u64),
        branch_name: value.get("branch_name").and_then(Value::as_str).map(String::from),
        restaurant_name: value.get("restaurant_name").and_then(Value::as_str).map(String::from),
        logo_url: value.get("logo_url").and_then(Value::as_str).map(String::from),
        locale: value.get("locale").or_else(|| value.get("language")).and_then(Value::as_str).map(String::from),
    })
}

pub async fn printers(branch_key: &str) -> Result<Vec<ServerPrinter>, String> {
    let cfg = crate::config::load()?;
    let response = client()?
        .get(format!("{}/api/printer-details", cfg.domain_url.trim_end_matches('/')))
        .header("X-TABLETRACK-KEY", branch_key.trim())
        .send().await.map_err(|e| e.to_string())?;
    if response.status() == StatusCode::UNAUTHORIZED {
        return Err("La clave de la sucursal debe verificarse nuevamente".into());
    }
    response.error_for_status().map_err(|e| e.to_string())?
        .json::<Vec<ServerPrinter>>().await.map_err(|e| e.to_string())
}

pub async fn pull_jobs(client: &Client, config: &crate::models::AppConfig) -> Result<(Vec<PrintJob>, u64), String> {
    let response = client
        .get(format!("{}/api/print-jobs/pull-multiple", config.domain_url.trim_end_matches('/')))
        .header("X-TABLETRACK-KEY", config.branch_key.trim())
        .send().await.map_err(|e| e.to_string())?;
    if response.status() == StatusCode::UNAUTHORIZED {
        return Err("AUTH_REQUIRED".into());
    }
    let poll_ms = response.headers().get("X-Print-Poll-Ms")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(config.idle_poll_ms);
    let jobs = response.error_for_status().map_err(|e| e.to_string())?
        .json::<Vec<PrintJob>>().await.map_err(|e| e.to_string())?;
    Ok((jobs, poll_ms))
}

/// Wait for a server-sent print signal. The stream only wakes the worker;
/// pull_jobs remains the single owner that atomically claims pending jobs.
/// This preserves the legacy REST contract and prevents SSE/poll duplicates.
pub async fn wait_for_sse_signal(config: &crate::models::AppConfig) -> Result<(), String> {
    let stream_client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        // The stream is kept alive by the server heartbeat; reconnect hourly
        // to avoid keeping a single HTTP connection indefinitely.
        .timeout(Duration::from_secs(60 * 60))
        .user_agent(format!("RestaAPP-Printer/{}/sse", crate::models::APP_VERSION))
        .build()
        .map_err(|e| e.to_string())?;
    let response = stream_client
        .get(format!(
            "{}/api/print-stream/{}",
            config.domain_url.trim_end_matches('/'),
            config.branch_key.trim()
        ))
        .header("Accept", "text/event-stream")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::FORBIDDEN {
        return Err("AUTH_REQUIRED".into());
    }
    let mut response = response.error_for_status().map_err(|e| e.to_string())?;

    let mut buffer: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        buffer.extend_from_slice(&chunk);
        while let Some(position) = buffer.windows(2).position(|pair| pair == b"\n\n") {
            let event = String::from_utf8_lossy(&buffer[..position]).into_owned();
            buffer.drain(..position + 2);
            if event.lines().any(|line| line.trim() == "event: print") {
                return Ok(());
            }
        }
        // Bound malformed responses so a bad server cannot grow the agent indefinitely.
        if buffer.len() > 64 * 1024 {
            buffer.clear();
        }
    }
    Err("El canal en tiempo real se desconectó".into())
}

pub async fn mark_job(client: &Client, config: &crate::models::AppConfig, job_id: i64, status: &str, printer: Option<&str>, error: Option<&str>) -> Result<(), String> {
    let body = serde_json::json!({
        "status": status,
        "printed_at": if status == "done" { Some(chrono::Utc::now().to_rfc3339()) } else { None },
        "printer": printer,
        "error": error
    });
    client
        .patch(format!("{}/api/print-jobs/{}", config.domain_url.trim_end_matches('/'), job_id))
        .header("X-TABLETRACK-KEY", config.branch_key.trim())
        .json(&body).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn background_client() -> Result<Client, String> { client() }
