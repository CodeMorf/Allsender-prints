# Cómo compilar RestaAPP Printer 3.1.2 en Windows x64

## Requisitos

1. **Windows 10/11 x64**
2. **Node.js 20 LTS o superior** (en esta PC: Node 24 OK)
3. **Rust estable MSVC**
4. **Visual Studio 2022 Build Tools** con:
   - Workload: *Desktop development with C++*
   - Windows 10/11 SDK
5. **WebView2 Runtime** (normalmente ya instalado)

## 1. Instalar Build Tools (si falta `link.exe`)

### Opción A — winget (recomendado)

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools -e `
  --accept-package-agreements --accept-source-agreements `
  --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Tarda 15–40 minutos. Al terminar, **cierra y abre** PowerShell.

### Opción B — instalador visual

1. Descargar [Build Tools for Visual Studio 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. Marcar **Desarrollo para el escritorio con C++**
3. Instalar y reiniciar terminal

### Verificar

```powershell
# Debe encontrar link.exe en VS 2022
Get-ChildItem "C:\Program Files*\Microsoft Visual Studio\2022" -Filter link.exe -Recurse -ErrorAction SilentlyContinue |
  Select-Object -First 1 FullName
```

## 2. Configurar Rust MSVC

```powershell
rustup default stable-x86_64-pc-windows-msvc
rustc --version
cargo --version
```

Si `cargo` no se reconoce:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
```

## 3. Compilar el proyecto

```powershell
cd C:\RestaAPP_Printer_Starter_3.1.0\RestaAPP_Printer_Starter_3.1.0

npm install
npm run build
cd src-tauri
cargo check
cd ..
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1
```

O:

```powershell
npm run desktop:build
```

## 4. Copiar a `release\`

```powershell
$root = "C:\RestaAPP_Printer_Starter_3.1.0\RestaAPP_Printer_Starter_3.1.0"
$rel  = Join-Path $root "release"
New-Item -ItemType Directory -Force -Path $rel | Out-Null

$nsis = Get-ChildItem "$root\src-tauri\target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
$msi  = Get-ChildItem "$root\src-tauri\target\release\bundle\msi\*.msi"  -ErrorAction SilentlyContinue | Select-Object -First 1

if ($nsis) {
  Copy-Item $nsis.FullName (Join-Path $rel "RestaAPP-Printer-Setup-3.1.2-x64.exe") -Force
}
if ($msi) {
  Copy-Item $msi.FullName (Join-Path $rel "RestaAPP-Printer-3.1.2-x64.msi") -Force
}

# SHA-256
Get-ChildItem $rel -Include *.exe,*.msi -File -ErrorAction SilentlyContinue | ForEach-Object {
  $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
  "$h  $($_.Name)"
} | Set-Content (Join-Path $rel "SHA256SUMS.txt") -Encoding ASCII

Write-Host "Listo en $rel"
Get-ChildItem $rel
```

## 5. Pruebas mínimas después del instalador

1. Instalar el Setup x64  
2. Abrir RestaAPP Printer  
3. Pegar **Branch Key** de `https://restapp.allsender.tech/settings?tab=printer`  
4. **Probar conexión**  
5. Mapear un área → **Microsoft Print to PDF**  
6. **Iniciar servicio**  
7. En el POS, imprimir un KOT y verificar PDF / cola  
8. (Si hay hardware) mapear a **POS-80** y repetir  

## 6. Errores frecuentes

| Error | Solución |
|-------|----------|
| `linker link.exe not found` | Instalar VS Build Tools C++ y reiniciar terminal |
| `dlltool.exe not found` | No uses toolchain GNU; usa `pc-windows-msvc` |
| `WebView2 not found` | Instalar Evergreen WebView2 Runtime |
| 401 en test-connection | Clave incorrecta o reseteada en Ajustes → Impresora |
| Jobs vacíos | Impresora del servidor en `directPrint`, no solo browser |

## 7. Firma (opcional)

Ver `CERTIFICADO_FIRMA.md` y `scripts\sign-with-pfx.ps1`.  
Nunca subir el PFX al repositorio.
