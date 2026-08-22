#Requires -Version 5.1
<#
.SYNOPSIS
  Builds and signs the sparse MSIX package for Meetral package identity.

  - Syncs Identity Version from src-tauri/tauri.conf.json (appends .0 to semver).
  - Signing cert resolution order:
      1. $env:SPARSE_DEV_PFX_BASE64 (+ $env:SPARSE_DEV_PFX_PASSWORD) - CI path
      2. Existing cert in Cert:\CurrentUser\My matching -Publisher - dev machine
      3. Create a new self-signed cert - first-run dev convenience
  - Stages Meetral-sparse.msix + logos + installer runtime scripts into -OutDir
    (src-tauri/installer/sparse by default), which the NSIS hook ships into $INSTDIR.
  - Emits MeetralAppList targetsize altform-unplated / lightunplated assets +
    resources.pri so the taskbar does not paint a solid plate when package
    identity is active (tray / desktop / title-bar keep using the EXE .ico).
#>
[CmdletBinding()]
param(
  [string]$ConfigPath,
  [string]$PkgRoot,
  [string]$OutDir,
  [string]$Publisher = 'CN=MeetralDev',
  [string]$PackageName = 'Meetral'
)
$ErrorActionPreference = 'Stop'

$scriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
$repoRoot  = (Resolve-Path (Join-Path $scriptDir '..\..')).Path
if (-not $ConfigPath) { $ConfigPath = Join-Path $repoRoot 'src-tauri\tauri.conf.json' }
if (-not $PkgRoot)    { $PkgRoot    = Join-Path $scriptDir 'pkg-root' }
if (-not $OutDir)     { $OutDir     = Join-Path $repoRoot 'src-tauri\installer\sparse' }
$PkgRoot = (Resolve-Path $PkgRoot).Path
$null = New-Item -ItemType Directory -Force -Path $OutDir
$OutDir = (Resolve-Path $OutDir).Path

# --- SDK tools (latest installed Windows SDK with makeappx) ----------------------
$sdkKitsRoot = 'C:\Program Files (x86)\Windows Kits\10\bin'
$sdkBin = Get-ChildItem $sdkKitsRoot -Directory -ErrorAction SilentlyContinue |
  Where-Object { Test-Path (Join-Path $_.FullName 'x64\makeappx.exe') } |
  Sort-Object Name -Descending | Select-Object -First 1
if (-not $sdkBin) { throw "Windows SDK makeappx.exe not found under $sdkKitsRoot" }
$makeappx = Join-Path $sdkBin.FullName 'x64\makeappx.exe'
$signtool = Join-Path $sdkBin.FullName 'x64\signtool.exe'
$makepri = Join-Path $sdkBin.FullName 'x64\makepri.exe'
Write-Host "SDK: $($sdkBin.Name)"

# --- 1. Sync Identity Version (+ Publisher) from tauri.conf.json -----------------
$conf = Get-Content $ConfigPath -Raw | ConvertFrom-Json
$v = [string]$conf.version
$quad = if (($v -split '\.').Count -ge 4) { $v } else { "$v.0" }

$manifestPath = Join-Path $PkgRoot 'AppxManifest.xml'
$xml = [xml](Get-Content $manifestPath -Raw)
$xml.PreserveWhitespace = $true
$xml.Package.Identity.Version = $quad
$xml.Package.Identity.Publisher = $Publisher
$xml.Save($manifestPath)
Write-Host "Identity: $PackageName v$quad, Publisher=$Publisher"

# --- 1b. Logos + unplated taskbar variants ---------------------------------------
# With sparse package identity, the shell taskbar uses uap:VisualElements
# (Square44x44Logo attribute) — NOT the EXE icon. We ship brand-named files:
#   MeetralAppList.png  ← 44px app-list / taskbar
#   MeetralTile.png     ← 150px medium tile / Properties.Logo
# BackgroundColor="transparent" alone still gets a system plate unless
# altform-unplated (+ lightunplated) targetsize assets are in resources.pri.
$iconsDir = Join-Path $repoRoot 'src-tauri\icons'
$appListSrc = Join-Path $iconsDir 'Square44x44Logo.png'
$tileSrc = Join-Path $iconsDir 'Square150x150Logo.png'
if (-not (Test-Path $appListSrc)) { throw "Missing icon source: $appListSrc" }
if (-not (Test-Path $tileSrc)) { throw "Missing icon source: $tileSrc" }
# Drop legacy Square* copies + previous Meetral* variants from pkg-root.
Get-ChildItem $PkgRoot -Filter 'Square*.png' -ErrorAction SilentlyContinue | Remove-Item -Force
Get-ChildItem $PkgRoot -Filter 'Meetral*.png' -ErrorAction SilentlyContinue | Remove-Item -Force
Copy-Item $appListSrc (Join-Path $PkgRoot 'MeetralAppList.png') -Force
Copy-Item $tileSrc (Join-Path $PkgRoot 'MeetralTile.png') -Force
$baseAppList = Join-Path $PkgRoot 'MeetralAppList.png'
# Include 44 explicitly — Desktop Bridge unplated guidance keys off targetsize-44.
$targetSizes = @(16, 20, 24, 30, 32, 36, 40, 44, 48, 60, 64, 72, 80, 96, 256)
foreach ($size in $targetSizes) {
  foreach ($alt in @('altform-unplated', 'altform-lightunplated')) {
    # Same bitmap, qualifier in the filename — Windows scales as needed (Desktop
    # Bridge guidance). Keeps the build free of a raster pipeline dependency.
    Copy-Item $baseAppList (Join-Path $PkgRoot "MeetralAppList.targetsize-${size}_${alt}.png") -Force
  }
}
if (-not (Test-Path $makepri)) { throw "Windows SDK makepri.exe not found next to makeappx" }
$priConfig = Join-Path $PkgRoot 'priconfig.xml'
$priOut = Join-Path $PkgRoot 'resources.pri'
Remove-Item $priConfig, $priOut -Force -ErrorAction SilentlyContinue
& $makepri createconfig /cf $priConfig /dq en-US /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makepri createconfig failed ($LASTEXITCODE)" }
& $makepri new /pr $PkgRoot /cf $priConfig /of $priOut /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makepri new failed ($LASTEXITCODE)" }
Remove-Item $priConfig -Force -ErrorAction SilentlyContinue
Write-Host "Logos: MeetralAppList + MeetralTile; indexed $($targetSizes.Count) targetsize x 2 altforms"

# --- 2. Signing certificate -------------------------------------------------------
# GitHub Actions runs the workflow step under pwsh 7, then Node spawns Windows
# PowerShell 5.1 with a polluted PSModulePath that prefers PowerShell 7 modules.
# That hides the Certificate provider ("Cannot find a provider with the name
# 'Certificate'") so Cert:\ never mounts. Reset to the machine module path first
# (same mitigation as actions/runner-images#9381 / PowerShell#18530).
if ($PSVersionTable.PSEdition -ne 'Core') {
  $machineModulePath = [Environment]::GetEnvironmentVariable('PSModulePath', 'Machine')
  if ($machineModulePath) {
    $env:PSModulePath = $machineModulePath
    Write-Host 'Reset PSModulePath to Machine (Windows PowerShell Certificate provider)'
  }
}
Import-Module Microsoft.PowerShell.Security -Force -ErrorAction SilentlyContinue
if (-not (Get-PSDrive -Name Cert -ErrorAction SilentlyContinue)) {
  # Provider root is "\", not "Cert:\" (drive name is supplied by -Name).
  $certDrive = New-PSDrive -Name Cert -PSProvider Certificate -Root '\' -Scope Global `
    -ErrorAction SilentlyContinue
  if (-not $certDrive -and -not (Get-PSDrive -Name Cert -ErrorAction SilentlyContinue)) {
    throw "Certificate provider unavailable - cannot open Cert:\CurrentUser\My to sign the sparse MSIX"
  }
  Write-Host 'Mounted Cert: PSDrive (Certificate provider)'
}

# Prefer file-based signtool signing when CI provides a PFX — avoids depending on
# store import succeeding. Still import into Cert:\ so /sha1 fallback works.
$signPfxFile = $null
$signPfxPasswordPlain = $null
if ($env:SPARSE_DEV_PFX_BASE64) {
  Write-Host 'Importing PFX from SPARSE_DEV_PFX_BASE64 (CI) ...'
  $signPfxFile = Join-Path $env:TEMP "$PackageName-sparse-sign.pfx"
  $signPfxPasswordPlain = [string]$env:SPARSE_DEV_PFX_PASSWORD
  [IO.File]::WriteAllBytes($signPfxFile, [Convert]::FromBase64String($env:SPARSE_DEV_PFX_BASE64))
  $pwd = ConvertTo-SecureString -String $signPfxPasswordPlain -AsPlainText -Force
  Import-PfxCertificate -FilePath $signPfxFile -CertStoreLocation Cert:\CurrentUser\My -Password $pwd | Out-Null
}

$cert = Get-ChildItem Cert:\CurrentUser\My |
  Where-Object { $_.Subject -eq $Publisher -and $_.HasPrivateKey } |
  Sort-Object NotAfter -Descending | Select-Object -First 1
if (-not $cert) {
  Write-Host "Creating self-signed dev cert $Publisher ..."
  $cert = New-SelfSignedCertificate -Type Custom -Subject $Publisher `
    -KeyUsage DigitalSignature -FriendlyName 'Meetral sparse identity dev' `
    -CertStoreLocation Cert:\CurrentUser\My `
    -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
}
Write-Host "Signing cert: $($cert.Thumbprint)"

# --- 3. Pack + sign ------------------------------------------------------------------
$msix = Join-Path $OutDir "$PackageName-sparse.msix"
if (Test-Path $msix) { Remove-Item $msix -Force }
& $makeappx pack /d $PkgRoot /p $msix /nv | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makeappx failed ($LASTEXITCODE)" }

if ($signPfxFile) {
  & $signtool sign /fd SHA256 /f $signPfxFile /p $signPfxPasswordPlain $msix | Out-Null
  Remove-Item $signPfxFile -Force -ErrorAction SilentlyContinue
} else {
  & $signtool sign /fd SHA256 /sha1 $cert.Thumbprint $msix | Out-Null
}
if ($LASTEXITCODE -ne 0) { throw "signtool failed ($LASTEXITCODE)" }
Write-Host "Packed + signed: $msix"

# --- 4. Stage installer payload (NSIS hook ships these into $INSTDIR) -----------------
# Sparse packages resolve content at ExternalLocation (NOT inside the .msix):
# logos + resources.pri must sit next to meetral.exe. Without resources.pri
# there, MRT cannot select altform-unplated and the taskbar falls back to a
# plated Square44 / MeetralAppList (AppModelSamples#15).
Get-ChildItem $OutDir -Filter 'Square*.png' -ErrorAction SilentlyContinue | Remove-Item -Force
Get-ChildItem $OutDir -Filter 'Meetral*.png' -ErrorAction SilentlyContinue | Remove-Item -Force
Copy-Item (Join-Path $PkgRoot 'Meetral*.png') $OutDir -Force
Copy-Item (Join-Path $PkgRoot 'resources.pri') (Join-Path $OutDir 'resources.pri') -Force
foreach ($f in 'Register-MeetralIdentity.ps1', 'Unregister-MeetralIdentity.ps1') {
  Copy-Item (Join-Path $scriptDir $f) (Join-Path $OutDir $f) -Force
}
Write-Host "Staged installer payload in $OutDir (logos + resources.pri)"

# --- 5. Render NSIS hooks + WiX fragment with absolute payload path -----------------
# NSIS resolves ${__FILEDIR__} to the main script workdir under target/, so the
# hook must carry baked absolute paths to find the payload at makensis time.
# WiX candle similarly needs absolute Source= paths for the sparse payload files.
$nsisTemplate = Join-Path $repoRoot 'src-tauri\installer-hooks.template.nsh'
$nsisRendered = Join-Path $repoRoot 'src-tauri\installer-hooks.nsh'
(Get-Content $nsisTemplate -Raw).Replace('@SPARSE_PAYLOAD_DIR@', $OutDir) |
  Set-Content $nsisRendered -Encoding ascii
Write-Host "Rendered $nsisRendered"

$wixTemplate = Join-Path $repoRoot 'src-tauri\windows\fragments\sparse-identity.wxs.template'
$wixRendered = Join-Path $repoRoot 'src-tauri\windows\fragments\sparse-identity.wxs'
$logoFiles = Get-ChildItem $OutDir -Filter 'Meetral*.png' | Sort-Object Name
# Bake absolute paths here — replacing @SPARSE_PAYLOAD_DIR@ before inserting
# logo lines would leave placeholder Source= values in the rendered wxs.
$logoFileXml = ($logoFiles | ForEach-Object {
  $id = ($_.BaseName -replace '[^A-Za-z0-9]', '_')
  if ($id.Length -gt 64) { $id = $id.Substring(0, 64) }
  "        <File Id=`"MeetralSparse_$id`" Source=`"$OutDir\$($_.Name)`" />"
}) -join "`r`n"
$wix = (Get-Content $wixTemplate -Raw).
  Replace('@SPARSE_LOGO_FILES@', $logoFileXml).
  Replace('@SPARSE_PAYLOAD_DIR@', $OutDir)
if ($wix -match '@SPARSE_') {
  throw 'Rendered sparse-identity.wxs still contains unresolved @SPARSE_* placeholders'
}
Set-Content $wixRendered -Value $wix -Encoding utf8
Write-Host "Rendered $wixRendered ($($logoFiles.Count) logo files)"
