# 03 — SQL y modelos

## Tablas involucradas

### `branches`
| Columna | Tipo / notas | Uso desktop |
|---------|--------------|-------------|
| `id` | PK | `print_jobs.branch_id`, filtro jobs |
| `restaurant_id` | FK | multi-tenant |
| `name` | string | UI |
| **`unique_hash`** | string ~20 hex | **API KEY del .exe** |
| address, lat, lng, clones… | — | otros módulos |

**Generar key:**
```php
// Branch::generateUniqueHash()
$this->unique_hash = substr(hash('sha256', $baseString), 0, 20);
```

---

### `printers`
| Columna | Notas |
|---------|--------|
| `id` | PK |
| `restaurant_id`, `branch_id` | tenant |
| `name` | nombre lógico |
| `printing_choice` | `browserPopupPrint` \| `directPrint` |
| `print_type` | `image` \| `pdf` |
| `print_format` | thermal56/80/112mm |
| `share_name` | impresora compartida Windows |
| `printer_name` | nombre local Windows |
| `open_cash_drawer` | bool/flag |
| `kots` | cast array — IDs kot_places |
| `orders` | cast array — IDs multiple_orders |
| `is_default`, `is_active` | flags |
| `type` | kitchen/receipt (SSE) |
| `invoice_qr_code`, `characters_per_line` | legacy thermal |

Relaciones útiles:
- `Printer` → `kotPlaces()`, `orderPlaces()`
- `Printer::printerConnected` append: “Connected / Not Connected” según `printing_choice`

---

### `kot_places` (Áreas — tab=kot)
| Columna | Notas |
|---------|--------|
| `id` | = kotPlaceId en filename |
| `branch_id` | |
| `printer_id` | FK → printers (nullable) |
| `name` | Cocina, Bar… |
| `type` | food, etc. |
| `is_active`, `is_default` | |

---

### `kot_settings` (tab=kotSettings)
| Columna | Notas |
|---------|--------|
| `branch_id` | |
| `default_status_pos` | pending / … |
| `default_status_customer` | |
| `enable_item_level_status` | bool |

---

### `print_jobs` ⭐ cola del agente
| Columna | Notas |
|---------|--------|
| `id` | job id |
| `printer_id` | a qué impresora lógicamente va |
| `restaurant_id` | |
| `branch_id` | filtro del agente |
| **`status`** | `pending` → `printing` → `done` \| `failed` |
| `error` | mensaje si failed |
| `response_printer` | nombre impresora Windows reportado por .exe |
| **`image_filename`** | `kot-595-11.png` |
| `printed_at` | timestamp |
| `payload` | JSON (legacy ESC/POS text, opcional) |
| `created_at` / `updated_at` | |

**Appended:** `image_path` = URL pública completa.

Estados esperados por el agente:
1. Lee solo `pending`
2. Al pull-multiple el server marca `printing`
3. Agente imprime → `done` o `failed`

Cron: `print-jobs:prune-stuck --minutes=15` cada 5 min.

---

### `desktop_applications`
| Columna | Notas |
|---------|--------|
| `windows_file_path` | URL descarga Windows |
| `mac_file_path` | URL Mac |
| `linux_file_path` | |
| `partner_app_*` | delivery apps |
| `waiter_pos_app_*` | waiter POS apps |

Una sola fila global (Super Admin).

---

### Tablas relacionadas (contexto, no API desktop)

- `kots` — tickets de cocina  
- `orders` — pedidos  
- `multiple_orders` — “POS terminals” / order places  
- `receipt_settings` — layout de recibo  

---

## Ejemplo real (producción 2026-07-18)

**PrintJob reciente:**
```json
{
  "id": 1,
  "printer_id": 14,
  "restaurant_id": 8,
  "branch_id": 9,
  "status": "pending",
  "image_filename": "kot-595-11.png",
  "image_path": "https://restapp.allsender.tech/user-uploads/print/kot-595-11.png"
}
```

**Branch key (concepto):** el valor de `branches.unique_hash` de la sucursal activa del usuario (la que muestra Ajustes → Impresora).

---

## Diagramas ER (simplificado)

```
restaurants 1──* branches
branches 1──* printers
branches 1──* kot_places
branches 1──* print_jobs
kot_places *──1 printers   (printer_id)
print_jobs *──1 printers
print_jobs *──1 branches
```
