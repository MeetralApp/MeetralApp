#Requires -Version 5.1
<#
.SYNOPSIS
  Runs at install time from $INSTDIR (shipped by the NSIS hook).
  Registers the sparse MSIX against this install location. Never fails the
  installer: if the signing cert is not trusted on this machine, registration
  is skipped and the app simply runs without package identity.
#>
[CmdletBinding()]
param([string]$InstallDir)

if (-not $InstallDir) { $InstallDir = Split-Path -Parent $MyInvocation.MyCommand.Path }
$log = Join-Path $InstallDir 'sparse-register.log'
function Log([string]$m) { "$(Get-Date -Format o) $m" | Out-File $log -Append -Encoding utf8 }

$msix = Join-Path $InstallDir 'Meetral-sparse.msix'
if (-not (Test-Path $msix)) { Log "msix missing: $msix"; exit 0 }

try {
  Add-AppxPackage -Path $msix -ExternalLocation $InstallDir -ErrorAction Stop
  Log 'registered'
} catch {
  Log "first attempt failed: $($_.Exception.Message)"
  try {
    Get-AppxPackage -Name Meetral -ErrorAction SilentlyContinue | Remove-AppxPackage -ErrorAction Stop
    Add-AppxPackage -Path $msix -ExternalLocation $InstallDir -ErrorAction Stop
    Log 'registered after reinstall'
  } catch {
    Log "registration skipped (dev cert not trusted on this machine?): $($_.Exception.Message)"
  }
}
exit 0
