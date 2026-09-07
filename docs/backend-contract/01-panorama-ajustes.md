# 01 — Ajustes: KOT, Impresora y Conexión de escritorio

## 1. `/settings?tab=kot` → **Áreas de preparación**

**Componente:** `App\Livewire\Settings\KotAreasSettings`  
**Vista:** `resources/views/livewire/settings/kot-areas-settings.blade.php`  
**Tabla principal:** `kot_places`

### Qué es
Cada **área** (Cocina, Bar, Parrilla…) es un `kot_places` row ligado a:
- `branch_id`
- `printer_id` (opcional) → qué impresora física/virtual imprime esa área
- `name`, `type`, `is_active`, `is_default`

### Por qué importa para el .exe
Al imprimir un KOT multi-área, el servidor genera **un job por área**:

```
image_filename = kot-{kotId}-{kotPlaceId}.png
```

Ejemplo real en BD: `kot-595-11.png`  
Así Cocina y Bar no se pisan. El agente debe imprimir **cada job** en la impresora Windows mapeada a ese `printer_id` / `share_name` / nombre local.

### Relacionado (otra pestaña)
- `/settings?tab=kotSettings` → `KotSettings` (estados default POS/cliente, item-level status)  
  **No** es la app de escritorio; es lógica de cocina en el panel web.

---

## 2. `/settings?tab=printer` → **Configuración de impresora**

**Componente:** `App\Livewire\Settings\PrinterSetting`  
**Vista:** `resources/views/livewire/settings/printer-setting.blade.php`  
**Tabla:** `printers`

### Campos clave de cada impresora (`printers`)

| Columna | Significado |
|---------|-------------|
| `name` | Nombre lógico (ej. “Cocina principal”) |
| `printing_choice` | `browserPopupPrint` \| `directPrint` |
| `print_type` | `image` \| `pdf` |
| `print_format` | `thermal56mm` / `thermal80mm` / `thermal112mm` / null |
| `share_name` | Nombre de impresora compartida Windows (direct print) |
| `printer_name` | Nombre local en el PC del agente |
| `open_cash_drawer` | Abrir cajón tras imprimir |
| `kots` / `orders` | JSON/array de IDs de áreas / POS terminals |
| `is_default` | Impresora por defecto del branch |
| `type` | p.ej. receipt / kitchen (enviado en SSE) |

### Modos de impresión

| `printing_choice` | Quién imprime | Necesita .exe |
|-------------------|---------------|---------------|
| `browserPopupPrint` | Navegador (diálogo de impresión) | No |
| `directPrint` | Agente desktop en segundo plano | **Sí** |

Cuando hay impresoras en modo direct print, la UI muestra el aviso de **Desktop App Required**.

---

## 3. Bloque “Conexión de aplicación de escritorio”

Visible en la misma pantalla de impresora (si `desktop_applications` está activa):

### URL de dominio
```
https://restapp.allsender.tech
```
Fuente en Blade: `request()->getSchemeAndHttpHost()`  
El cliente la pega en el campo **Domain URL** del .exe.

### Clave secreta (= Branch Key)
```
{{ branch()->unique_hash }}
```
- Columna: `branches.unique_hash`
- Generación: `Branch::generateUniqueHash()` → `substr(hash('sha256', $baseString), 0, 20)`
- También se usa en **QR de pedidos** de mesa (`?branch={unique_hash}`)

### Restablecer clave de sucursal
`PrinterSetting::resetBranchKey()`:
1. Llama `$branch->generateUniqueHash()`
2. Guarda
3. Los .exe con la key vieja dejan de autenticar (401)

### Instrucciones al cliente (ya en UI)
1. Descargar e instalar app de escritorio  
2. Abrir configuración de la app  
3. Ingresar **URL** + **clave de rama**  
4. Conectar  

---

## 4. Descarga del .exe (Super Admin / Ajustes apps)

**Tabla:** `desktop_applications`  
**Modelo:** `App\Models\DesktopApplication`  
**UI Super Admin:** `DesktopApplicationSettings` (URLs Windows/Mac/Linux, apps delivery/waiter)

Campos actuales (producción):
- `windows_file_path` → Google Drive folder (Allsender)
- `mac_file_path` → Google Drive
- Default original TableTrack: `https://envato.froid.works/app/download/windows`

**Pestaña cliente:** `?tab=download` → `DownloadSettings` lista el link de descarga.

---

## 5. Cómo se ve el .exe actual (captura)

**Allsender RESTAper v3.0.0**

- Connection: Domain URL + API Key + Test Connection  
- Printer Setup: mapear cocina → impresora local, Refresh Printers  
- Save & Start Printing  
- Autostart / Check Updates  
- Cash Drawer: Open Drawer After Print  

Esto coincide 1:1 con el contrato de API del backend (test-connection, printer-details, pull jobs).
