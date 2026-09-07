import type { Locale } from "./i18n";

export interface AppConfig {
  schema_version: number;
  domain_url: string;
  branch_key: string;
  domain_locked: boolean;
  locale: Locale;
  autostart: boolean;
  open_drawer_after_print: boolean;
  poll_ms: number;
  idle_poll_ms: number;
  printer_mappings: Record<string, string>;
  backup_mappings: Record<string, string>;
  copies: number;
  app_version: string;
  device_id: string;
  restaurant_name?: string;
  branch_name?: string;
}

export interface AboutInfo {
  app_name: string;
  version: string;
  company: string;
  device_id: string;
  domain_url: string;
}

export interface ConnectionResult {
  ok: boolean;
  message: string;
  recommended_poll_ms?: number;
  recommended_idle_poll_ms?: number;
  branch_name?: string;
  restaurant_name?: string;
  logo_url?: string;
  locale?: string;
}

export interface ServerPrinter {
  id: number;
  name: string;
  printing_choice?: string;
  print_type?: string;
  print_format?: string;
  share_name?: string;
  printer_name?: string;
  type?: string;
  open_cash_drawer?: boolean;
}

export interface LocalPrinter {
  name: string;
  is_default: boolean;
  is_network: boolean;
  is_shared: boolean;
  available: boolean;
}

export interface RuntimeStatus {
  running: boolean;
  state: "ready" | "paused" | "reconnecting" | "setup_required" | "printing";
  printed_today: number;
  last_print_at?: string;
  last_message?: string;
}
