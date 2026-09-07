mod api;
mod config;
mod logging;
mod models;
mod polling;
mod printers;
mod secure;

use models::{AboutInfo, AppConfig, ConnectionResult, LocalPrinter, RuntimeStatus, ServerPrinter};
use polling::RuntimeState;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    config::load()
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<AppConfig, String> {
    config::save(&config)
}

#[tauri::command]
async fn test_connection(branch_key: String) -> Result<ConnectionResult, String> {
    api::test(&branch_key).await
}

#[tauri::command]
async fn get_server_printers(branch_key: String) -> Result<Vec<ServerPrinter>, String> {
    api::printers(&branch_key).await
}

#[tauri::command]
fn list_local_printers() -> Result<Vec<LocalPrinter>, String> {
    printers::list()
}

#[tauri::command]
fn print_test(printer_name: String, locale: String) -> Result<(), String> {
    printers::print_test(&printer_name, &locale)
}

#[tauri::command]
async fn start_print_service(state: tauri::State<'_, RuntimeState>) -> Result<RuntimeStatus, String> {
    polling::start(state.inner().clone()).await
}

#[tauri::command]
async fn stop_print_service(state: tauri::State<'_, RuntimeState>) -> Result<RuntimeStatus, String> {
    Ok(polling::stop(state.inner()).await)
}

#[tauri::command]
async fn get_runtime_status(state: tauri::State<'_, RuntimeState>) -> Result<RuntimeStatus, String> {
    Ok(polling::current(state.inner()).await)
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn open_logs(_app: tauri::AppHandle) -> Result<(), String> {
    let path = config::logs_dir()?;
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn export_diagnostics() -> Result<String, String> {
    let path = config::export_diagnostics()?;
    let _ = tauri_plugin_opener::open_path(
        std::path::Path::new(&path)
            .parent()
            .unwrap_or(std::path::Path::new(".")),
        None::<&str>,
    );
    Ok(path)
}

#[tauri::command]
fn get_about() -> Result<AboutInfo, String> {
    let cfg = config::load().unwrap_or_default();
    Ok(AboutInfo {
        app_name: "RestaAPP Printer".into(),
        version: models::APP_VERSION.into(),
        company: "GRUPOOHLA SRL".into(),
        device_id: cfg.device_id,
        domain_url: models::DOMAIN_URL.into(),
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().args(["--minimized"]).build())
        .plugin(tauri_plugin_opener::init())
        .manage(RuntimeState::default())
        .setup(|app| {
            logging::init().map_err(std::io::Error::other)?;
            let open =
                MenuItem::with_id(app, "open", "Abrir RestaAPP Printer", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pausar servicio", true, None::<&str>)?;
            let resume = MenuItem::with_id(app, "resume", "Reanudar servicio", true, None::<&str>)?;
            let test = MenuItem::with_id(app, "test", "Imprimir prueba", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &pause, &resume, &test, &sep, &quit])?;
            let mut tray = TrayIconBuilder::with_id("restaapp-printer")
                .menu(&menu)
                .tooltip("RestaAPP Printer — By RestaAPP");
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            let handle = app.handle().clone();
            tray.on_menu_event(move |app, event| match event.id.as_ref() {
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "pause" => {
                    let state = app.state::<RuntimeState>();
                    let state = state.inner().clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = polling::stop(&state).await;
                    });
                }
                "resume" => {
                    let state = app.state::<RuntimeState>();
                    let state = state.inner().clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = polling::start(state).await;
                    });
                }
                "test" => {
                    if let Ok(cfg) = config::load() {
                        if let Some((_, name)) = cfg.printer_mappings.iter().next() {
                            let name = name.clone();
                            let locale = cfg.locale.clone();
                            let _ = std::thread::spawn(move || {
                                let _ = printers::print_test(&name, &locale);
                            });
                        }
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .build(app)?;
            let _ = handle;

            // Solo ocultar al arrancar con Windows (--minimized). En uso normal la ventana permanece.
            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                // La X NO cierra el proceso: minimiza a la barra de tareas (sigue visible).
                // Para salir del todo: menú de bandeja → Salir.
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.minimize();
                    }
                });
            }

            // Auto-start print service if already configured and autostart
            if let Ok(cfg) = config::load() {
                if cfg.autostart && !cfg.branch_key.is_empty() && !cfg.printer_mappings.is_empty() {
                    let state = app.state::<RuntimeState>().inner().clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        let _ = polling::start(state).await;
                    });
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            test_connection,
            get_server_printers,
            list_local_printers,
            print_test,
            start_print_service,
            stop_print_service,
            get_runtime_status,
            set_autostart,
            open_logs,
            export_diagnostics,
            get_about
        ])
        .run(tauri::generate_context!())
        .expect("RestaAPP Printer no pudo iniciar");
}
