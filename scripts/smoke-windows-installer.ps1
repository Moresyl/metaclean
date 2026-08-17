param(
  [Parameter(Mandatory = $true)]
  [string]$BundleDirectory
)

$ErrorActionPreference = "Stop"
$installRoot = Join-Path ([IO.Path]::GetTempPath()) "metaclean-release-smoke-$PID"
$process = $null

try {
  $packageMetadata = Get-Content -LiteralPath (Join-Path $PSScriptRoot "..\package.json") -Raw | ConvertFrom-Json
  $installers = @(Get-ChildItem -LiteralPath $BundleDirectory -Recurse -File -Filter "MetaClean_$($packageMetadata.version)_*-setup.exe")
  if ($installers.Count -ne 1) { throw "Expected one NSIS installer, found $($installers.Count)" }
  $installer = $installers[0].FullName
  $install = Start-Process -FilePath $installer -ArgumentList @("/S", "/D=$installRoot") -PassThru -Wait -WindowStyle Hidden
  if ($install.ExitCode -ne 0) { throw "NSIS installer exited with code $($install.ExitCode)" }

  $application = Join-Path $installRoot "MetaClean.exe"
  if (-not (Test-Path -LiteralPath $application -PathType Leaf)) { throw "Installed executable is missing: $application" }
  $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
  Start-Sleep -Seconds 6
  $process.Refresh()
  if ($process.HasExited) { throw "Installed MetaClean exited during the smoke window" }
  if ($process.MainWindowTitle -ne "MetaClean") { throw "Unexpected installed window title: $($process.MainWindowTitle)" }
}
finally {
  if ($null -ne $process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force }
  $uninstallers = @(Get-ChildItem -LiteralPath $installRoot -File -Filter "*uninstall*.exe" -ErrorAction SilentlyContinue)
  if ($uninstallers.Count -eq 1) {
    $uninstall = Start-Process -FilePath $uninstallers[0].FullName -ArgumentList "/S" -PassThru -Wait -WindowStyle Hidden
    if ($uninstall.ExitCode -ne 0) { throw "NSIS uninstaller exited with code $($uninstall.ExitCode)" }
  } elseif (Test-Path -LiteralPath $installRoot) {
    throw "Expected one installed uninstaller, found $($uninstallers.Count)"
  }
}

if (Test-Path -LiteralPath (Join-Path $installRoot "MetaClean.exe")) { throw "Installed executable remains after uninstall" }
Write-Output "Installed NSIS package launched for six seconds and uninstalled cleanly."
