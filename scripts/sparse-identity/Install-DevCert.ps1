#Requires -Version 5.1
<#
.SYNOPSIS
  One-time per dev machine: ensures the MeetralDev self-signed cert exists and
  trusts it machine-wide (LocalMachine\TrustedPeople, needs one UAC approval).
  Without this trust, Add-AppxPackage rejects the sparse MSIX (0x800B0109).
#>
[CmdletBinding()]
param([string]$Publisher = 'CN=MeetralDev')
$ErrorActionPreference = 'Stop'

$cert = Get-ChildItem Cert:\CurrentUser\My |
  Where-Object { $_.Subject -eq $Publisher -and $_.HasPrivateKey } |
  Select-Object -First 1
if (-not $cert) {
  Write-Host "Creating self-signed cert $Publisher ..."
  $cert = New-SelfSignedCertificate -Type Custom -Subject $Publisher `
    -KeyUsage DigitalSignature -FriendlyName 'Meetral sparse identity dev' `
    -CertStoreLocation Cert:\CurrentUser\My `
    -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
}

$trusted = Get-ChildItem Cert:\LocalMachine\TrustedPeople -ErrorAction SilentlyContinue |
  Where-Object Thumbprint -EQ $cert.Thumbprint
if ($trusted) {
  Write-Host "Cert already trusted machine-wide ($($cert.Thumbprint))"
  exit 0
}

$cer = Join-Path $env:TEMP 'MeetralDev.cer'
Export-Certificate -Cert $cert -FilePath $cer -Force | Out-Null
Write-Host 'A UAC prompt will appear to trust the dev cert machine-wide ...'
Start-Process powershell -Verb RunAs -Wait -ArgumentList `
  '-NoProfile', '-Command', "Import-Certificate -FilePath '$cer' -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null"

$trusted = Get-ChildItem Cert:\LocalMachine\TrustedPeople -ErrorAction SilentlyContinue |
  Where-Object Thumbprint -EQ $cert.Thumbprint
if ($trusted) { Write-Host 'Trusted OK' } else { throw 'Trust failed - was the UAC prompt declined?' }
