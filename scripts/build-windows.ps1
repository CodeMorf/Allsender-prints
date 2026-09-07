$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "Instalando dependencias..."
npm install

Write-Host "Compilando RestaAPP Printer para Windows..."
npm run desktop:build

Write-Host "Proceso completado. Revisa src-tauri\target\release\bundle"
