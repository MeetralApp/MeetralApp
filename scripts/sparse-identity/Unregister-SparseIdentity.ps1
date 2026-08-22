#Requires -Version 5.1
[CmdletBinding()]
param([string]$PackageName = 'Meetral')
$ErrorActionPreference = 'Stop'

$pkg = Get-AppxPackage -Name $PackageName -ErrorAction SilentlyContinue
if ($pkg) {
  Remove-AppxPackage $pkg.PackageFullName
  Write-Host "Removed $($pkg.PackageFullName)"
} else {
  Write-Host "Package '$PackageName' is not registered."
}
