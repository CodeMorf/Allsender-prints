# 08 — Chuleta de endpoints (imprimible)

```
BASE = https://restapp.allsender.tech
KEY  = branches.unique_hash   (Ajustes → Impresora → Clave secreta)
H    = X-TABLETRACK-KEY: {KEY}
```

| Acción | Método | URL | Auth |
|--------|--------|-----|------|
| Probar conexión | GET | `{BASE}/api/test-connection` | Header H |
| Listar printers del branch | GET | `{BASE}/api/printer-details` | Header H |
| Pull jobs | GET | `{BASE}/api/print-jobs/pull-multiple` | Header H |
| Marcar done/failed | PATCH | `{BASE}/api/print-jobs/{id}` | Header H + JSON body |
| Marcar printed (SSE) | POST | `{BASE}/api/print-jobs/{id}/printed` | Header H |
| Stream SSE | GET | `{BASE}/api/print-stream/{KEY}` | Key en path |

### Body PATCH
```json
{ "status": "done|failed", "printed_at": "ISO8601?", "error": "string?", "printer": "WindowsName?" }
```

### Imagen del job
```
{BASE}/user-uploads/print/{image_filename}
```
también en campo `image_path` del JSON.
