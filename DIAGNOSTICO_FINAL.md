# DIAGNÓSTICO FINAL — RestaAPP Printer 3.1.0

**Fecha:** 2026-07-18 (compilación completada)  
**Producto:** RestaAPP Printer — By RestaAPP  
**Empresa:** GRUPOOHLA SRL  
**Proyecto:** `C:\RestaAPP_Printer_Starter_3.1.0\RestaAPP_Printer_Starter_3.1.0`  
**Estado global:** **INSTALADORES GENERADOS — QA HARDWARE PENDIENTE**

---

## 1. Veredicto

| Área | Estado | Notas |
|------|--------|--------|
| Código fuente (Tauri 2 + React + TS + Rust) | ✅ | Fix DPAPI `LocalFree(Some(...))` para windows-rs 0.59 |
| Contrato RestApp (no romper) | ✅ | URL fija, `X-TABLETRACK-KEY`, KOT multiárea |
| Backend multi-sucursal | ✅ | test-connection con nombre/logo; update/printed scoped a branch |
| Toolchain MSVC | ✅ | VS 2022 Build Tools + `link.exe` |
| `npm run desktop:build` | ✅ | NSIS + MSI generados |
| Instaladores en `release/` | ✅ | Ver sección 3 |
| Pruebas reales POS-80 / KOT | ⏳ | Pendiente en local del restaurante |
| Firma de código (PFX) | ⏳ Opcional | `CERTIFICADO_FIRMA.md` |

**Conclusión:** el instalador **sí se compiló** en esta PC. No se afirma QA de impresora térmica hasta probar en POS real.

---

## 2. Backend (por usuario / sucursal)

Servidor: `https://restapp.allsender.tech`  
Auth: header `X-TABLETRACK-KEY` = `branches.unique_hash` (por sucursal).

### Cambios desplegados en servidor (2026-07-18)

**Archivo:** `app/Http/Controllers/PrintJobController.php`  
**Backup:** `*.bak_desktop_20260718_*`

1. **`testConnection`** ahora responde, además de lo existente:
   - `branch_id`, `restaurant_id`
   - `branch_name`, `restaurant_name`
   - `logo_url`, `locale`
2. **`update` (PATCH print-jobs)** rechaza jobs de otra branch (404).
3. **`PrintStreamController::markPrinted`** misma validación de branch.

Smoke real (ejemplo):

```json
{
  "status": "success",
  "branch_id": 27,
  "restaurant_id": 26,
  "branch_name": "Sucursal Principal",
  "restaurant_name": "Ciro Kebab",
  "logo_url": "https://restapp.allsender.tech/user-uploads/logo/..."
}
```

Jobs y impresoras del API siguen filtrados por `branch_id` de la key (comportamiento previo conservado).

---

## 3. Instaladores generados

```text
C:\RestaAPP_Printer_Starter_3.1.0\RestaAPP_Printer_Starter_3.1.0\release\

  RestaAPP-Printer-Setup-3.1.0-x64.exe   (~3.34 MB)  ← recomendado
  RestaAPP-Printer-3.1.0-x64.msi         (~4.70 MB)
  restaapp-printer-3.1.0-x64.exe         (~12.9 MB)  portable
  SHA256SUMS.txt
  GUIA_INSTALACION_ES.md / EN / IT / FR
  CHANGELOG.md
```

Origen Tauri:

```text
src-tauri\target\release\bundle\nsis\RestaAPP Printer_3.1.0_x64-setup.exe
src-tauri\target\release\bundle\msi\RestaAPP Printer_3.1.0_x64_en-US.msi
src-tauri\target\release\restaapp-printer.exe
```

---

## 4. Cambios de código del agente (esta sesión)

| Archivo | Cambio |
|---------|--------|
| `src-tauri/src/secure.rs` | `LocalFree(Some(HLOCAL(...)))` compatible windows-rs 0.59 |
| `src-tauri/src/config.rs` | warning `mut` eliminado |
| `src/vite-env.d.ts` | tipos `*.png` para build TypeScript |
| `src/App.tsx` | guarda y muestra `restaurant_name` · `branch_name` por key |

---

## 5. Pruebas realizadas

| Prueba | Resultado |
|--------|-----------|
| `npm run build` (Vite/TS) | ✅ |
| `npm run desktop:build` (Tauri release) | ✅ NSIS + MSI |
| `GET /api/test-connection` con key real | ✅ identidad por branch |
| `php -l` controllers parcheados | ✅ |
| Impresión Microsoft Print to PDF / POS-80 | ⏳ No ejecutada aquí |
| KOT multi-área real | ⏳ No ejecutada aquí |

---

## 6. Cómo compilar de nuevo

Ver `COMO_COMPILAR_WINDOWS.md`. Resumen:

```powershell
$vsDev = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
cmd /c "`"$vsDev`" -arch=x64 -host_arch=x64 && cd /d C:\RestaAPP_Printer_Starter_3.1.0\RestaAPP_Printer_Starter_3.1.0 && npm run desktop:build"
```

---

## 7. Próximos pasos humanos

1. Instalar `release\RestaAPP-Printer-Setup-3.1.0-x64.exe` en PC del restaurante.
2. Configurar Branch Key de **esa** sucursal.
3. Mapear Cocina/Bar/Caja → impresoras Windows.
4. Probar KOT + recibo; si falla, exportar diagnóstico (clave enmascarada).
5. (Opcional) Firmar con PFX y subir a CDN / `desktop_applications`.
