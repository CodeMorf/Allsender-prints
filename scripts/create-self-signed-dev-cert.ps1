param(
  [string]$Publisher = "GRUPOOHLA SRL",
  [string]$Password = "Cambiar-Esta-Clave-123!"
)

$secure = ConvertTo-SecureString -String $Password -Force -AsPlainText
$cert = New-SelfSignedCertificate `
  -Type CodeSigningCert `
  -Subject "CN=$Publisher" `
  -CertStoreLocation "Cert:\CurrentUser\My" `
  -KeyAlgorithm RSA `
  -KeyLength 3072 `
  -HashAlgorithm SHA256 `
  -NotAfter (Get-Date).AddYears(2)

$Out = Join-Path $PSScriptRoot "restaapp-dev-signing.pfx"
Export-PfxCertificate -Cert $cert -FilePath $Out -Password $secure | Out-Null
Write-Host "Certificado de desarrollo creado en: $Out"
Write-Host "Úsalo únicamente en equipos de prueba donde se instale manualmente como certificado confiable."
