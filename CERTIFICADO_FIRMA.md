# Firma de código para Windows

## Opciones

### 1. Certificado autofirmado — gratis, solo para pruebas

El script `scripts/create-self-signed-dev-cert.ps1` crea un certificado local. Sirve para desarrollo interno y computadoras donde el certificado se instale manualmente como confiable. No convierte el instalador en un programa públicamente reconocido por Windows.

### 2. SignPath Foundation — gratis para proyectos Open Source elegibles

Puede firmar proyectos públicos y verdaderamente Open Source, sujetos a aprobación y a sus condiciones. El certificado pertenece al programa de la fundación y la compilación debe integrarse con su proceso verificable.

### 3. Certificado comercial o servicio administrado

Para un producto privado/comercial distribuido a restaurantes, la opción normal es una identidad verificada de empresa mediante una autoridad certificadora o un servicio de firma administrado. Debe emitirse a nombre de **GRUPOOHLA SRL** tras validar la empresa.

## Importante

Firmar identifica al editor y protege la integridad del archivo, pero Microsoft SmartScreen también considera la reputación del editor y del archivo. Un instalador nuevo puede recibir advertencias al principio incluso estando correctamente firmado.

## Uso de PFX

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\sign-with-pfx.ps1 `
  -File ".\RestaAPP-Printer-Setup.exe" `
  -Pfx ".\empresa.pfx" `
  -Password "CLAVE_SEGURA"
```

No guardes el PFX ni su contraseña dentro de GitHub o del ZIP público.
