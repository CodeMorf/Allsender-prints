# 04 — Contrato API del agente desktop

**Base URL (cliente Allsender):** `https://restapp.allsender.tech`  
**Prefijo API Laravel:** normalmente `/api/...`  
**Auth:** header `X-TABLETRACK-KEY: {branches.unique_hash}`

> Compatibilidad: el nombre histórico del header es `X-TABLETRACK-KEY` (TableTrack origin).  
> **No lo renombres** si quieres que el .exe 3.0.0 y el tuyo nuevo compartan backend.

---

## 1. Test Connection

```http
GET /api/test-connection
X-TABLETRACK-KEY: {unique_hash}
```

**200 OK**
```json
{
  "message": "Connection established",
  "status": "success",
  "pusher_enabled": false,
  "recommended_poll_ms": 4000,
  "recommended_idle_poll_ms": 10000,
  "pusher_config": { ... }   // solo si pusher_enabled
}
```

**401**
```json
{ "message": "No authentication key found" }
// o
{ "message": "Invalid authentication key" }
```

Usar al pulsar **Test Connection** en la UI del .exe.

---

## 2. Detalle de impresoras del branch

```http
GET /api/printer-details
X-TABLETRACK-KEY: {unique_hash}
```

**200** — array de modelos `Printer` del branch (id, name, printing_choice, print_format, share_name, type, open_cash_drawer, kots, orders…).

El agente usa esto para:
- Mostrar qué “cocinas” existen en el servidor
- Mapear `printer.id` / `name` → impresora local Windows

---

## 3. Pull de trabajos (polling)

```http
GET /api/print-jobs/pull-multiple
X-TABLETRACK-KEY: {unique_hash}
```

**Respuesta vacía (idle)**
```json
[]
```
Headers: `X-Print-Empty: 1`, `X-Print-Poll-Ms: 8000`

**Con trabajos**
```json
[
  {
    "id": 1,
    "branch_id": 9,
    "restaurant_id": 8,
    "printer_id": 14,
    "status": "printing",
    "image_filename": "kot-595-11.png",
    "created_at": "...",
    "updated_at": "...",
    "printer": {
      "id": 14,
      "name": "...",
      "printing_choice": "directPrint",
      "print_format": "thermal80mm",
      "share_name": "Kitchen80",
      "type": "kitchen"
    },
    "image_path": "https://restapp.allsender.tech/user-uploads/print/kot-595-11.png"
  }
]
```

**Comportamiento servidor:**
1. Cuenta pending (cache 2s)
2. Si hay, selecciona hasta 20
3. Verifica que el archivo exista en `public/user-uploads/print/`
4. Marca `status = printing`
5. Devuelve JSON

**Agente debe:**
1. Descargar `image_path` (o construir URL con filename)
2. Imprimir en la impresora mapeada
3. Reportar resultado (abajo)

---

## 4. Marcar impreso / fallido

### Opción A — PATCH genérico
```http
PATCH /api/print-jobs/{id}
X-TABLETRACK-KEY: {unique_hash}
Content-Type: application/json

{
  "status": "done",          // o "failed"
  "printed_at": "2026-07-18T20:30:00Z",
  "error": null,
  "printer": "EPSON TM-T20"  // nombre Windows
}
```

### Opción B — POST companion SSE
```http
POST /api/print-jobs/{id}/printed
X-TABLETRACK-KEY: {unique_hash}
```
Marca `done` + `printed_at = now()` (valida que el job sea del mismo branch).

---

## 5. SSE (push sin header)

```http
GET /api/print-stream/{unique_hash}
Accept: text/event-stream
```

**Evento ejemplo:**
```
id: 1
event: print
data: {"id":1,"content":"https://.../kot-595-11.png","type":"kitchen","copies":1,"printer":{...}}

: heartbeat
```

Notas:
- Token en la **URL** (EventSource no envía `X-TABLETRACK-KEY`)
- Mismo valor que la Branch Key
- No marca automáticamente a `printing` igual que pull (el agente debe ser cuidadoso con duplicados si usa SSE + poll a la vez)

---

## 6. Estrategia recomendada del agente (sin romper nada)

```
1. Guardar config: domainUrl + key (+ version)
2. test-connection → OK
3. printer-details → construir UI de mapeo
4. Iniciar:
   a) Preferir SSE si la red lo permite
   b) Fallback poll-multiple con backoff (headers X-Print-Poll-Ms)
   c) Opcional: Pusher si pusher_enabled
5. Por cada job:
   - resolver impresora Windows (mapping[printer.id] || share_name || printer_name)
   - descargar content/image_path
   - imprimir (GDI / WinSpool / ESC-POS)
   - PATCH done|failed
6. Tray: Connected / Polling Active / Disconnected
7. Autostart Windows (Run key) opcional
```

---

## 7. Solo KEY (sin URL en UI del cliente)

| Opción | Implementación |
|--------|----------------|
| **A. Dominio fijo** | .exe Allsender con `DOMAIN_URL=https://restapp.allsender.tech` compilado; UI solo pide Branch Key |
| **B. Multi-instancia SaaS** | UI Domain + Key (como ahora) para white-label de otros dominios |
| **C. Híbrida** | Default URL Allsender; campo avanzado “Dominio personalizado” colapsado |

El **backend no cambia** en A/B/C: solo cambia la UX del .exe.

---

## 8. Errores frecuentes

| Síntoma | Causa | Fix |
|---------|-------|-----|
| 401 Invalid key | unique_hash reseteado o mal copiado | Nueva key en Ajustes → Impresora |
| Jobs siempre vacíos | `printing_choice` = browserPopupPrint | Cambiar a directPrint y generar job |
| Image not found | captura PNG falló en web | Reimprimir KOT; revisar `user-uploads/print` |
| Imprime en impresora incorrecta | mapping local mal | Refresh Printers + re-guardar mapping |
| Duplicados | SSE + poll sin dedupe | Set de job ids “en vuelo” en el agente |
