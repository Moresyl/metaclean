param(
  [Parameter(Mandatory = $true)]
  [string]$BundleDirectory
)

$ErrorActionPreference = "Stop"
$packageMetadata = Get-Content -LiteralPath (Join-Path $PSScriptRoot "..\package.json") -Raw | ConvertFrom-Json
$msis = @(Get-ChildItem -LiteralPath $BundleDirectory -Recurse -File -Filter "MetaClean_$($packageMetadata.version)_*.msi")
if ($msis.Count -ne 1) { throw "Expected one MSI package, found $($msis.Count)" }

$uninstallRegistryPaths = @(
  "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
  "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*"
)
$existing = @(Get-ItemProperty $uninstallRegistryPaths -ErrorAction SilentlyContinue |
  Where-Object { $_.DisplayName -eq "MetaClean" })
if ($existing.Count -gt 0) {
  $versions = ($existing | ForEach-Object { $_.DisplayVersion } | Sort-Object -Unique) -join ", "
  throw "Refusing to replace an existing MetaClean installation during MSI smoke testing. Installed version(s): $versions"
}

$logRoot = Join-Path ([IO.Path]::GetTempPath()) "metaclean-msi-smoke-$PID"
$installLog = Join-Path $logRoot "install.log"
$uninstallLog = Join-Path $logRoot "uninstall.log"
$productCode = $null
$process = $null
New-Item -ItemType Directory -Path $logRoot -Force | Out-Null

try {
  $install = Start-Process -FilePath "msiexec.exe" -ArgumentList @(
    "/i", $msis[0].FullName, "/qn", "/norestart", "/L*v", $installLog
  ) -PassThru -Wait -WindowStyle Hidden
  if ($install.ExitCode -ne 0) { throw "MSI installation exited with code $($install.ExitCode)" }

  $entry = Get-ItemProperty $uninstallRegistryPaths -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -eq "MetaClean" -and $_.DisplayVersion -eq $packageMetadata.version } |
    Select-Object -First 1
  if (-not $entry) { throw "Installed MetaClean MSI registration was not found" }
  $productCode = $entry.PSChildName
  $application = Join-Path $entry.InstallLocation "MetaClean.exe"
  if (-not (Test-Path -LiteralPath $application -PathType Leaf)) { throw "Installed MSI executable is missing: $application" }
  $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
  Start-Sleep -Seconds 6
  $process.Refresh()
  if ($process.HasExited) { throw "Installed MSI MetaClean exited during the smoke window" }
  if ($process.MainWindowTitle -ne "MetaClean") { throw "Unexpected MSI window title: $($process.MainWindowTitle)" }
}
finally {
  if ($null -ne $process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force }
  if ($productCode) {
    $uninstall = Start-Process -FilePath "msiexec.exe" -ArgumentList @(
      "/x", $productCode, "/qn", "/norestart", "/L*v", $uninstallLog
    ) -PassThru -Wait -WindowStyle Hidden
    if ($uninstall.ExitCode -ne 0) { throw "MSI uninstall exited with code $($uninstall.ExitCode)" }
  }
}

$remaining = Get-ItemProperty $uninstallRegistryPaths -ErrorAction SilentlyContinue |
  Where-Object { $_.DisplayName -eq "MetaClean" -and $_.DisplayVersion -eq $packageMetadata.version }
if ($remaining) { throw "MSI registration remains after uninstall" }
if (Test-Path -LiteralPath $logRoot) { Remove-Item -LiteralPath $logRoot -Recurse -Force }
Write-Output "MSI package launched for six seconds and uninstalled cleanly."
