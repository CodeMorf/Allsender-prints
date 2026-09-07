# Estado actual — RestaAPP Printer 3.1.2

Última actualización: **2026-09-07**

## En una frase

**Impresión RAW ESC/POS 203 DPI (nítida) validada en POS-80C; separación KOT/pre-cuenta/cuenta final; canal SSE y fallback de sondeo; instalador 3.1.2 listo.**

## Semáforo

| | |
|--|--|
| 🟢 Documentación y contrato API | Listo |
| 🟢 Código fuente del agente | Listo |
| 🟢 MSVC Build Tools + Rust MSVC | Instalado |
| 🟢 Build / release NSIS + MSI 3.1.2 | **Listo** |
| 🟢 Impresión térmica nítida | **RAW ESC/POS** (no GDI) en POS-80C |
| 🟡 QA multi-sucursal / 50 mm físico | Probar en campo |

## Dónde está el proyecto

```text
C:\RestaAPP_Printer_Starter_3.1.0\RestaAPP_Printer_Starter_3.1.0\
```

## Instaladores generados (target)

```text
src-tauri\target\release\bundle\nsis\RestaAPP Printer_3.1.2_x64-setup.exe
src-tauri\target\release\bundle\msi\RestaAPP Printer_3.1.2_x64_en-US.msi
src-tauri\target\release\restaapp-printer.exe
```

## Backend (por sucursal / usuario)

`GET /api/test-connection` con `X-TABLETRACK-KEY` ahora devuelve:

- `branch_id`, `restaurant_id`
- `branch_name`, `restaurant_name`, `logo_url`, `locale`
- Poll hints + pusher (si aplica)

`PATCH /api/print-jobs/{id}` y `POST .../printed` validan que el job sea de la **misma branch** de la key.

## Cómo probar

1. Instalar `RestaAPP Printer_3.1.2_x64-setup.exe`
2. Pegar **Branch Key** de Ajustes → Impresora en restapp
3. Probar conexión → debe mostrar restaurante · sucursal
4. Mapear área → Microsoft Print to PDF (o POS-80)
5. Iniciar servicio e imprimir KOT desde el POS
