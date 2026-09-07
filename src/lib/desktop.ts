import { invoke } from "@tauri-apps/api/core";
import type { AboutInfo, AppConfig, ConnectionResult, LocalPrinter, RuntimeStatus, ServerPrinter } from "../types";

export const desktop = {
  loadConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<AppConfig>("save_config", { config }),
  testConnection: (branchKey: string) => invoke<ConnectionResult>("test_connection", { branchKey }),
  serverPrinters: (branchKey: string) => invoke<ServerPrinter[]>("get_server_printers", { branchKey }),
  localPrinters: () => invoke<LocalPrinter[]>("list_local_printers"),
  printTest: (printerName: string, locale: string) => invoke<void>("print_test", { printerName, locale }),
  start: () => invoke<RuntimeStatus>("start_print_service"),
  stop: () => invoke<RuntimeStatus>("stop_print_service"),
  status: () => invoke<RuntimeStatus>("get_runtime_status"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  openLogs: () => invoke<void>("open_logs"),
  exportDiagnostics: () => invoke<string>("export_diagnostics"),
  about: () => invoke<AboutInfo>("get_about")
};
