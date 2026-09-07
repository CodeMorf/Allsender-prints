# 05 — Análisis del .exe actual (v3.0.0)

**Archivo:** `C:\Users\grupo.SHIP24GO\Downloads\RestaPP_By_Allsender_Printer_3.0.0.exe`  
**Tamaño:** ~17.3 MB  
**Fecha vista:** 2026-07-18  

## Stack detectado (strings del binario)

| Evidencia | Conclusión |
|-----------|------------|
| `tauri-2.6.2`, `__tauriW`, `Tauri App` | **Tauri 2** |
| `src-tauri`, `config.json` | Proyecto Rust + frontend |
| `com.allsender.printer`, `allsender-printer` | Bundle / product name |
| `Allsender RESTAPP` | Branding UI |
| `reqwest`, HTTP/1.1 | Cliente HTTP Rust |
| `Get-WmiObject -Class Win32_Printer` | Lista impresoras vía PowerShell |
| `Get-Printer -Name` | PowerShell PrintManagement |
| `openDrawerAfterPrint`, `printerMappings` | Config local |
| `Status: Polling Active/Inactive` | Loop de polling |
| `X-TABLETRACK` (legado en ecosistema) | Compatible API TableTrack |
| `Printing KOT from JSON`, `ESC/POS`, `80mm` | Soporte raw térmico |
| `Check Updates`, `checksums`, `app_version` | Actualizaciones |
| `SOFTWARE\...\Run` | Autostart Windows |
| `http://localhost:1420` | Dev server Vite típico Tauri |
| Path build: `/Volumes/Data/Htdocs/codecanyon/tauri-app` | Origen CodeCanyon / TableTrack forkeado |

## Config local del agente (struct detectado)

```rust
// reconstruido desde strings
struct Config {
  domainUrl: String,           // https://restapp.allsender.tech
  key: String,                 // branches.unique_hash
  printerName: String,         // default local printer?
  printerMappings: Map,        // kitchen/server printer id → local name
  additionalPrinterMappings: Vec,
  openDrawerAfterPrint: bool,
  drawerPin: Option,
  deviceId: String,
  clientSyncKey: Option,
  // + labels, etc.
}
```

Persistencia típica Tauri: `%APPDATA%\com.allsender.printer\config.json`  
(o similar; confirmar al instalar).

## Comandos / features Rust expuestos (strings)

- `get_config` / `save_config`
- `check_printer_availability`
- `check_printer_sharing`
- `get_printer_troubleshooting_info`
- `save_printer_mappings`
- `get_app_identifier`
- `set_polling_status`
- `update_tray_status`

## Flujo de impresión en el .exe (inferido)

1. **Test Connection** → `GET {domain}/api/test-connection` + header key  
2. **Refresh Printers** → Win32_Printer + posiblemente `printer-details`  
3. **Save & Start Printing** → guarda mapping + inicia poll/SSE  
4. Por job:
   - descarga imagen/PDF o interpreta JSON KOT  
   - imprime con settings 56/80 mm  
   - opcional abre cajón  
   - marca done/failed  
5. Tray + Autostart + Check Updates  

## Limitaciones / riesgos del binario actual

- Origen **white-label CodeCanyon/TableTrack** → licencia / branding a depurar  
- Path de build de otro autor (`ajay`, `froid.works`)  
- API de updates puede apuntar a `envato.froid.works`  
- Header y rutas fijas al backend TableTrack-compatible (nuestro Laravel **sí** lo implementa)

## Qué reutilizar vs reescribir

| Reutilizar (idea/contrato) | Reescribir (recomendado) |
|----------------------------|---------------------------|
| Contrato API (documentado) | UI Allsender propia |
| Mapping cocina→impresora | Branding, iconos, about |
| Poll + SSE + tray | Canal de updates **nuestro** |
| ESC/POS 80mm tricks | Código fuente limpio con licencia clara |
| Autostart | Versiones semver + changelog |

**No necesitas reverse-engineer completo del .exe** si implementas el contrato de `04-api-contrato-desktop.md`.
