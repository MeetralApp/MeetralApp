#Requires -Version 5.1
# Runs at uninstall time from $INSTDIR (shipped by the NSIS hook). Never fails.
try {
  Get-AppxPackage -Name Meetral -ErrorAction SilentlyContinue | Remove-AppxPackage -ErrorAction Stop
} catch { }
exit 0
