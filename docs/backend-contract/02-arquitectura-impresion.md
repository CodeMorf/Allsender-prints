# 02 — Arquitectura de impresión (end-to-end)

## Diagrama de flujo

```
┌─────────────────────┐     1. Usuario imprime KOT/recibo
│  Panel web / POS    │─────────────────────────────────────┐
│  (Livewire/PHP)     │                                     │
└─────────────────────┘                                     ▼
                                              ┌──────────────────────────┐
                                              │ Trait PrinterSetting     │
                                              │ handleKotPrint / Order   │
                                              └────────────┬─────────────┘
                     2. HTML del ticket                     │
                        (KotController::printKot)           │
                                                           ▼
                                              ┌──────────────────────────┐
                     3a. Image path           │ PNG o PDF en disco       │
                         user-uploads/print/  │ kot-{id}-{place}.png     │
                                              └────────────┬─────────────┘
                                                           │
                     4. PrintJob::create status=pending     ▼
                                              ┌──────────────────────────┐
                                              │ print_jobs               │
                                              │ + event PrintJobCreated  │
                                              └────────────┬─────────────┘
                         ┌─────────────────────────────────┼────────────────────────────┐
                         ▼                                 ▼                            ▼
              ┌──────────────────┐             ┌──────────────────┐          ┌──────────────────┐
              │ Pusher broadcast │             │ SSE stream       │          │ Polling REST     │
              │ print-job.created│             │ /api/print-stream│          │ pull-multiple    │
              └────────┬─────────┘             └────────┬─────────┘          └────────┬─────────┘
                       └───────────────────┬────────────┴─────────────────────────────┘
                                           ▼
                              ┌────────────────────────────┐
                              │  Agente desktop (.exe)     │
                              │  PC del restaurante        │
                              │  - tray / background       │
                              │  - mapeo impresoras        │
                              │  - descarga image_path     │
                              │  - imprime / cajón         │
                              └────────────┬───────────────┘
                                           │ 5. PATCH status done|failed
                                           │    o POST .../printed
                                           ▼
                              ┌────────────────────────────┐
                              │ print_jobs.status = done   │
                              └────────────────────────────┘
```

## Canales de entrega de jobs al .exe

El backend ofrece **tres** mecanismos (el agente puede usar uno o combinarlos):

### A) Smart polling (siempre disponible)
```
GET /api/print-jobs/pull-multiple
Header: X-TABLETRACK-KEY: {unique_hash}
```
- Devuelve jobs `pending` (máx 20), cambia a `printing`, incluye `printer` relation.
- Headers de hint: `X-Print-Poll-Ms`, `X-Print-Empty`
- Idle: poll ~8–10 s; con trabajo: ~2 s  
  (recomendaciones también en `test-connection`)

### B) SSE (Server-Sent Events)
```
GET /api/print-stream/{token}
token = unique_hash (misma key; EventSource no manda headers)
```
- Stream infinito `text/event-stream`
- Eventos `print` con JSON: id, content (URL imagen), type, printer{}
- Heartbeat cada 25 s

### C) Pusher (opcional)
Si `pusherSettings()->is_enabled_pusher_broadcast`:
- Canales: `print-jobs` + `print-jobs.branch.{branchId}`
- Evento: `print-job.created`
- Config se devuelve en `test-connection`

## Flujo KOT multi-área (CRÍTICO — no romper)

En `app/Traits/PrinterSetting.php`:

1. `handleKotPrint($kotId, $kotPlaceId, $alsoPrintOrder)`
2. Resuelve `KotPlace` → `printer_id` → `Printer`
3. Genera HTML con `KotController::printKot`
4. Genera PNG (`kot-{kotId}-{kotPlaceId}.png`) o PDF
5. `createPrintJobRecord` → `PrintJob` + `PrintJobCreated`

> Comentario en código: **“FLUJO CRITICO KOT MULTI-AREA - NO TOCAR SIN DIAGNOSTICO.”**  
> El nombre de archivo **debe** incluir `kotPlaceId`.

## Imagen pública

```
PrintJob.image_path  (appended)
= asset( user-uploads/print/{image_filename} )
= https://restapp.allsender.tech/user-uploads/print/kot-595-11.png
```

El agente **descarga esa URL** y la manda a la impresora Windows (o convierte a ESC/POS para 80mm).

## Autenticación del agente

```
X-TABLETRACK-KEY: <branches.unique_hash>
```

Middleware: `DesktopUniqueKeyMiddleware`  
- Busca `Branch::where('unique_hash', $key)`  
- Inyecta `$request['branch']`  
- Sin key → 401 (excepto env `development`, que usa la primera branch)

## Qué NO hace el backend por el cliente

- No lista impresoras del PC del cliente  
- No envía ESC/POS crudo por defecto (envía **imagen/PDF**)  
- No abre el cajón: el .exe interpreta `open_cash_drawer` / su config local  

El .exe actual **sí** tiene lógica ESC/POS 80mm y “Open Drawer After Print” en el lado cliente.
