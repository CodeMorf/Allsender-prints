param(
  [string]$Publisher = "GRUPOOHLA SRL",
  [Parameter(Mandatory=$true)][SecureString]$Password
)

$cert = New-SelfSignedCertificate `
  -Type CodeSigningCert `
  -Subject "CN=$Publisher" `
  -CertStoreLocation "Cert:\CurrentUser\My" `
  -KeyAlgorithm RSA `
  -KeyLength 3072 `
  -HashAlgorithm SHA256 `
  -NotAfter (Get-Date).AddYears(2)

$Out = Join-Path $PSScriptRoot "restaapp-dev-signing.pfx"
Export-PfxCertificate -Cert $cert -FilePath $Out -Password $Password | Out-Null
Write-Host "Certificado de desarrollo creado en: $Out"
Write-Host "Úsalo únicamente en equipos de prueba donde se instale manualmente como certificado confiable."
