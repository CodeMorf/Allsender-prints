use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DOMAIN_URL: &str = "https://restapp.allsender.tech";
pub const APP_VERSION: &str = "3.1.2";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub schema_version: u32,
    pub domain_url: String,
    pub branch_key: String,
    pub domain_locked: bool,
    pub locale: String,
    pub autostart: bool,
    pub open_drawer_after_print: bool,
    pub poll_ms: u64,
    pub idle_poll_ms: u64,
    /// server printer.id -> Windows printer name
    pub printer_mappings: HashMap<String, String>,
    /// server printer.id -> fallback Windows printer name
    pub backup_mappings: HashMap<String, String>,
    pub copies: u32,
    pub app_version: String,
    pub device_id: String,
    pub restaurant_name: Option<String>,
    pub branch_name: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            domain_url: DOMAIN_URL.into(),
            branch_key: String::new(),
            domain_locked: true,
            locale: "es".into(),
            autostart: true,
            open_drawer_after_print: false,
            poll_ms: 4_000,
            idle_poll_ms: 10_000,
            printer_mappings: HashMap::new(),
            backup_mappings: HashMap::new(),
            copies: 1,
            app_version: APP_VERSION.into(),
            device_id: uuid::Uuid::new_v4().to_string(),
            restaurant_name: None,
            branch_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionResult {
    pub ok: bool,
    pub message: String,
    pub recommended_poll_ms: Option<u64>,
    pub recommended_idle_poll_ms: Option<u64>,
    pub branch_name: Option<String>,
    pub restaurant_name: Option<String>,
    pub logo_url: Option<String>,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPrinter {
    pub id: i64,
    pub name: String,
    pub printing_choice: Option<String>,
    pub print_type: Option<String>,
    pub print_format: Option<String>,
    pub share_name: Option<String>,
    pub printer_name: Option<String>,
    #[serde(rename = "type")]
    pub printer_type: Option<String>,
    pub open_cash_drawer: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPrinter {
    pub name: String,
    pub is_default: bool,
    pub is_network: bool,
    pub is_shared: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJob {
    pub id: i64,
    pub branch_id: Option<i64>,
    pub restaurant_id: Option<i64>,
    pub printer_id: Option<i64>,
    pub status: Option<String>,
    pub image_filename: Option<String>,
    pub image_path: Option<String>,
    pub printer: Option<ServerPrinter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub running: bool,
    pub state: String,
    pub printed_today: u64,
    pub last_print_at: Option<String>,
    pub last_message: Option<String>,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            running: false,
            state: "paused".into(),
            printed_today: 0,
            last_print_at: None,
            last_message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AboutInfo {
    pub app_name: String,
    pub version: String,
    pub company: String,
    pub device_id: String,
    pub domain_url: String,
}
