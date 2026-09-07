# 07 — Checklist de implementación (cuando empieces a crear)

## Fase 0 — Preparación (1 día)
- [ ] Crear repo `allsender-restapp-printer`
- [ ] Elegir stack (Tauri 2 recomendado)
- [ ] Branch de prueba en restapp + impresora `directPrint`
- [ ] Guardar Branch Key de prueba
- [ ] Postman/Insomnia colección con los 5 endpoints

## Fase 1 — API client
- [ ] `testConnection(domain, key)`
- [ ] `printerDetails(domain, key)`
- [ ] `pullMultiple(domain, key)`
- [ ] `updateJob(domain, key, id, status, error?, printer?)`
- [ ] Manejo 401 con mensaje “clave inválida o restablecida”

## Fase 2 — Windows printers
- [ ] Listar impresoras instaladas
- [ ] Detectar si están compartidas (opcional warning)
- [ ] Persist mapping `server_printer_id → local_name`

## Fase 3 — Print engine
- [ ] Descargar PNG/PDF
- [ ] Imprimir imagen a impresora nombrada
- [ ] (Plus) ESC/POS 80mm
- [ ] (Plus) Open cash drawer

## Fase 4 — Background
- [ ] Tray icon (Connected / Idle / Error)
- [ ] Loop poll con backoff `X-Print-Poll-Ms`
- [ ] (Plus) SSE
- [ ] Autostart
- [ ] Dedupe jobs

## Fase 5 — Producto Allsender
- [ ] Branding icon/splash “Allsender RestApp Printer”
- [ ] Domain locked por defecto
- [ ] About: versión, build, device_id
- [ ] Logs en `%APPDATA%\AllsenderRestAppPrinter\logs\`
- [ ] Instalador MSI/NSIS versionado

## Fase 6 — Updates (opcional pero recomendado)
- [ ] Endpoint Laravel `DesktopAgentUpdateController`
- [ ] JSON latest + sha256
- [ ] Botón Check Updates en UI

## Fase 7 — QA en local real
- [ ] KOT 1 área
- [ ] KOT multi-área (2+ kot_places)
- [ ] Recibo de orden
- [ ] Reset branch key → re-pair
- [ ] PC sin red → reintentos
- [ ] Impresora apagada → status failed con error claro

## Fase 8 — Despliegue a clientes
- [ ] Subir instalador a Drive/CDN
- [ ] Actualizar `desktop_applications.windows_file_path`
- [ ] Guía corta en español para el restaurante
- [ ] Mantener .exe 3.0.0 como fallback temporal

---

## Comandos útiles servidor (diagnóstico)

```bash
# Últimos jobs
php artisan tinker
>>> App\Models\PrintJob::latest()->take(5)->get(['id','status','image_filename','branch_id','printer_id'])

# Key de una branch
>>> App\Models\Branch::find(9)->unique_hash

# Impresoras direct print
>>> App\Models\Printer::where('printing_choice','directPrint')->get(['id','name','branch_id','share_name'])
```

## Archivos servidor que NO debes tocar al azar

- `app/Traits/PrinterSetting.php` (flujo KOT multi-área)
- `DesktopUniqueKeyMiddleware.php` (auth)
- Nombre header `X-TABLETRACK-KEY`
- Convención `kot-{kotId}-{kotPlaceId}.png`

---

## Próximo paso concreto

Cuando digas **“empezamos a crear el .exe”**, el primer commit debería ser:

1. Scaffold Tauri “Allsender RestApp Printer 3.1.0”  
2. Pantalla Domain (fijo) + Key + Test Connection  
3. Poll cada 5s imprimiendo a “Microsoft Print to PDF” (sin térmica)  

Eso valida el 80% del contrato **sin** hardware térmico.
