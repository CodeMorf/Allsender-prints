param(
  [Parameter(Mandatory=$true)][string]$File,
  [Parameter(Mandatory=$true)][string]$Pfx,
  [Parameter(Mandatory=$true)][string]$Password,
  [string]$TimestampUrl = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"
$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Filter signtool.exe -Recurse | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $signtool) { throw "No se encontró SignTool. Instala Windows SDK desde Visual Studio Build Tools." }

& $signtool.FullName sign /fd SHA256 /f $Pfx /p $Password /tr $TimestampUrl /td SHA256 $File
& $signtool.FullName verify /pa /v $File
