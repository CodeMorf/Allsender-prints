# RestApp Desktop Printer — Documentación técnica

**Fecha:** 2026-07-18  
**Sistema:** restapp.allsender.tech (Laravel 12 + Livewire)  
**Cliente actual:** `RestaPP_By_Allsender_Printer_3.0.0.exe` (~17 MB)  
**Stack del .exe actual:** **Tauri 2.6.2** (Rust + WebView) — *no Electron*

---

## Índice de documentos

| Archivo | Contenido |
|---------|-----------|
| [01-panorama-ajustes.md](./01-panorama-ajustes.md) | Qué hace `/settings?tab=kot`, `printer` y la conexión de escritorio |
| [02-arquitectura-impresion.md](./02-arquitectura-impresion.md) | Flujo end-to-end: POS → print_jobs → agente desktop → impresora |
| [03-sql-y-modelos.md](./03-sql-y-modelos.md) | Tablas, columnas, relaciones |
| [04-api-contrato-desktop.md](./04-api-contrato-desktop.md) | Endpoints, headers, SSE, polling, estados |
| [05-exe-actual-tauri.md](./05-exe-actual-tauri.md) | Análisis del .exe 3.0.0 y config interna |
| [06-como-crear-desde-cero.md](./06-como-crear-desde-cero.md) | Plan para crear **nuestro** agente con versiones, sin romper el backend |
| [07-checklist-implementacion.md](./07-checklist-implementacion.md) | Fases para empezar a programar |

---

## Resumen ejecutivo (1 página)

### Problema que resuelve el .exe
Los navegadores **no pueden** enviar ESC/POS ni hablar con impresoras térmicas compartidas de Windows de forma fiable.  
Por eso RestApp genera una **imagen/PDF** del ticket (KOT o recibo), crea un **job** en MySQL y el **agente de escritorio** (en la PC del restaurante) lo imprime en segundo plano.

### Credenciales que ve el cliente (Ajustes → Impresora)

| Campo en UI | Valor real en BD | Uso |
|-------------|------------------|-----|
| **URL de dominio** | `https://restapp.allsender.tech` | Base de la API |
| **Clave secreta / Branch key** | `branches.unique_hash` (20 chars hex) | Header `X-TABLETRACK-KEY` |
| Restablecer clave | `Branch::generateUniqueHash()` | Invalida agentes viejos |

> **Idea tuya “solo la key”:** en una versión white-label Allsender puedes **quemar** el dominio `https://restapp.allsender.tech` en el .exe y pedir **solo la Branch Key**. El backend actual **ya soporta eso** (el dominio solo se usa como base URL del cliente; la auth es 100% la key).

### No romper lo actual
- **No renombres** el header `X-TABLETRACK-KEY`.
- **No cambies** el significado de `branches.unique_hash`.
- Mantén los endpoints de `routes/api.php` (pull + SSE + mark done).
- El agente nuevo puede coexistir: dos .exe distintos pueden usar la **misma API**.

### Stack recomendado para el nuevo .exe
**Tauri 2 + Rust** (igual que el actual) o **.NET 8 Worker + WPF/WinUI** si prefieres C#.  
Ambos pueden: tray icon, autostart, polling/SSE, listar impresoras Windows, imprimir imagen/PDF/raw.

---

## Mapa rápido de código servidor

```
routes/api.php
  GET  /api/print-stream/{token}          → SSE (token = unique_hash)
  middleware DesktopUniqueKeyMiddleware:
    GET  /api/test-connection
    GET  /api/print-jobs/pull-multiple
    GET  /api/printer-details
    PATCH /api/print-jobs/{id}
    POST  /api/print-jobs/{id}/printed

app/Http/Middleware/DesktopUniqueKeyMiddleware.php  → Branch by unique_hash
app/Http/Controllers/PrintJobController.php
app/Http/Controllers/PrintStreamController.php
app/Traits/PrinterSetting.php                       → crea jobs KOT/Order
app/Models/PrintJob.php, Printer.php, Branch.php
app/Events/PrintJobCreated.php                      → Pusher broadcast
app/Livewire/Settings/PrinterSetting.php            → UI key + impresoras
app/Livewire/Settings/KotAreasSettings.php          → tab=kot (áreas)
app/Livewire/Settings/KotSettings.php               → tab=kotSettings
```

---

## Versión del producto Allsender

Propuesta de versionado del cliente desktop:

| Campo | Ejemplo |
|-------|---------|
| Product name | `Allsender RestApp Printer` |
| Bundle ID | `tech.allsender.restapp.printer` |
| SemVer | `3.1.0` (compatible con API actual) |
| Canal | `stable` / `beta` |
| Update feed | JSON en restapp (`/api/desktop-agent/updates`) *a implementar* |

El .exe actual ya tiene strings de **Check Updates**, `app_version`, `checksums`, `windows`/`macos` downloads — se puede reutilizar el mismo esquema.
