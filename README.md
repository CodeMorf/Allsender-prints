# RestaAPP Printer 3.1.2 — Agente de impresión para Windows

Código fuente del agente de impresión de Windows de **RestaAPP**.

## Ya incluido

- Tauri 2 + React + TypeScript + Rust.
- Logo suministrado, icono `.ico` y pantalla inicial animada.
- Idiomas `es`, `es-DO`, `it`, `en` y `fr`; el alias heredado `do` se normaliza a `es-DO`.
- Dominio fijo `https://restapp.allsender.tech` y acceso mediante clave de sucursal.
- Prueba de conexión con `X-TABLETRACK-KEY`.
- Lectura de impresoras configuradas en la sucursal.
- Detección de impresoras instaladas en Windows.
- Asociación área de RestaAPP → impresora local.
- Prueba de impresión.
- Polling de trabajos, descarga del archivo, impresión y confirmación `done/failed`.
- Canal SSE de aviso inmediato con polling REST como respaldo; la reclamación atómica sigue siendo única para evitar duplicados.
- Segundo plano, bandeja de Windows, cierre minimizado y apertura automática.
- Reconexión, cola en vuelo y prevención básica de duplicados.
- Instalador NSIS/MSI y flujo de compilación para Windows.
- Instalador NSIS en español por defecto, con selector de español, inglés, italiano o francés.
- Separación de intención de impresión: KOT, pre-cuenta y cuenta final pagada.
- El cajón solo se evalúa para trabajos de cuenta final (`order-*`/`split-*`); nunca para KOT ni pre-cuenta.
- Registros técnicos fuera de la interfaz, en la carpeta local de diagnóstico.

## Estado del código (2026-09-07)

- Flujo principal y mejoras (DPAPI, impresión RAW/GDI, bandeja, diagnóstico) están en el código.
- La protección de impresión KOT/pre-cuenta/cuenta final está compilada y validada con `cargo check`.
- El instalador 3.1.2 NSIS/MSI fue generado en Windows x64; la prueba con hardware térmico real sigue siendo una validación de campo.
- Lee en este orden: **`ESTADO_ACTUAL.md`** → **`DIAGNOSTICO_FINAL.md`** → **`COMO_COMPILAR_WINDOWS.md`**.

## Flujo de impresión y cajón

| Trabajo | Uso | Cajón |
|---|---|---|
| `kot-{kotId}-{kotPlaceId}` | Comanda interna para Cocina, Bar u otra estación | Nunca |
| `pre-account-{orderId}` | Pre-cuenta completa antes del cobro | Nunca |
| `order-{orderId}` / `split-*` | Cuenta final o recibo de pago | Solo después de `paid` y si la configuración lo habilita |

El servidor impide que una orden no pagada se convierta en recibo final. El agente aplica una segunda barrera antes de enviar el trabajo a la impresora.

## Actualización de clientes existentes

El instalador 3.1.2 reemplaza el programa, pero la configuración de cada equipo se mantiene fuera del ejecutable en:

```text
%APPDATA%\Allsender\RestaAPP Printer\config\config.json
```

La clave de sucursal se conserva protegida con Windows DPAPI, junto con los mapeos locales de impresoras. Antes de usar el agente con una caja real, instala 3.1.2 y verifica conexión, mapeo de áreas y una impresión de prueba.

## Requisitos en Windows

- Windows 10 u 11 x64.
- Node.js 20 LTS.
- Rust estable.
- Visual Studio Build Tools 2022 con “Desktop development with C++” y Windows SDK.

## Ejecutar en desarrollo

```powershell
npm install
npm run desktop:dev
```

## Crear instalador

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1
```

Los instaladores aparecerán en:

```text
src-tauri\target\release\bundle\nsis\
src-tauri\target\release\bundle\msi\
```

## Contrato obligatorio

- Base: `https://restapp.allsender.tech`
- Header: `X-TABLETRACK-KEY`
- Clave: `branches.unique_hash`
- Test: `GET /api/test-connection`
- Impresoras: `GET /api/printer-details`
- Trabajos: `GET /api/print-jobs/pull-multiple`
- Aviso en tiempo real: `GET /api/print-stream/{branch_unique_hash}`
- Resultado: `PATCH /api/print-jobs/{id}`
- No cambiar el flujo KOT multiárea ni el nombre `kot-{kotId}-{kotPlaceId}.png`.

## Pendientes para distribución pública

1. Firmar EXE/MSI con certificado público o servicio administrado.
2. Validar SHA-256 y firma en actualizaciones.
3. Evitar incluir claves reales dentro del repositorio.
4. Aplicar retención configurable de registros y mantener la exportación de diagnóstico lista para soporte.
