# Instala (ou atualiza) o Órbita pelo terminal, a partir da última versão
# publicada em https://github.com/Richard-Vieira/orbita/releases
#
# Uso (PowerShell):
# (arquivo salvo SEM BOM de propósito: o "irm | iex" quebraria com ele)
#   irm https://raw.githubusercontent.com/Richard-Vieira/orbita/main/scripts/install.ps1 | iex
#
# O que ele faz:
#   1. baixa o instalador da última versão;
#   2. confere o SHA-256 com o SHA256SUMS.txt publicado junto;
#   3. instala só para o usuário atual (sem pedir administrador) e abre o Órbita.

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repo = 'Richard-Vieira/orbita'
Write-Host 'Procurando a última versão do Órbita...'
$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" -Headers @{ 'User-Agent' = 'orbita-install' }

$asset = $release.assets | Where-Object { $_.name -like 'Orbita_*_x64-setup.exe' } | Select-Object -First 1
if (-not $asset) { throw "Nenhum instalador encontrado na versão $($release.tag_name)." }
$sums = $release.assets | Where-Object { $_.name -eq 'SHA256SUMS.txt' } | Select-Object -First 1

$dir = Join-Path ([IO.Path]::GetTempPath()) "orbita-$($release.tag_name)"
New-Item -ItemType Directory -Force $dir | Out-Null
$setup = Join-Path $dir 'orbita-setup.exe'

Write-Host "Baixando $($asset.name) ($([math]::Round($asset.size / 1MB, 1)) MB)..."
Invoke-WebRequest $asset.browser_download_url -OutFile $setup -UseBasicParsing

if ($sums) {
    $sumsFile = Join-Path $dir 'SHA256SUMS.txt'
    Invoke-WebRequest $sums.browser_download_url -OutFile $sumsFile -UseBasicParsing
    $line = Get-Content $sumsFile | Where-Object { $_ -like "*$($asset.name)*" } | Select-Object -First 1
    $expected = if ($line) { ($line.Trim() -split '\s+')[0] } else { $null }
    $actual = (Get-FileHash -Algorithm SHA256 $setup).Hash
    if (-not $expected -or $actual -ne $expected.ToUpper()) {
        Remove-Item $setup -Force
        throw 'O SHA-256 do download não confere. Instalação cancelada.'
    }
    Write-Host 'SHA-256 conferido.'
}

Write-Host 'Instalando...'
$process = Start-Process -FilePath $setup -ArgumentList '/S' -Wait -PassThru
Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
if ($process.ExitCode -ne 0) { throw "O instalador terminou com código $($process.ExitCode)." }

Start-Process (Join-Path $env:LOCALAPPDATA 'Órbita\orbita.exe')
Write-Host ''
Write-Host "Órbita $($release.tag_name) instalado!" -ForegroundColor Green
Write-Host 'Clique com o botão direito num PDF ou imagem > "Converter com Órbita",'
Write-Host 'ou selecione o arquivo e aperte Ctrl+Alt+Espaço.'
