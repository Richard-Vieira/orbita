# Baixa o pdfium (bblanchon/pdfium-binaries) numa versão fixa e confere o SHA-256.
# Uso: powershell -ExecutionPolicy Bypass -File scripts/fetch-pdfium.ps1
# Resultado: vendor/pdfium/bin/pdfium.dll (a pasta vendor/ não vai para o git).
#
# Ao atualizar a versão, troque também a feature "pdfium_XXXX" do pdfium-render
# em crates/orbita-core/Cargo.toml.

$ErrorActionPreference = 'Stop'

$Version = '7881'
$Sha256  = '73CC0DE638AC2095E7445BF56A38200A5B7C7CA0E9F4BA144598F2457377AC08'
$Url     = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F$Version/pdfium-win-x64.tgz"

$Root   = Split-Path -Parent $PSScriptRoot
$Dest   = Join-Path $Root 'vendor\pdfium'
$Marker = Join-Path $Dest ".version-$Version"

if (Test-Path $Marker) {
    Write-Host "pdfium $Version já está em $Dest"
    exit 0
}

$Tmp = Join-Path ([IO.Path]::GetTempPath()) "pdfium-$Version.tgz"
Write-Host "Baixando pdfium $Version..."
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri $Url -OutFile $Tmp -UseBasicParsing

$Actual = (Get-FileHash -Algorithm SHA256 $Tmp).Hash
if ($Actual -ne $Sha256) {
    Remove-Item $Tmp -Force
    throw "SHA-256 não confere (esperado $Sha256, recebido $Actual)"
}

if (Test-Path $Dest) { Remove-Item $Dest -Recurse -Force }
New-Item -ItemType Directory -Force $Dest | Out-Null
# tar.exe do próprio Windows (o tar do Git Bash não entende caminhos "C:\").
& "$env:SystemRoot\System32\tar.exe" -xzf $Tmp -C $Dest
if ($LASTEXITCODE -ne 0) { throw 'Falha ao extrair o pdfium' }
Remove-Item $Tmp -Force
New-Item -ItemType File $Marker | Out-Null

Write-Host "pdfium $Version pronto em $Dest\bin\pdfium.dll"
