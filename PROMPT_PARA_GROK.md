# PROMPT PARA GROK — TERMINAR RESTAAPP PRINTER

Abre y analiza completamente esta carpeta del proyecto. Debes terminar un software de escritorio profesional para Windows llamado **RestaAPP Printer — By RestaAPP**, propiedad de **GRUPOOHLA SRL**.

## Objetivo

Entregar un instalador Windows x64 funcional que se inicie con Windows, permanezca en segundo plano y reciba automáticamente trabajos de impresión desde `https://restapp.allsender.tech` para enviarlos a impresoras locales de Cocina, Bar, Caja u otras áreas.

No reemplaces el proyecto por Electron, Flutter ni una página web. Mantén **Tauri 2 + React + TypeScript + Rust**.

## Antes de modificar

1. Lee `README.md`.
2. Lee todos los documentos de `docs/backend-contract/`.
3. Revisa el frontend y cada módulo Rust.
4. Ejecuta instalación, chequeo TypeScript, `cargo check`, modo desarrollo y compilación de instalador.
5. Conserva una copia de seguridad antes de cambios grandes.

## Identidad visual

- Nombre: `RestaAPP Printer`.
- Texto secundario: `By RestaAPP`.
- Empresa: `GRUPOOHLA SRL`.
- Usa el logo existente en `src/assets/restaapp-logo.png` y los iconos de `src-tauri/icons/`.
- Mantén una pantalla inicial animada de aproximadamente 2–3 segundos.
- Diseño moderno, oscuro, comercial, responsive y usable con pantallas pequeñas.
- No mostrar textos técnicos, trazas, rutas internas ni errores de programación en la interfaz.
- En la interfaz usar mensajes profesionales como “No hay impresiones pendientes”, “Sucursal conectada correctamente”, “La conexión se restablecerá automáticamente” y “Revisión requerida”.
- Los detalles técnicos deben quedar exclusivamente en registros y archivos de diagnóstico.

## Idiomas

Debe funcionar completamente en:

- `es` — Español.
- `es-DO` — Español de República Dominicana.
- `it` — Italiano.
- `en` — Inglés.
- `fr` — Francés.

Aceptar el alias heredado `do`, pero guardarlo internamente como `es-DO`. No dejar textos sin traducir. Detectar primero el idioma devuelto por la empresa/sucursal, luego Windows, luego la preferencia local.

## Contrato obligatorio de RestApp

```text
BASE = https://restapp.allsender.tech
AUTH = X-TABLETRACK-KEY: {branches.unique_hash}
```

Endpoints existentes que NO debes renombrar ni romper:

```text
GET   /api/test-connection
GET   /api/printer-details
GET   /api/print-jobs/pull-multiple
PATCH /api/print-jobs/{id}
POST  /api/print-jobs/{id}/printed
GET   /api/print-stream/{KEY}
```

Reglas críticas:

- La clave pertenece a la sucursal, no al restaurante.
- El dominio queda fijo y oculto para el cliente normal.
- Mantener exactamente el header histórico `X-TABLETRACK-KEY`.
- Respetar estados `pending`, `printing`, `done`, `failed`.
- Respetar KOT multiárea. Cada área puede tener una impresora diferente.
- No modificar ni ignorar la convención `kot-{kotId}-{kotPlaceId}.png`.
- Nunca borrar trabajos ni documentos del servidor desde el agente.
- Polling siempre disponible; SSE puede usarse como acelerador, pero sin duplicar trabajos.

## Funciones obligatorias

1. Conexión con solo clave de sucursal.
2. Mostrar logo, nombre del restaurante y nombre de la sucursal cuando la API los devuelva.
3. Detectar automáticamente impresoras instaladas en Windows.
4. Mostrar impresora predeterminada, red, compartida y disponibilidad cuando Windows lo permita.
5. Asociar cada impresora/área del servidor a una impresora local.
6. Guardar mapeo por `printer.id`, nunca solamente por nombre.
7. Botón para actualizar la lista de impresoras.
8. Prueba de conexión completa.
9. Prueba real de impresión.
10. Polling inteligente usando `X-Print-Poll-Ms`.
11. Reconexión automática y backoff.
12. Cola local en memoria y persistente para trabajos descargados.
13. Prevención fuerte de duplicados por `job.id`.
14. Imprimir PNG y PDF.
15. Formatos térmicos 56, 80 y 112 mm.
16. Motor nativo Windows WinSpool/GDI o ESC/POS. El uso actual de Paint/PrintTo es solo temporal: reemplázalo para producción.
17. Corte de papel cuando el modelo lo permita.
18. Apertura de cajón mediante ESC/POS configurable.
19. Número de copias.
20. Impresora de respaldo por área.
21. Bandeja de Windows con Abrir, Pausar/Reanudar, Imprimir prueba y Salir.
22. Cerrar la ventana debe minimizar, no detener el servicio.
23. Inicio automático al iniciar sesión en Windows, abierto en segundo plano.
24. Recuperarse después de suspensión, cambio de red y reinicio.
25. Identificador único del dispositivo.
26. Registros rotativos en `%APPDATA%\Allsender\RestaAPP Printer\logs\`.
27. Exportar un ZIP de diagnóstico sin incluir la clave completa.
28. Pantalla Acerca de con versión y dispositivo.
29. Instalador NSIS o MSI con desinstalador.
30. Preparar actualizaciones firmadas, con SHA-256 y canal stable/beta.

## Seguridad obligatoria

- Proteger la clave mediante Windows DPAPI o Credential Manager; no dejarla en texto plano.
- Nunca escribir la clave completa en registros.
- HTTPS obligatorio.
- Sanitizar nombres y rutas.
- Validar tamaño, tipo y contenido de archivos descargados.
- Aplicar límite de tiempo y reintentos.
- No ejecutar contenido descargado.
- Toda actualización debe validar firma y hash.
- No incluir credenciales reales en el repositorio.

## Interfaz

Mantener dos áreas principales:

### Configuración inicial

- Logo animado.
- Campo Clave de sucursal.
- Idioma.
- Iniciar con Windows.
- Probar conexión.
- Cuando conecta, cargar automáticamente impresoras de la sucursal y pasar al panel.

### Panel principal

- Estado: conectado, imprimiendo, reconectando, pausado o revisión requerida.
- Nombre/logo de empresa y sucursal.
- Impresiones del día.
- Última impresión.
- Lista de áreas y su impresora Windows asociada.
- Imprimir prueba.
- Iniciar/Pausar servicio.
- Configuración del cajón.
- Abrir diagnóstico.

No mostrar errores internos. Usa códigos internos en logs y mensajes comerciales en UI.

## Pruebas obligatorias

- `npm run build` sin errores.
- `cargo fmt --check`.
- `cargo clippy -- -D warnings` o justificar advertencias inevitables.
- `cargo check`.
- `npm run desktop:dev`.
- Crear instalador x64.
- Probar con Microsoft Print to PDF.
- Probar al menos con `POS-80` real.
- KOT de una sola área.
- KOT de Cocina + Bar en impresoras diferentes.
- Recibo de caja.
- Impresora apagada.
- Sin internet y recuperación posterior.
- Reinicio de Windows.
- Restablecimiento de la Branch Key.
- Evitar duplicados usando polling y SSE.
- Idiomas completos.

## Firma y publicación

- Mantén el workflow de GitHub para compilación, pero no exijas GitHub para ejecutar el proyecto localmente.
- Permite firma con PFX o servicio administrado.
- Nunca subas PFX, tokens o contraseñas.
- Genera SHA-256 de cada instalador.

## Entrega final

Al terminar, entrega dentro de una carpeta `release/`:

```text
RestaAPP-Printer-Setup-3.1.0-x64.exe
RestaAPP-Printer-3.1.0-x64.msi
SHA256SUMS.txt
CHANGELOG.md
GUIA_INSTALACION_ES.md
GUIA_INSTALACION_IT.md
GUIA_INSTALACION_EN.md
GUIA_INSTALACION_FR.md
DIAGNOSTICO_FINAL.md
```

En `DIAGNOSTICO_FINAL.md` documenta qué se completó, pruebas realizadas, impresoras probadas, pendientes reales y ubicación exacta de los instaladores. No digas que algo funciona si no fue compilado y probado.
