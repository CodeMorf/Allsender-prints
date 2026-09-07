# Historial de cambios

## 3.1.2 — 2026-09-07

- El agente abre un canal SSE por sucursal para despertar el procesamiento cuando el servidor crea un trabajo de impresión.
- El polling REST continúa siendo el mecanismo que reclama los trabajos de forma atómica; SSE no crea una segunda ruta de impresión ni duplica KOT.
- El servidor publica `X-Print-Poll-Ms` con respaldo de 1.5 s cuando hay trabajos y 3 s cuando la cola está vacía.
- La interfaz y los instaladores Windows NSIS/MSI quedan versionados como 3.1.2.
- La configuración local, la clave protegida con DPAPI y los mapeos de cada cliente se conservan al actualizar.

## Pendientes de distribución pública

- Firma pública de EXE/MSI.
- Validación de firma y SHA-256 dentro del mecanismo de actualización.
- Prueba de campo por restaurante con la impresora térmica instalada.
