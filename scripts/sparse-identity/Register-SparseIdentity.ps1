#Requires -Version 5.1
<#
.SYNOPSIS
  Dev convenience: builds the sparse MSIX (version synced from tauri.conf.json),
  ensures the dev cert exists + is trusted, and registers the package against a
  local build output (default: src-tauri/target/debug) so meetral.exe launched
  from there runs with package identity (Task Manager grouping, notifications).
#>
[CmdletBinding()]
param(
  [string]$ExeDir,
  [string]$Publisher = 'CN=MeetralDev',
  [string]$PackageName = 'Meetral'
)
$ErrorActionPreference = 'Stop'

$scriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
$repoRoot  = (Resolve-Path (Join-Path $scriptDir '..\..')).Path
if (-not $ExeDir) { $ExeDir = Join-Path $repoRoot 'src-tauri\target\debug' }
$resolvedExeDir = (Resolve-Path $ExeDir).Path
if (-not (Test-Path (Join-Path $resolvedExeDir 'meetral.exe'))) {
  throw "meetral.exe not found in $resolvedExeDir - run ``cargo build`` in src-tauri first."
}
$outDir = Join-Path $repoRoot 'src-tauri\installer\sparse'

# --- 1. Cert exists + trusted (one UAC prompt on first run) ----------------------
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptDir 'Install-DevCert.ps1') -Publisher $Publisher
if ($LASTEXITCODE -ne 0) { throw "Install-DevCert failed ($LASTEXITCODE)" }

# --- 2. Build + sign the sparse package -------------------------------------------
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptDir 'Build-SparsePackage.ps1') -Publisher $Publisher -PackageName $PackageName
if ($LASTEXITCODE -ne 0) { throw "Build-SparsePackage failed ($LASTEXITCODE)" }

# Sparse packages resolve content at ExternalLocation (not inside the .msix):
# Meetral* logos + resources.pri must sit next to meetral.exe or the taskbar
# cannot pick altform-unplated (falls back to a plated icon).
Get-ChildItem $resolvedExeDir -Filter 'Square*.png' -ErrorAction SilentlyContinue |
  Remove-Item -Force
Get-ChildItem $resolvedExeDir -Filter 'Meetral*.png' -ErrorAction SilentlyContinue |
  Remove-Item -Force
Copy-Item (Join-Path $outDir 'Meetral*.png') $resolvedExeDir -Force
Copy-Item (Join-Path $outDir 'resources.pri') (Join-Path $resolvedExeDir 'resources.pri') -Force

# --- 3. Register with external location --------------------------------------------
$msix = Join-Path $outDir "$PackageName-sparse.msix"
$existing = Get-AppxPackage -Name $PackageName -ErrorAction SilentlyContinue
if ($existing) {
  Write-Host "Removing previous registration $($existing.PackageFullName) ..."
  Remove-AppxPackage $existing.PackageFullName
}

Write-Host "Registering sparse package (ExternalLocation = $resolvedExeDir) ..."
Add-AppxPackage -Path $msix -ExternalLocation $resolvedExeDir

$pkg = Get-AppxPackage -Name $PackageName
Write-Host ''
Write-Host "Registered: $($pkg.PackageFullName)"
Write-Host 'Any meetral.exe launched from the ExternalLocation now runs with package identity.'
