# 06 — Cómo crear el software .exe desde cero (sin romper el backend)

## Objetivo

Un **Allsender RestApp Printer** propio, versionado, con:

- Segundo plano (system tray)
- Polling y/o SSE
- Mapeo impresoras Windows
- Solo Branch Key (URL fija) o Domain+Key
- Auto-update controlado por nosotros
- **100% compatible** con la API actual de restapp.allsender.tech

---

## Opción de stack (elige una)

### Opción A — Tauri 2 + React/Svelte (recomendado, igual al actual)

**Pros:** binario chico (~15–20 MB), Rust nativo print/spooler, UI web moderna.  
**Contras:** curva Rust para impresión avanzada.

Estructura:
```
allsender-printer/
  src/                 # UI (React)
  src-tauri/
    src/main.rs
    src/commands/
      config.rs
      printers.rs
      poll.rs
      print.rs
    tauri.conf.json
  package.json
```

### Opción B — .NET 8 Worker Service + WPF tray

**Pros:** excelente integración Windows Print, fácil para devs C#.  
**Contras:** instalador más pesado.

### Opción C — Electron + node-printer

**Pros:** JS only.  
**Contras:** 100+ MB, más frágil con térmicas; no recomendado.

---

## Principios “no romper”

1. **Header exacto:** `X-TABLETRACK-KEY`  
2. **Key = `branches.unique_hash`** (no inventar otra tabla de tokens sin migración dual)  
3. **No exigir cambios de API** en el primer release  
4. Cualquier API nueva (`/api/v2/desktop/...`) debe ser **aditiva**  
5. Jobs: respetar estados `pending|printing|done|failed`  
6. No borrar ni renombrar archivos en `user-uploads/print/` desde el agente  

---

## Arquitectura del nuevo agente

```
┌─────────────────────────────────────────────┐
│ UI (config)                                 │
│  - Domain (opcional, default Allsender)     │
│  - Branch Key                               │
│  - Mapeo impresoras                         │
│  - Autostart / Cajón / Versión              │
└─────────────────┬───────────────────────────┘
                  │ save config.json
┌─────────────────▼───────────────────────────┐
│ Background Service (siempre vivo)           │
│  1. test-connection loop                    │
│  2. pull-multiple OR SSE                    │
│  3. print queue local (dedupe job ids)      │
│  4. report status                           │
│  5. tray icon state                         │
└─────────────────────────────────────────────┘
```

---

## Config propuesta (nuestro)

```json
{
  "schema_version": 1,
  "domain_url": "https://restapp.allsender.tech",
  "branch_key": "",
  "domain_locked": true,
  "poll_ms": 4000,
  "idle_poll_ms": 10000,
  "use_sse": true,
  "open_drawer_after_print": false,
  "autostart": true,
  "printer_mappings": {
    "14": "EPSON TM-T20II",
    "15": "Kitchen_80mm"
  },
  "app_version": "3.1.0",
  "device_id": "uuid-generado-una-vez"
}
```

`domain_locked: true` → UI no muestra URL (solo Key).  
Super Admin / multi-tenant white-label → `domain_locked: false`.

---

## Implementación por módulos

### M1 — Config + Test Connection
- Guardar key
- `GET /api/test-connection`
- Mostrar Connected / Invalid key

### M2 — Listar impresoras Windows
- PowerShell `Get-WmiObject Win32_Printer` o Win32 API
- Mostrar nombre + estado shared

### M3 — Printer details del server
- `GET /api/printer-details`
- UI: cada printer del branch → dropdown impresora local

### M4 — Motor de jobs
- Poll `pull-multiple`
- (Opcional) SSE `/api/print-stream/{key}`
- Dedupe `HashSet<jobId>`
- Download image
- Print:
  - Windows Spooler (imagen)
  - O ESC/POS si `print_format` thermal*

### M5 — Feedback
- PATCH done/failed
- Logs locales rotativos
- Tray: “3 jobs today / last error”

### M6 — Versiones y update
- Endpoint **nuestro** (futuro):  
  `GET https://restapp.allsender.tech/api/desktop-agent/latest?channel=stable&os=windows`
- JSON:
```json
{
  "version": "3.1.1",
  "url": "https://cdn.allsender.tech/printer/AllsenderPrinter_3.1.1_x64.msi",
  "sha256": "...",
  "mandatory": false,
  "notes": "Fix KOT multi-area mapping"
}
```
- UI: badge v3.1.0 + Check Updates (como captura)

### M7 — Instalador
- MSI / NSIS / Tauri bundle
- Firma de código (opcional pero ideal)
- Autostart opcional

---

## Scaffold Tauri mínimo (inicio de proyecto)

```bash
# Requisitos: Node 20+, Rust stable, VS Build Tools (Windows)
npm create tauri-app@latest allsender-printer
cd allsender-printer
# framework: React + TypeScript
```

**Comando Rust ejemplo (poll):**
```rust
// pseudocódigo
let client = reqwest::Client::new();
let res = client
  .get(format!("{}/api/print-jobs/pull-multiple", domain))
  .header("X-TABLETRACK-KEY", &key)
  .send()
  .await?;
let jobs: Vec<PrintJob> = res.json().await?;
for job in jobs {
  let bytes = client.get(&job.image_path).send().await?.bytes().await?;
  print_image(&mapping[&job.printer_id], &bytes)?;
  client.patch(format!("{}/api/print-jobs/{}", domain, job.id))
    .header("X-TABLETRACK-KEY", &key)
    .json(&json!({"status":"done","printer": local_name}))
    .send().await?;
}
```

---

## Pruebas de integración (sin tocar prod a ciegas)

1. Branch de prueba con `printing_choice = directPrint`  
2. Copiar Branch Key desde Ajustes → Impresora  
3. Agente local → Test Connection  
4. Crear KOT en POS → debe aparecer job `pending`  
5. Agente lo imprime → `done`  
6. Reset Branch Key → agente debe fallar 401 hasta reconfigurar  

SQL útil:
```sql
SELECT id, status, image_filename, printer_id, branch_id, created_at
FROM print_jobs
ORDER BY id DESC LIMIT 20;

SELECT id, name, unique_hash, restaurant_id FROM branches WHERE id = ?;
SELECT id, name, printing_choice, share_name, branch_id FROM printers WHERE branch_id = ?;
```

---

## Multi-versión / coexistencia

| Cliente | API | Notas |
|---------|-----|-------|
| RestaPP 3.0.0 (actual) | v1 actual | Sigue funcionando |
| Allsender Printer 3.1+ | v1 actual | Mismo backend |
| Futuro 4.0 | v1 + v2 optional | Features nuevas opt-in |

Nunca rompas v1 hasta deprecar con 90 días de aviso.

---

## Seguridad

- Branch Key es **secreto de sucursal** (como API token)  
- No loguear la key en claro  
- HTTPS only  
- Al resetear key, forzar re-pair del .exe  
- Considerar rate-limit en middleware (futuro)  
- No exponer listado de unique_hash en APIs públicas  

---

## Qué NO hacer

- ❌ Compilar la URL de un solo restaurante y olvidar multi-tenant  
- ❌ Usar `restaurant_id` como key (es por **branch**)  
- ❌ Asumir un solo kot place (multi-área)  
- ❌ Borrar `print_jobs` desde el cliente  
- ❌ Depender de Pusher sin fallback poll  
- ❌ Reverse-engineering ilegal del .exe de terceros para redistribuir su código  

---

## Entregables de un MVP (2–3 semanas dev enfocado)

1. UI: Key + Test + Mapeo + Start  
2. Poll loop en background  
3. Imprimir PNG a impresora Windows  
4. PATCH done/failed  
5. Tray + autostart  
6. Instalador versionado `3.1.0`  
7. Doc de instalación para el restaurante (1 página)  
